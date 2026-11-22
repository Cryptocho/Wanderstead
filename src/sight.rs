//! Hide every model between the optical centre and the traveler's visible body.
//! Static GLBs keep their material batches: the second UV set names the owner
//! of each triangle. The camera color pass and mouse share the same owner mask.
mod geometry;

use crate::{
    figures::PlayerVisual,
    game::Outing,
    map::{self, Area},
};
use bevy::{
    prelude::*,
    render::{
        Render, RenderApp, RenderSet,
        extract_resource::{ExtractResource, ExtractResourcePlugin},
        primitives::Aabb,
        render_resource::{Buffer, BufferInitDescriptor, BufferUsages},
        renderer::{RenderDevice, RenderQueue},
        view::{RenderLayers, VisibilitySystems},
    },
    transform::TransformSystem,
};
use geometry::{Bounds, Corridor, RELEASE_MARGIN, View};
use serde::Deserialize;
use std::sync::LazyLock;

pub const SHADOW_LAYER: usize = 1;
const WORDS: usize = 32; // 1024 global IDs; 0 = terrain, never hidden
type Bits = [u32; WORDS];

fn marked(mask: &Bits, id: u16) -> bool {
    mask[id as usize / 32] & (1 << (id % 32)) != 0
}
fn mark(mask: &mut Bits, id: u16) {
    mask[id as usize / 32] |= 1 << (id % 32);
}

#[derive(Deserialize)]
struct Record {
    id: u16,
    name: Box<str>,
    cell: Option<[i32; 2]>,
    boxes: Vec<[[f32; 3]; 2]>,
}
struct Model {
    id: u16,
    name: Box<str>,
    cell: Option<IVec2>,
    outer: Bounds,
    parts: Box<[Bounds]>,
}
static SITES: LazyLock<[Vec<Model>; 2]> = LazyLock::new(|| {
    let mut used = [0; WORDS];
    [
        [
            include_str!("../assets/lanes/sight.json"),
            include_str!("../assets/market/town-sight.json"),
        ],
        [
            include_str!("../assets/cistern/sight.json"),
            include_str!("../assets/market/cistern-sight.json"),
        ],
    ]
    .map(|files| {
        files
            .into_iter()
            .flat_map(|src| {
                serde_json::from_str::<Vec<Record>>(src).expect("authored sight model manifest")
            })
            .map(|r| {
                assert!(
                    r.id != 0 && r.id < 1024 && !marked(&used, r.id),
                    "invalid or duplicate model {}",
                    r.name
                );
                mark(&mut used, r.id);
                let parts: Box<[_]> = r
                    .boxes
                    .into_iter()
                    .map(|[a, b]| Bounds {
                        min: a.into(),
                        max: b.into(),
                    })
                    .collect();
                let outer = parts
                    .iter()
                    .copied()
                    .reduce(Bounds::union)
                    .expect("model without sight bounds");
                Model {
                    id: r.id,
                    name: r.name,
                    cell: r.cell.map(IVec2::from),
                    outer,
                    parts,
                }
            })
            .collect()
    })
});
fn models(area: Area) -> &'static [Model] {
    &SITES[match area {
        Area::Town => 0,
        Area::Cistern => 1,
    }]
}

/// Used only at spawn/bind time to attach separate draw calls to a batched model.
pub(crate) fn part(area: Area, name: &str) -> SightPart {
    SightPart(
        models(area)
            .iter()
            .find(|m| m.name.as_ref() == name)
            .unwrap_or_else(|| panic!("missing {area:?} sight model {name}"))
            .id,
    )
}

#[derive(Component, Clone, Copy)]
#[require(RenderLayers)]
pub(crate) struct SightPart(pub u16);

#[derive(Component, Clone, Copy)]
pub(crate) struct SightActor {
    pub id: u16,
    pub area: Area,
    pub half_width: f32,
    pub height: f32,
    pub lift: f32,
}

/// Operable solid whose extent moves independently of the baked masonry.
#[derive(Component)]
pub(crate) struct MovingSolid(pub u16);

