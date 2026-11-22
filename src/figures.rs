//! Original, lit pixel figures: a moving traveler and decorative scene ambience.
//! Art/layout/provenance: art/figures/{contact-sheet.png,atlas-layout.json,RIGHTS.txt}.
use bevy::{
    asset::RenderAssetUsages,
    image::{ImageLoaderSettings, ImageSampler},
    pbr::{ExtendedMaterial, MaterialExtension, NotShadowCaster, NotShadowReceiver},
    prelude::*,
    render::{
        mesh::{Indices, MeshAabb},
        primitives::Aabb,
        render_resource::{AsBindGroup, PrimitiveTopology, ShaderRef},
        view::RenderLayers,
    },
    transform::TransformSystem,
};

use crate::{
    map::Area,
    scene::{CISTERN_FIGURES, JELLY_POSITION, SceneState, ground},
    sight::{SHADOW_LAYER, SightActor, SightPart},
};
#[derive(Asset, AsBindGroup, TypePath, Clone, Debug)]
struct FigureInk {
    #[texture(105)]
    #[sampler(106)]
    shelter: Handle<Image>,
}
impl MaterialExtension for FigureInk {
    fn fragment_shader() -> ShaderRef {
        "inks/figures.wgsl".into()
    }
}
type FigureMaterial = ExtendedMaterial<StandardMaterial, FigureInk>;

pub struct FiguresPlugin;

/// The camera tracks this entity's interpolated world-space foot, never its head/card.
#[derive(Component)]
pub struct PlayerVisual;

const BODY_PIXELS: f32 = 108.0;
const TRAVEL_HEIGHT: f32 = 1.32;
const WALK_TIME: f32 = 0.28;
const KEEPER_FOOT: Vec3 = Vec3::new(-5.0, 0.0, 3.0);
const HERBALIST_FOOT: Vec3 = Vec3::new(6.0, 0.0, 2.0);
const PLUM_CELL: IVec2 = IVec2::new(-3, 3);

// Explicit key-pose holds, in presentation seconds.
// End times match art/figures/trade-atlas-layout.json.
const KEEPER_CYCLE: &[(f32, usize)] = &[
    (1.10, 0),
    (1.36, 1),
    (1.60, 2),
    (1.84, 3),
    (2.08, 2),
    (2.32, 3),
    (2.56, 2),
    (2.80, 3),
    (3.16, 4),
    (4.61, 5),
    (4.93, 6),
    (6.28, 7),
    (6.60, 6),
];
const APOTHECARY_CYCLE: &[(f32, usize)] = &[
    (1.20, 0),
    (1.48, 1),
    (1.72, 2),
    (2.06, 3),
    (2.34, 1),
    (2.58, 2),
    (2.92, 3),
    (3.34, 0),
    (3.76, 4),
    (4.08, 5),
    (5.98, 6),
    (6.30, 5),
    (6.72, 7),
    (7.37, 0),
];
const WORKER_FOOT: Vec3 = Vec3::new(7.55, 0.0, -5.85);
const WORKER_CYCLE: &[(f32, usize)] = &[
    (2.60, 0),
    (3.20, 1),
    (3.85, 2),
    (5.00, 3),
    (6.50, 4),
    (7.00, 5),
    (8.25, 6),
    (11.25, 7),
    (11.85, 6),
];

fn trade_frame(seconds: f32, cycle: &[(f32, usize)]) -> usize {
    let phase = seconds % cycle[cycle.len() - 1].0;
    cycle.iter().find(|&&(end, _)| phase < end).unwrap().1
}

impl Plugin for FiguresPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(MaterialPlugin::<FigureMaterial>::default())
            .add_systems(Startup, spawn_figures)
            .add_systems(
                Update,
                (
                    change_shelter,
                    observe_scene,
                    animate_figures,
                    light_shadow_layer,
                )
                    .chain()
                    .in_set(crate::Frame::Draw),
            )
            .add_systems(
                PostUpdate,
                face_camera
                    .after(crate::lens::LensSet)
                    .before(TransformSystem::TransformPropagate),
            );
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum ActorKind {
    Traveler,
    Keeper,
    Herbalist,
    Worker,
    Jelly,
    Sentry,
    Moth,
}

