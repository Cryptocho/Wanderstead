//! Authored floor seams, surfaced hover picking, and the actual walking route.
//! Two persistent meshes retain their CPU buffers across preview changes.
use bevy::{
    asset::RenderAssetUsages,
    pbr::{NotShadowCaster, NotShadowReceiver},
    prelude::*,
    render::{
        mesh::VertexAttributeValues, render_resource::PrimitiveTopology, view::NoFrustumCulling,
    },
    window::PrimaryWindow,
};

use crate::{
    Frame,
    controls::ControlsSet,
    map::{self, Area},
    scene::SceneState,
};

pub struct TileViewPlugin;

/// `cell` is the actual surfaced tile under the pointer. `path` contains only
/// passable eight-way steps, excluding the player, and is empty for solid props.
#[derive(Resource, Default)]
pub struct HoverTile {
    pub cell: Option<IVec2>,
    pub path: Vec<IVec2>,
    reachable: bool,
    area: Option<Area>,
    surface_y: f32,
    path_key: Option<(Area, IVec2, Option<IVec2>)>,
}

#[derive(Resource)]
struct TileMeshes {
    grid: Handle<Mesh>,
    cursor: Handle<Mesh>,
}

impl Plugin for TileViewPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<HoverTile>()
            .add_systems(Startup, install_tiles)
            .add_systems(
                Update,
                sample_hover
                    .in_set(Frame::Input)
                    .after(ControlsSet::Keys)
                    .before(ControlsSet::Pointer),
            )
            .add_systems(Update, paint_tiles.in_set(Frame::Draw));
    }
}

fn empty_mesh() -> Mesh {
    Mesh::new(
        PrimitiveTopology::TriangleList,
        RenderAssetUsages::default(),
    )
    .with_inserted_attribute(Mesh::ATTRIBUTE_POSITION, Vec::<[f32; 3]>::new())
    .with_inserted_attribute(Mesh::ATTRIBUTE_NORMAL, Vec::<[f32; 3]>::new())
    .with_inserted_attribute(Mesh::ATTRIBUTE_COLOR, Vec::<[f32; 4]>::new())
}

fn install_tiles(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    let material = materials.add(StandardMaterial {
        base_color: Color::WHITE,
        alpha_mode: AlphaMode::Blend,
        unlit: true,
        cull_mode: None,
        fog_enabled: false,
        depth_bias: 0.5,
        ..default()
    });
    let handles = TileMeshes {
        grid: meshes.add(empty_mesh()),
        cursor: meshes.add(empty_mesh()),
    };
    for (name, handle) in [
        ("Map tile seams", &handles.grid),
        ("Hovered tile and real route", &handles.cursor),
    ] {
        commands.spawn((
            Name::new(name),
            Mesh3d(handle.clone()),
            MeshMaterial3d(material.clone()),
            Transform::default(),
            NotShadowCaster,
            NotShadowReceiver,
            NoFrustumCulling,
        ));
    }
    commands.insert_resource(handles);
}

fn sample_hover(
    window: Single<&Window, With<PrimaryWindow>>,
    camera: Single<(&Camera, &GlobalTransform), With<Camera3d>>,
    scene: Res<SceneState>,
    sight: Res<crate::sight::WorldSight>,
    mut hover: ResMut<HoverTile>,
) {
    let hit = window.cursor_position().and_then(|position| {
        camera
            .0
            .viewport_to_world(camera.1, position)
            .ok()
            .and_then(|ray| sight.pick(scene.area, ray))
    });
    let cell = hit.map(|(cell, _)| cell);
    let surface_y = hit.map_or(0.0, |(_, y)| y);
    if cell != hover.cell
        || hover.area != Some(scene.area)
        || (hover.surface_y - surface_y).abs() > 0.04
    {
        hover.cell = cell;
        hover.area = Some(scene.area);
        hover.surface_y = surface_y;
    }
    if hover.path_stale(&scene) {
        hover.update_path(&scene);
    }
}