#[derive(Resource, Clone, ExtractResource)]
pub(crate) struct SightMask {
    pub buffer: Buffer,
    hidden: Bits,
}
#[derive(Clone, Copy)]
struct Body {
    id: u16,
    base: Vec3,
    half: f32,
    height: f32,
    cell: Option<IVec2>,
}
#[derive(Resource)]
pub(crate) struct WorldSight {
    pub enabled: bool,
    last: Option<(View, Area, bool)>,
    statics: Bits,
    hidden: Bits,
    bodies: Vec<Body>,
    moving: Vec<(u16, Bounds)>,
    cones: Option<(Corridor, Corridor)>,
}
impl Default for WorldSight {
    fn default() -> Self {
        Self {
            enabled: true,
            last: None,
            statics: [0; WORDS],
            hidden: [0; WORDS],
            bodies: Vec::with_capacity(12),
            moving: Vec::with_capacity(12),
            cones: None,
        }
    }
}
pub struct SightPlugin;
impl Plugin for SightPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<WorldSight>()
            .add_plugins(ExtractResourcePlugin::<SightMask>::default())
            .add_systems(PreStartup, install)
            .add_systems(
                PostUpdate,
                (refresh, apply_parts)
                    .chain()
                    .after(TransformSystem::TransformPropagate)
                    .after(VisibilitySystems::VisibilityPropagate)
                    .after(VisibilitySystems::CalculateBounds)
                    .before(VisibilitySystems::CheckVisibility),
            );
        if let Some(render) = app.get_sub_app_mut(RenderApp) {
            render.add_systems(Render, upload.in_set(RenderSet::PrepareResources));
        }
    }
}
fn install(mut c: Commands, device: Res<RenderDevice>) {
    let buffer = device.create_buffer_with_data(&BufferInitDescriptor {
        label: Some("Foreground model IDs"),
        contents: &[0; WORDS * 4],
        usage: BufferUsages::STORAGE | BufferUsages::COPY_DST,
    });
    c.insert_resource(SightMask {
        buffer,
        hidden: [0; WORDS],
    });
}
fn upload(s: Option<Res<SightMask>>, q: Res<RenderQueue>) {
    if let Some(s) = s
        && s.is_changed()
    {
        q.write_buffer(&s.buffer, 0, s.hidden.map(u32::to_ne_bytes).as_flattened());
    }
}

fn static_hits(objects: &[Model], enter: &Corridor, keep: &Corridor, previous: &Bits) -> Bits {
    let mut hits = [0; WORDS];
    for obj in objects {
        let corridor = if marked(previous, obj.id) {
            keep
        } else {
            enter
        };
        if corridor.hits(obj.outer)
            && (obj.parts.len() == 1 || obj.parts.iter().any(|&p| corridor.hits(p)))
        {
            mark(&mut hits, obj.id);
        }
    }
    hits
}