#[derive(Component)]
struct Actor {
    kind: ActorKind,
    from: Vec3,
    to: Vec3,
    age: f32,
    moving: bool,
    facing: IVec2,
    area: Area,
    card: Entity,
    caster: Entity,
    shadow: Entity,
}

#[derive(Component)]
struct FigureCard;

#[derive(Component)]
struct GroundPlum;

struct Sheet {
    frames: Vec<Handle<Mesh>>,
    material: Handle<FigureMaterial>,
    bounds: Aabb,
}

#[derive(Resource)]
struct FigureArt {
    walk: Sheet,
    diagonal: Sheet,
    sentry: Sheet,
    moth: Sheet,
    keeper: Sheet,
    herbalist: Sheet,
    worker: Sheet,
    jelly: Sheet,
    plum: Sheet,
    contact: Handle<Mesh>,
    contact_material: Handle<StandardMaterial>,
    sole_contact: Handle<Mesh>,
    sole_material: Handle<StandardMaterial>,
    seated_material: Handle<StandardMaterial>,
    sun_caster_rotation: Quat,
}

/// Pixel (width/2,height-2) is the sole/base pivot. The empty two-pixel gutter
/// stays inside each cell, so nearest sampling cannot bleed adjacent frames.
fn card_mesh(
    frame: usize,
    columns: usize,
    rows: usize,
    pixels: Vec2,
    body_pixels: f32,
    height: f32,
) -> Mesh {
    let unit = height / body_pixels;
    let left = -pixels.x * 0.5 * unit;
    let right = -left;
    let bottom = -2.0 * unit;
    let top = (pixels.y - 2.0) * unit;
    let u0 = (frame % columns) as f32 / columns as f32;
    let u1 = ((frame % columns) + 1) as f32 / columns as f32;
    let v0 = (frame / columns) as f32 / rows as f32;
    let v1 = ((frame / columns) + 1) as f32 / rows as f32;
    Mesh::new(
        PrimitiveTopology::TriangleList,
        RenderAssetUsages::RENDER_WORLD,
    )
    .with_inserted_attribute(
        Mesh::ATTRIBUTE_POSITION,
        vec![
            [left, bottom, 0.0],
            [right, bottom, 0.0],
            [right, top, 0.0],
            [left, top, 0.0],
        ],
    )
    .with_inserted_attribute(Mesh::ATTRIBUTE_NORMAL, vec![[0.0, 0.0, 1.0]; 4])
    .with_inserted_attribute(
        Mesh::ATTRIBUTE_UV_0,
        vec![[u0, v1], [u1, v1], [u1, v0], [u0, v0]],
    )
    .with_inserted_indices(Indices::U16(vec![0, 1, 2, 0, 2, 3]))
}

fn sheet(
    path: &'static str,
    grid: UVec2,
    pixels: Vec2,
    body_pixels: f32,
    height: f32,
    server: &AssetServer,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<FigureMaterial>,
    shelter: &Handle<Image>,
) -> Sheet {
    let texture = server.load_with_settings(path, |settings: &mut ImageLoaderSettings| {
        settings.sampler = ImageSampler::nearest();
    });
    let material = materials.add(ExtendedMaterial {
        base: StandardMaterial {
            base_color: Color::srgb(0.98, 0.975, 1.),
            base_color_texture: Some(texture.clone()),
            // A small diffuse floor preserves local colours in a deep alcove.
            // Phosphor wings themselves emit light; cloth still takes real shadows.
            emissive_texture: Some(texture),
            emissive: if path == "figures/phosphor-moth.png" {
                LinearRgba::rgb(0.34, 0.38, 0.48)
            } else {
                LinearRgba::rgb(0.086, 0.092, 0.115)
            },
            alpha_mode: AlphaMode::Mask(0.5),
            cull_mode: None,
            double_sided: true,
            perceptual_roughness: 1.0,
            reflectance: 0.0,
            ..default()
        },
        extension: FigureInk {
            shelter: shelter.clone(),
        },
    });
    let mut bounds = Aabb::default();
    let frames = (0..(grid.x * grid.y) as usize)
        .map(|i| {
            let mesh = card_mesh(
                i,
                grid.x as usize,
                grid.y as usize,
                pixels,
                body_pixels,
                height,
            );
            if i == 0 {
                bounds = mesh
                    .compute_aabb()
                    .expect("figure quad has position bounds");
            }
            meshes.add(mesh)
        })
        .collect();
    // RENDER_WORLD meshes leave CPU Assets after upload. Late-spawned cards
    // therefore need these finite bounds explicitly, including their casters.
    Sheet {
        frames,
        material,
        bounds,
    }
}