impl HoverTile {
    fn path_stale(&self, scene: &SceneState) -> bool {
        let cell = if self.area == Some(scene.area) {
            self.cell
        } else {
            None
        };
        self.path_key != Some((scene.area, scene.player, cell))
    }

    fn update_path(&mut self, scene: &SceneState) {
        if self.area != Some(scene.area) {
            self.cell = None;
            self.area = Some(scene.area);
        }
        self.path_key = Some((scene.area, scene.player, self.cell));
        self.reachable = if let Some(cell) = self.cell {
            scene.path_to(cell, &mut self.path)
        } else {
            self.path.clear();
            false
        };
    }
}

// Borrow the mesh's own CPU buffers so rebuilding a preview retains capacity.
struct Ink {
    positions: Vec<[f32; 3]>,
    colors: Vec<[f32; 4]>,
    normals: Vec<[f32; 3]>,
}
impl Ink {
    fn take(mesh: &mut Mesh) -> Self {
        let positions = match mesh.remove_attribute(Mesh::ATTRIBUTE_POSITION) {
            Some(VertexAttributeValues::Float32x3(mut v)) => {
                v.clear();
                v
            }
            _ => Vec::new(),
        };
        let colors = match mesh.remove_attribute(Mesh::ATTRIBUTE_COLOR) {
            Some(VertexAttributeValues::Float32x4(mut v)) => {
                v.clear();
                v
            }
            _ => Vec::new(),
        };
        let normals = match mesh.remove_attribute(Mesh::ATTRIBUTE_NORMAL) {
            Some(VertexAttributeValues::Float32x3(mut v)) => {
                v.clear();
                v
            }
            _ => Vec::new(),
        };
        Self {
            positions,
            colors,
            normals,
        }
    }
    fn finish(self, mesh: &mut Mesh) {
        mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, self.positions);
        mesh.insert_attribute(Mesh::ATTRIBUTE_NORMAL, self.normals);
        mesh.insert_attribute(Mesh::ATTRIBUTE_COLOR, self.colors);
    }
    fn quad(&mut self, min: Vec2, max: Vec2, y: f32, color: [f32; 4]) {
        for p in [
            min,
            Vec2::new(min.x, max.y),
            max,
            min,
            max,
            Vec2::new(max.x, min.y),
        ] {
            self.positions.push([p.x, y, p.y]);
            self.normals.push([0.0, 1.0, 0.0]);
            self.colors.push(color);
        }
    }
    fn line(&mut self, a: Vec2, b: Vec2, y: f32, width: f32, color: [f32; 4]) {
        let delta = b - a;
        let normal = Vec2::new(-delta.y, delta.x).normalize_or_zero() * width * 0.5;
        let points = [
            a - normal,
            a + normal,
            b + normal,
            a - normal,
            b + normal,
            b - normal,
        ];
        for p in points {
            self.positions.push([p.x, y, p.y]);
            self.normals.push([0.0, 1.0, 0.0]);
            self.colors.push(color);
        }
    }
    fn corners(&mut self, cell: IVec2, y: f32, color: [f32; 4], width: f32) {
        let c = cell.as_vec2();
        for sx in [-1.0, 1.0] {
            for sz in [-1.0, 1.0] {
                let corner = c + Vec2::new(sx, sz) * 0.46;
                self.line(corner, corner - Vec2::X * sx * 0.18, y, width, color);
                self.line(corner, corner - Vec2::Y * sz * 0.18, y, width, color);
            }
        }
    }
}