fn refresh(
    eye: Single<&GlobalTransform, With<Camera3d>>,
    player: Single<&GlobalTransform, With<PlayerVisual>>,
    game: Res<Outing>,
    actors: Query<(&SightActor, &GlobalTransform, &InheritedVisibility)>,
    solids: Query<(&MovingSolid, &Aabb, &GlobalTransform, &InheritedVisibility)>,
    mut sight: ResMut<WorldSight>,
    mut gpu: ResMut<SightMask>,
) {
    let v = View {
        eye: eye.translation(),
        foot: player.translation(),
        right: *eye.right(),
        up: *eye.up(),
        forward: *eye.forward(),
    };
    let s = &mut *sight;
    let key = (v, game.area, s.enabled);
    if s.last != Some(key) {
        let active = s.enabled && (v.foot - v.eye).dot(v.forward) > 0.;
        s.cones = active.then(|| (Corridor::new(v, 0.), Corridor::new(v, RELEASE_MARGIN)));
        s.statics = match &s.cones {
            Some((enter, keep)) => static_hits(models(game.area), enter, keep, &s.hidden),
            None => [0; WORDS],
        };
        s.last = Some(key);
    }
    let mut hidden = s.statics;
    s.bodies.clear();
    for (a, world, visible) in &actors {
        if !visible.get() || a.area != game.area {
            continue;
        }
        let foot = world.translation();
        let base = foot + Vec3::Y * a.lift;
        let margin = if marked(&s.hidden, a.id) {
            RELEASE_MARGIN
        } else {
            0.
        };
        if s.enabled && v.billboard_in_front(base, a.half_width, a.height, margin) {
            mark(&mut hidden, a.id);
        }
        s.bodies.push(Body {
            id: a.id,
            base,
            half: a.half_width,
            height: a.height,
            cell: a.pick_cell(&game, foot),
        });
    }
    s.moving.clear();
    for (owner, aabb, world, visible) in &solids {
        if !visible.get() {
            continue;
        }
        let tr = world.affine();
        let centre = tr.transform_point3a(aabb.center);
        let half = tr.matrix3.abs() * aabb.half_extents;
        let b = Bounds {
            min: (centre - half).into(),
            max: (centre + half).into(),
        };
        s.moving.push((owner.0, b));
        if let Some((enter, keep)) = &s.cones {
            let cone = if marked(&s.hidden, owner.0) {
                keep
            } else {
                enter
            };
            if cone.hits(b) {
                mark(&mut hidden, owner.0);
            }
        }
    }
    s.hidden = hidden;
    if gpu.hidden != hidden {
        gpu.hidden = hidden;
    }
}
fn apply_parts(sight: Res<WorldSight>, mut pieces: Query<(&SightPart, &mut RenderLayers)>) {
    for (part, mut layers) in &mut pieces {
        let target = if marked(&sight.hidden, part.0) {
            RenderLayers::layer(SHADOW_LAYER)
        } else {
            RenderLayers::layer(0)
        };
        layers.set_if_neq(target);
    }
}

impl WorldSight {
    pub fn description(&self, area: Area) -> serde_json::Value {
        let names: Vec<_> = models(area)
            .iter()
            .filter(|m| marked(&self.hidden, m.id))
            .map(|m| m.name.as_ref())
            .collect();
        let actors: Vec<_> = self
            .bodies
            .iter()
            .filter(|b| marked(&self.hidden, b.id))
            .map(|b| b.id)
            .collect();
        serde_json::json!({"enabled":self.enabled,"models":names,"actors":actors})
    }