fn change_shelter(
    s: Res<SceneState>,
    maps: Res<crate::stage::SkyShelter>,
    mut surfaces: ResMut<Assets<FigureMaterial>>,
    mut old: Local<Option<Area>>,
) {
    if *old != Some(s.area) {
        *old = Some(s.area);
        let field = maps.at(s.area);
        for (_, mat) in surfaces.iter_mut() {
            mat.extension.shelter = field.clone();
        }
    }
}

fn spawn_figures(
    mut commands: Commands,
    server: Res<AssetServer>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut skin: ResMut<Assets<FigureMaterial>>,
    state: Res<SceneState>,
    shelter: Res<crate::stage::SkyShelter>,
) {
    let human = Vec2::new(160.0, 128.0);
    let mut make = |path, grid, pixels, body, height| {
        sheet(
            path,
            grid,
            pixels,
            body,
            height,
            &server,
            &mut meshes,
            &mut skin,
            shelter.at(state.area),
        )
    };
    let sun = crate::daylight::SQUARE.to_sun();
    let art = FigureArt {
        sun_caster_rotation: Quat::from_rotation_y(sun.x.atan2(sun.z)),
        walk: make(
            "figures/traveler-walk.png",
            UVec2::splat(4),
            human,
            BODY_PIXELS,
            TRAVEL_HEIGHT,
        ),
        diagonal: make(
            "figures/traveler-diagonal.png",
            UVec2::splat(4),
            human,
            BODY_PIXELS,
            TRAVEL_HEIGHT,
        ),
        sentry: make(
            "figures/copper-sentry.png",
            UVec2::splat(4),
            Vec2::splat(128.),
            108.,
            1.34,
        ),
        moth: make(
            "figures/phosphor-moth.png",
            UVec2::splat(4),
            Vec2::splat(128.),
            94.,
            0.98,
        ),
        herbalist: make(
            "figures/apothecary-supplies.png",
            UVec2::new(4, 2),
            human,
            108.,
            1.31,
        ),
        keeper: make(
            "figures/keeper-work.png",
            UVec2::new(4, 2),
            human,
            108.,
            1.30,
        ),
        worker: make(
            "figures/seated-worker-cat.png",
            UVec2::new(4, 2),
            human,
            108.,
            1.08,
        ),
        jelly: make(
            "figures/jelly.png",
            UVec2::new(4, 2),
            Vec2::splat(96.),
            78.,
            0.70,
        ),
        plum: make("figures/plum.png", UVec2::ONE, Vec2::splat(32.), 25., 0.20),
        contact: meshes.add(Circle::new(0.5)),
        contact_material: materials.add(StandardMaterial {
            base_color: Color::WHITE,
            base_color_texture: Some(server.load_with_settings(
                "figures/contact-shadow.png",
                |settings: &mut ImageLoaderSettings| settings.sampler = ImageSampler::linear(),
            )),
            alpha_mode: AlphaMode::Blend,
            unlit: true,
            cull_mode: None,
            ..default()
        }),
        sole_contact: meshes.add(Rectangle::new(1.0, 1.0)),
        sole_material: materials.add(StandardMaterial {
            base_color: Color::WHITE,
            base_color_texture: Some(server.load_with_settings(
                "figures/sole-contact.png",
                |settings: &mut ImageLoaderSettings| settings.sampler = ImageSampler::linear(),
            )),
            alpha_mode: AlphaMode::Blend,
            unlit: true,
            cull_mode: None,
            ..default()
        }),
        seated_material: materials.add(StandardMaterial {
            base_color: Color::WHITE,
            base_color_texture: Some(server.load_with_settings(
                "figures/seated-contact.png",
                |settings: &mut ImageLoaderSettings| settings.sampler = ImageSampler::linear(),
            )),
            alpha_mode: AlphaMode::Blend,
            unlit: true,
            cull_mode: None,
            ..default()
        }),
    };
    spawn_actor(
        &mut commands,
        &art,
        ActorKind::Traveler,
        ground(state.player),
        state.facing,
        None,
        state.area,
    );
    for (kind, foot, facing, owner) in [
        (ActorKind::Keeper, KEEPER_FOOT, IVec2::X, 768),
        (ActorKind::Herbalist, HERBALIST_FOOT, IVec2::NEG_X, 769),
        (ActorKind::Worker, WORKER_FOOT, IVec2::NEG_Y, 770),
        (ActorKind::Jelly, ground(JELLY_POSITION), IVec2::X, 771),
    ] {
        spawn_actor(
            &mut commands,
            &art,
            kind,
            foot,
            facing,
            Some(owner),
            state.area,
        );
    }
    for (i, cell) in CISTERN_FIGURES.into_iter().enumerate() {
        let kind = if i == 0 || i == 2 {
            ActorKind::Sentry
        } else {
            ActorKind::Moth
        };
        spawn_actor(
            &mut commands,
            &art,
            kind,
            ground(cell),
            IVec2::NEG_X,
            Some(772 + i as u16),
            state.area,
        );
    }
    spawn_plum(&mut commands, &art, state.area);
    commands.insert_resource(art);
}