fn paint_tiles(
    scene: Res<SceneState>,
    mut hover: ResMut<HoverTile>,
    handles: Res<TileMeshes>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut last_area: Local<Option<Area>>,
) {
    // Navigation has committed this frame; refresh IPC and the preview from the
    // new position rather than the pre-action sample that dispatched the click.
    if hover.path_stale(&scene) {
        hover.update_path(&scene);
    }
    let changed_area = *last_area != Some(scene.area);
    if changed_area {
        *last_area = Some(scene.area);
        let mesh = meshes.get_mut(&handles.grid).unwrap();
        let mut ink = Ink::take(mesh);
        for z in map::MIN.y..=map::MAX.y {
            for x in map::MIN.x..=map::MAX.x {
                let p = IVec2::new(x, z);
                if !map::passable(scene.area, p) {
                    continue;
                }
                let c = p.as_vec2();
                let lo = c - Vec2::splat(0.5);
                let hi = c + Vec2::splat(0.5);
                for (a, b, show) in [
                    (lo, Vec2::new(hi.x, lo.y), true),
                    (lo, Vec2::new(lo.x, hi.y), true),
                    (
                        Vec2::new(lo.x, hi.y),
                        hi,
                        !map::passable(scene.area, p + IVec2::Y),
                    ),
                    (
                        Vec2::new(hi.x, lo.y),
                        hi,
                        !map::passable(scene.area, p + IVec2::X),
                    ),
                ] {
                    if show {
                        let dir = b - a; // each edge is exactly one metre
                        let rim = Vec2::new(-dir.y, dir.x) * 0.016;
                        if scene.area == Area::Town {
                            // A survey score in the flags, with just a little chipped lip
                            // at the junction. A continuous white bevel would light up in shade.
                            ink.line(a, b, 0.036, 0.021, [0.061, 0.050, 0.058, 0.34]);
                            ink.line(
                                a + rim,
                                a + rim + dir * 0.14,
                                0.039,
                                0.012,
                                [0.58, 0.49, 0.39, 0.29],
                            );
                        } else {
                            ink.line(a, b, 0.036, 0.032, [0.015, 0.019, 0.030, 0.60]);
                            ink.line(a + rim, b + rim, 0.039, 0.013, [0.69, 0.72, 0.64, 0.36]);
                        }
                    }
                }
                // Survey nail at the northwest corner of the square: still
                // legible where the edges traverse high-contrast cobbles.
                ink.quad(
                    lo + Vec2::splat(0.018),
                    lo + Vec2::splat(0.058),
                    0.040,
                    [0.79, 0.76, 0.58, 0.60],
                );
            }
        }
        ink.finish(mesh);
    }
    if !changed_area && !scene.is_changed() && !hover.is_changed() {
        return;
    }
    let mesh = meshes.get_mut(&handles.cursor).unwrap();
    let mut ink = Ink::take(mesh);
    ink.corners(scene.player, 0.048, [0.84, 0.80, 0.59, 0.46], 0.018);
    if let Some(cell) = hover.cell {
        let color = if hover.reachable {
            [0.40, 0.91, 0.81, 0.92]
        } else {
            [0.60, 0.67, 0.72, 0.70]
        };
        let center = cell.as_vec2();
        let y = (hover.surface_y + 0.04).max(0.068);
        ink.quad(
            center - Vec2::splat(0.46),
            center + Vec2::splat(0.46),
            y,
            [color[0], color[1], color[2], 0.22],
        );
        ink.corners(cell, y + 0.003, [0.033, 0.026, 0.056, 0.86], 0.090);
        ink.corners(cell, y + 0.007, color, 0.048);
        let mut from = scene.player;
        for &step in &hover.path {
            let start = from.as_vec2();
            let end = step.as_vec2();
            let border = [0.018, 0.065, 0.068, 0.92];
            let tint = [0.24, 0.96, 0.76, 0.97];
            ink.line(
                start.lerp(end, 0.17),
                start.lerp(end, 0.80),
                0.061,
                0.093,
                border,
            );
            ink.quad(
                end - Vec2::splat(0.079),
                end + Vec2::splat(0.079),
                0.062,
                border,
            );
            ink.line(
                start.lerp(end, 0.20),
                start.lerp(end, 0.77),
                0.063,
                0.047,
                tint,
            );
            ink.quad(
                end - Vec2::splat(0.053),
                end + Vec2::splat(0.053),
                0.065,
                tint,
            );
            from = step;
        }
    }
    ink.finish(mesh);
}