    /// Pick the geometry actually in this camera, including through any part
    /// of a hidden building. Its logical footprint remains blocked to movement.
    pub fn pick(&self, area: Area, ray: Ray3d) -> Option<(IVec2, f32)> {
        let current = self.last.is_some_and(|(_, a, _)| a == area);
        let visible = |id| !current || !marked(&self.hidden, id);
        let cell_at = |p: Vec3| {
            let c = IVec2::new(p.x.round() as i32, p.z.round() as i32);
            map::inside(c).then_some(c)
        };
        let mut closest = f32::INFINITY;
        let mut hit = None;
        if ray.direction.y < -0.0001 {
            let t = -ray.origin.y / ray.direction.y;
            if t >= 0. {
                closest = t;
                hit = cell_at(ray.get_point(t)).map(|c| (c, 0.));
            }
        }
        let objects = models(area);
        for obj in objects {
            if !visible(obj.id) {
                continue;
            }
            let Some(broad) = obj.outer.ray(ray, closest) else {
                continue;
            };
            let t = if obj.parts.len() == 1 {
                Some(broad)
            } else {
                obj.parts
                    .iter()
                    .filter_map(|b| b.ray(ray, closest))
                    .min_by(f32::total_cmp)
            };
            if let Some(t) = t {
                closest = t;
                let point = ray.get_point(t);
                hit = obj.cell.or_else(|| cell_at(point)).map(|c| (c, point.y));
            }
        }
        if !current {
            return hit;
        }
        for &(id, bounds) in &self.moving {
            if visible(id)
                && let Some(t) = bounds.ray(ray, closest)
            {
                closest = t;
                let point = ray.get_point(t);
                hit = objects
                    .iter()
                    .find(|m| m.id == id)
                    .and_then(|m| m.cell)
                    .or_else(|| cell_at(point))
                    .map(|c| (c, point.y));
            }
        }
        let view = self.last?.0;
        let denom = ray.direction.dot(view.forward);
        if denom.abs() > 1e-5 {
            for b in &self.bodies {
                if !visible(b.id) {
                    continue;
                }
                let t = (b.base - ray.origin).dot(view.forward) / denom;
                if t < 0. || t >= closest {
                    continue;
                }
                let point = ray.get_point(t);
                let p = point - b.base;
                if p.dot(view.right).abs() <= b.half && (0.0..=b.height).contains(&p.dot(view.up)) {
                    closest = t;
                    hit = b.cell.map(|c| (c, point.y));
                }
            }
        }
        hit
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn level(eye: Vec3, foot: Vec3) -> View {
        View {
            eye,
            foot,
            right: Vec3::X,
            up: Vec3::Y,
            forward: Vec3::NEG_Z,
        }
    }

    #[test]
    fn foreground_house_and_separate_stall_both_clear_including_clicks_outside_the_sprite() {
        // The camera is inside the southern slate house. A second stall is
        // on the same segment. The inn is on the far side of the traveler.
        let v = level(Vec3::new(-6., 1.34, 10.), Vec3::new(-6., 0., 3.));
        let c = Corridor::new(v, 0.);
        let hits = static_hits(models(Area::Town), &c, &c, &[0; WORDS]);
        let house = part(Area::Town, "Backdrop_11").0;
        let stall = part(Area::Town, "stall_-6.15_6.23").0;
        assert!(marked(&hits, house) && marked(&hits, stall));
        assert!(!marked(&hits, part(Area::Town, "CopperKettle").0));
        let ray = Ray3d::new(v.eye, Dir3::new(Vec3::new(-7., 0., 3.) - v.eye).unwrap());
        let mut s = WorldSight {
            last: Some((v, Area::Town, true)),
            ..default()
        };
        let exposed = Some(IVec2::new(-7, 3));
        assert_ne!(s.pick(Area::Town, ray).map(|(p, _)| p), exposed);
        s.hidden = hits;
        assert_eq!(s.pick(Area::Town, ray).map(|(p, _)| p), exposed);
        let close = Corridor::new(level(Vec3::new(-6., 1.34, 5.), v.foot), RELEASE_MARGIN);
        let restored = static_hits(models(Area::Town), &close, &close, &hits);
        assert!(!marked(&restored, house) && !marked(&restored, stall));
    }

    #[test]
    fn open_arch_and_objects_beyond_the_character_do_not_clear() {
        let v = level(Vec3::new(0., 1.3, 5.), Vec3::ZERO);
        let c = Corridor::new(v, 0.);
        let bounds = |min: [f32; 3], max: [f32; 3]| Bounds {
            min: min.into(),
            max: max.into(),
        };
        let parts = Box::new([
            bounds([-2., 0., 2.], [-1., 3., 2.3]),
            bounds([1., 0., 2.], [2., 3., 2.3]),
            bounds([-2., 2.3, 2.], [2., 3., 2.3]),
        ]);
        let arch = Model {
            id: 1,
            name: "open arch".into(),
            cell: None,
            outer: parts[0].union(parts[1]),
            parts,
        };
        assert!(c.hits(arch.outer));
        assert!(!marked(&static_hits(&[arch], &c, &c, &[0; WORDS]), 1));
        assert!(!c.hits(bounds([-0.4, 0.1, -1.], [0.4, 1.5, -0.1])));
        assert!(!c.hits(bounds([-0.4, 0.1, 5.2], [0.4, 1.5, 6.])));
        // A thin edge misses the centre line, but intersects the on-screen shoulder.
        assert!(c.hits(bounds([0.19, 1., 2.], [0.27, 1.3, 2.2])));
        assert!(v.billboard_in_front(Vec3::new(0.3, 0.1, 2.), 0.25, 1.2, 0.));
        assert!(!v.billboard_in_front(Vec3::new(0., 0., -0.1), 0.40, 1.4, 0.));
    }
}