// Alpha extents include held props and the seated worker's cat. Contacts share
// ownership; the separate shadow casters stay outside camera occlusion.
fn actor_sight(kind: ActorKind, owner: Option<u16>) -> Option<SightActor> {
    let (area, half_width, height) = match kind {
        ActorKind::Traveler => return None,
        ActorKind::Keeper => (Area::Town, 0.374, 1.30),
        ActorKind::Herbalist => (Area::Town, 0.425, 1.31),
        ActorKind::Worker => (Area::Town, 0.691, 1.08),
        ActorKind::Jelly => (Area::Town, 0.395, 0.593),
        ActorKind::Sentry => (Area::Cistern, 0.745, 1.34),
        ActorKind::Moth => (Area::Cistern, 0.626, 1.105),
    };
    Some(SightActor {
        id: owner.expect("decorative figure ownership"),
        area,
        half_width,
        height,
        lift: 0.0,
    })
}

impl SightActor {
    /// Decorative cards pick their authored world feet, including the seated worker.
    pub(crate) fn pick_cell(&self, state: &SceneState, foot: Vec3) -> Option<IVec2> {
        if self.area != state.area {
            return None;
        }
        match self.id {
            768..=776 | 800..=831 => Some(IVec2::new(foot.x.round() as i32, foot.z.round() as i32)),
            _ => None,
        }
    }
}

fn spawn_actor(
    commands: &mut Commands,
    art: &FigureArt,
    kind: ActorKind,
    foot: Vec3,
    facing: IVec2,
    owner: Option<u16>,
    active_area: Area,
) {
    let (body, index) = if kind == ActorKind::Traveler {
        walking(art, facing, 1)
    } else {
        let body = match kind {
            ActorKind::Keeper => &art.keeper,
            ActorKind::Herbalist => &art.herbalist,
            ActorKind::Worker => &art.worker,
            ActorKind::Jelly => &art.jelly,
            ActorKind::Sentry => &art.sentry,
            ActorKind::Moth => &art.moth,
            ActorKind::Traveler => unreachable!(),
        };
        (body, 0)
    };
    let sight = actor_sight(kind, owner);
    let area = sight.map_or(active_area, |sight| sight.area);
    let visible = if kind == ActorKind::Traveler || area == active_area {
        Visibility::Inherited
    } else {
        Visibility::Hidden
    };
    let root = commands
        .spawn((Transform::from_translation(foot), visible))
        .id();
    if kind == ActorKind::Traveler {
        commands.entity(root).insert(PlayerVisual);
    }
    if let Some(sight) = sight {
        commands.entity(root).insert(sight);
    }
    let card = commands
        .spawn((
            FigureCard,
            Mesh3d(body.frames[index].clone()),
            MeshMaterial3d(body.material.clone()),
            body.bounds,
            Transform::default(),
            NotShadowCaster,
        ))
        .id();
    if let Some(sight) = sight {
        commands.entity(card).insert(SightPart(sight.id));
    }
    // Upright town casters face the sun azimuth, retaining full silhouette breadth.
    // Underground lamp proxies retain their original +Z-facing convention.
    let caster = commands
        .spawn((
            Mesh3d(body.frames[index].clone()),
            MeshMaterial3d(body.material.clone()),
            body.bounds,
            Transform::from_rotation(if area == Area::Town {
                art.sun_caster_rotation
            } else {
                Quat::IDENTITY
            }),
            RenderLayers::layer(SHADOW_LAYER),
        ))
        .id();
    commands.entity(root).add_children(&[card, caster]);
    let shoes = matches!(
        kind,
        ActorKind::Traveler | ActorKind::Keeper | ActorKind::Herbalist
    );
    let seated = kind == ActorKind::Worker;
    let contact_size = actor_contact_size(kind);
    let shadow = commands
        .spawn((
            Mesh3d(if shoes || seated {
                art.sole_contact.clone()
            } else {
                art.contact.clone()
            }),
            MeshMaterial3d(if seated {
                art.seated_material.clone()
            } else if shoes {
                art.sole_material.clone()
            } else {
                art.contact_material.clone()
            }),
            Transform::from_xyz(foot.x, 0.012, foot.z)
                .with_rotation(Quat::from_rotation_x(-std::f32::consts::FRAC_PI_2))
                .with_scale(contact_size.extend(1.0)),
            visible,
            NotShadowCaster,
            NotShadowReceiver,
        ))
        .id();
    if let Some(sight) = sight {
        commands.entity(shadow).insert(SightPart(sight.id));
    }
    commands.entity(root).insert(Actor {
        kind,
        from: foot,
        to: foot,
        age: WALK_TIME,
        moving: false,
        facing,
        area,
        card,
        caster,
        shadow,
    });
}

fn spawn_plum(commands: &mut Commands, art: &FigureArt, area: Area) {
    commands.spawn((
        GroundPlum,
        FigureCard,
        SightActor {
            id: 800,
            area: Area::Town,
            half_width: 0.096,
            height: 0.128,
            lift: 0.0,
        },
        SightPart(800),
        Mesh3d(art.plum.frames[0].clone()),
        MeshMaterial3d(art.plum.material.clone()),
        art.plum.bounds,
        Transform::from_translation(ground(PLUM_CELL) + Vec3::Y * 0.02),
        if area == Area::Town {
            Visibility::Inherited
        } else {
            Visibility::Hidden
        },
    ));
}

fn observe_scene(
    state: Res<SceneState>,
    mut previous: Local<Option<(Area, u64)>>,
    mut actors: Query<(&mut Actor, &mut Transform, &mut Visibility)>,
    mut contacts: Query<&mut Visibility, (Without<Actor>, Without<GroundPlum>)>,
    mut plums: Query<&mut Visibility, (With<GroundPlum>, Without<Actor>)>,
) {
    let current = (state.area, state.movement);
    if *previous == Some(current) {
        return;
    }
    let reset = previous.is_none_or(|(area, _)| area != state.area);
    for (mut actor, mut transform, mut visibility) in &mut actors {
        if actor.kind == ActorKind::Traveler {
            let target = ground(state.player);
            actor.from = if reset { target } else { transform.translation };
            actor.to = target;
            actor.moving = !reset && actor.from.distance_squared(target) > 0.001;
            actor.age = if actor.moving { 0.0 } else { WALK_TIME };
            actor.facing = state.facing;
            actor.area = state.area;
            if reset {
                transform.translation = target;
            }
        }
        if reset {
            *visibility = if actor.area == state.area {
                Visibility::Inherited
            } else {
                Visibility::Hidden
            };
            if let Ok(mut contact) = contacts.get_mut(actor.shadow) {
                *contact = *visibility;
            }
        }
    }
    if reset {
        for mut visibility in &mut plums {
            *visibility = if state.area == Area::Town {
                Visibility::Inherited
            } else {
                Visibility::Hidden
            };
        }
    }
    *previous = Some(current);
}

fn direction_row(direction: IVec2) -> usize {
    if direction.x < 0 {
        1
    } else if direction.x > 0 {
        3
    } else if direction.y < 0 {
        2
    } else {
        0
    }
}

fn walking(art: &FigureArt, dir: IVec2, step: usize) -> (&Sheet, usize) {
    let row = match (dir.x.signum(), dir.y.signum()) {
        (-1, 1) => 0,
        (-1, -1) => 1,
        (1, -1) => 2,
        (1, 1) => 3,
        _ => return (&art.walk, direction_row(dir) * 4 + step),
    };
    (&art.diagonal, row * 4 + step)
}

fn actor_contact_size(kind: ActorKind) -> Vec2 {
    match kind {
        ActorKind::Worker => Vec2::new(1.30, 0.34),
        ActorKind::Jelly => Vec2::new(0.55, 0.30),
        _ => Vec2::new(0.53, 0.30),
    }
}

fn animate_figures(
    time: Res<Time>,
    art: Res<FigureArt>,
    state: Res<SceneState>,
    mut actors: Query<(&mut Actor, &mut Transform)>,
    mut cards: Query<
        (
            &mut Mesh3d,
            &mut MeshMaterial3d<FigureMaterial>,
            &mut Transform,
        ),
        Without<Actor>,
    >,
    mut shadows: Query<&mut Transform, (Without<Actor>, Without<MeshMaterial3d<FigureMaterial>>)>,
) {
    for (mut actor, mut root) in &mut actors {
        if actor.area != state.area {
            continue;
        }
        let (sheet, frame) = match actor.kind {
            ActorKind::Traveler => {
                actor.age = (actor.age + time.delta_secs()).min(WALK_TIME);
                let t = actor.age / WALK_TIME;
                let smooth = t * t * (3.0 - 2.0 * t);
                root.translation = actor.from.lerp(actor.to, smooth);
                let step = if actor.moving && t < 1.0 {
                    ((t * 4.0) as usize).min(3)
                } else {
                    actor.moving = false;
                    1
                };
                walking(&art, actor.facing, step)
            }
            ActorKind::Herbalist => (
                &art.herbalist,
                trade_frame(time.elapsed_secs(), APOTHECARY_CYCLE),
            ),
            ActorKind::Keeper => (&art.keeper, trade_frame(time.elapsed_secs(), KEEPER_CYCLE)),
            ActorKind::Worker => (&art.worker, trade_frame(time.elapsed_secs(), WORKER_CYCLE)),
            ActorKind::Jelly => (&art.jelly, 0),
            ActorKind::Sentry => (&art.sentry, ((time.elapsed_secs() / 0.18) as usize) % 4),
            ActorKind::Moth => (&art.moth, ((time.elapsed_secs() / 0.18) as usize) % 4),
        };
        for entity in [actor.card, actor.caster] {
            if let Ok((mut mesh, mut material, mut transform)) = cards.get_mut(entity) {
                if mesh.0 != sheet.frames[frame] {
                    mesh.0 = sheet.frames[frame].clone();
                }
                if material.0 != sheet.material {
                    material.0 = sheet.material.clone();
                }
                if entity == actor.caster {
                    transform.rotation = if actor.area == Area::Town {
                        art.sun_caster_rotation
                    } else {
                        Quat::IDENTITY
                    };
                }
                transform.translation.y = if actor.kind == ActorKind::Moth {
                    0.26
                } else {
                    0.0
                };
            }
        }
        if actor.kind == ActorKind::Traveler {
            if let Ok(mut shadow) = shadows.get_mut(actor.shadow) {
                shadow.translation = Vec3::new(root.translation.x, 0.012, root.translation.z);
            }
        }
    }
}

fn face_camera(
    // Camera rotation is copied below; this system never moves a foot pivot.
    camera: Query<&Transform, (With<Camera3d>, Without<FigureCard>)>,
    mut cards: Query<&mut Transform, With<FigureCard>>,
) {
    let Ok(camera) = camera.single() else {
        return;
    };
    // One screen orientation for the complete image plane. Looking at the camera
    // position would toe in outer figures and deform them during the eye-level dolly.
    for mut card in &mut cards {
        card.rotation = camera.rotation;
    }
}

fn light_shadow_layer(
    mut commands: Commands,
    lights: Query<
        (Entity, Option<&RenderLayers>),
        Or<(Added<DirectionalLight>, Added<PointLight>, Added<SpotLight>)>,
    >,
) {
    for (entity, layers) in &lights {
        commands
            .entity(entity)
            .insert(layers.cloned().unwrap_or_default().with(SHADOW_LAYER));
    }
}
