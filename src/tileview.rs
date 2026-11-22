//! One map-space contract for picking, hover, route previews, and danger telegraphs.
//! Three persistent meshes are reused; tile feedback never spawns per-frame entities.
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
    game::{Interface, Outing, Page, ground},
    map::{self, Area},
};

pub struct TileViewPlugin;

/// `cell` is None outside the map, behind UI, above the horizon, or during a
/// keyboard takeover. `path` contains real eight-way steps, excluding the player.
#[derive(Resource, Default)]
pub struct HoverTile {
    pub cell: Option<IVec2>,
    pub path: Vec<IVec2>,
    pub reachable: bool,
    pub interactive: bool,
    pub dangerous: bool,
    area: Option<Area>,
    surface_y: f32,
    mouse_active: bool,
}

#[derive(Resource)]
struct TileMeshes {
    grid: Handle<Mesh>,
    danger: Handle<Mesh>,
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
        danger: meshes.add(empty_mesh()),
        cursor: meshes.add(empty_mesh()),
    };
    for (name, handle) in [
        ("Map tile seams", &handles.grid),
        ("Next-turn danger and vent grates", &handles.danger),
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

#[derive(Default)]
struct PointerMode {
    position: Option<Vec2>,
    active: bool,
    area: Option<Area>,
    page: Option<Page>,
}

fn sample_hover(
    window: Single<&Window, With<PrimaryWindow>>,
    camera: Single<(&Camera, &GlobalTransform), With<Camera3d>>,
    keys: Res<ButtonInput<KeyCode>>,
    mouse: Res<ButtonInput<MouseButton>>,
    game: Res<Outing>,
    sight: Res<crate::sight::WorldSight>,
    mut ui: ResMut<Interface>,
    mut hover: ResMut<HoverTile>,
    mut mode: Local<PointerMode>,
) {
    let position = window.cursor_position();
    let moved = position != mode.position;
    mode.position = position;
    if moved || mouse.just_pressed(MouseButton::Left) || ui.page == Page::Roam {
        mode.active = true;
    }
    if matches!(ui.page, Page::Aim | Page::Look)
        && keys.get_just_pressed().any(|k| {
            matches!(
                k,
                KeyCode::ArrowUp
                    | KeyCode::ArrowDown
                    | KeyCode::ArrowLeft
                    | KeyCode::ArrowRight
                    | KeyCode::KeyW
                    | KeyCode::KeyS
                    | KeyCode::KeyA
                    | KeyCode::KeyD
                    | KeyCode::Numpad1
                    | KeyCode::Numpad2
                    | KeyCode::Numpad3
                    | KeyCode::Numpad4
                    | KeyCode::Numpad6
                    | KeyCode::Numpad7
                    | KeyCode::Numpad8
                    | KeyCode::Numpad9
            )
        })
    {
        mode.active = false;
    }
    let switched = mode.area != Some(game.area);
    mode.area = Some(game.area);
    let page_changed = mode.page != Some(ui.page);
    mode.page = Some(ui.page);
    if hover.mouse_active != mode.active {
        hover.mouse_active = mode.active;
    }
    let hit = if switched || !mode.active || !matches!(ui.page, Page::Roam | Page::Aim | Page::Look)
    {
        None
    } else {
        position
            .filter(|&p| {
                !crate::book::pointer_over_book(
                    p,
                    window.resolution.size(),
                    ui.page,
                    ui.hud,
                    game.area,
                )
            })
            .and_then(|p| {
                camera
                    .0
                    .viewport_to_world(camera.1, p)
                    .ok()
                    .and_then(|ray| sight.pick(game.area, ray))
            })
    };
    let cell = hit.map(|(cell, _)| cell);
    let surface_y = hit.map_or(0.0, |(_, y)| y);
    if cell != hover.cell
        || hover.area != Some(game.area)
        || page_changed
        || (hover.surface_y - surface_y).abs() > 0.04
    {
        hover.cell = cell;
        hover.area = Some(game.area);
        hover.surface_y = surface_y;
    }
    if let Some(p) = cell
        && matches!(ui.page, Page::Aim | Page::Look)
        && (moved || page_changed || mouse.just_pressed(MouseButton::Left))
    {
        ui.cursor = p;
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
    game: Res<Outing>,
    ui: Res<Interface>,
    mut hover: ResMut<HoverTile>,
    handles: Res<TileMeshes>,
    mut meshes: ResMut<Assets<Mesh>>,
    sight: Res<crate::sight::WorldSight>,
    mut last_area: Local<Option<Area>>,
) {
    // Rules have now resolved this frame's action. Preview from the actual new
    // position, not the pre-action hover sample used to dispatch the click.
    if game.is_changed() || ui.is_changed() || hover.is_changed() {
        if hover.area != Some(game.area) {
            hover.cell = None;
            hover.area = Some(game.area);
        }
        let cell = hover.cell;
        hover.path.clear();
        hover.interactive = cell.is_some_and(|p| game.is_interactable(p));
        hover.dangerous = cell.is_some_and(|p| game.danger_at(p));
        hover.reachable = if let Some(p) = cell {
            match ui.page {
                Page::Aim => game.can_toss(p),
                Page::Look => true,
                _ => game.path_to(p, &mut hover.path),
            }
        } else {
            false
        };
    }
    let changed_area = *last_area != Some(game.area);
    if changed_area {
        *last_area = Some(game.area);
        let mesh = meshes.get_mut(&handles.grid).unwrap();
        let mut ink = Ink::take(mesh);
        for z in map::MIN.y..=map::MAX.y {
            for x in map::MIN.x..=map::MAX.x {
                let p = IVec2::new(x, z);
                if !map::passable(game.area, p) {
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
                        !map::passable(game.area, p + IVec2::Y),
                    ),
                    (
                        Vec2::new(hi.x, lo.y),
                        hi,
                        !map::passable(game.area, p + IVec2::X),
                    ),
                ] {
                    if show {
                        let dir = b - a; // each edge is exactly one metre
                        let rim = Vec2::new(-dir.y, dir.x) * 0.016;
                        if game.area == Area::Town {
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
    if changed_area || game.is_changed() {
        let mesh = meshes.get_mut(&handles.danger).unwrap();
        let mut ink = Ink::take(mesh);
        if game.area == Area::Cistern {
            for z in map::MIN.y..=map::MAX.y {
                for x in map::MIN.x..=map::MAX.x {
                    let p = IVec2::new(x, z);
                    if map::glyph(game.area, p) == b'P' {
                        ink.corners(p, 0.047, [0.92, 0.55, 0.15, 0.68], 0.028);
                    }
                    if game.danger_at(p) {
                        let c = p.as_vec2();
                        ink.quad(
                            c - Vec2::splat(0.43),
                            c + Vec2::splat(0.43),
                            0.044,
                            [0.95, 0.055, 0.035, 0.24],
                        );
                        ink.corners(p, 0.052, [1.0, 0.11, 0.065, 0.95], 0.055);
                        for offset in [-0.22, 0.0, 0.22] {
                            ink.line(
                                c + Vec2::new(-0.28, offset - 0.10),
                                c + Vec2::new(0.28, offset + 0.10),
                                0.051,
                                0.024,
                                [0.95, 0.12, 0.06, 0.72],
                            );
                        }
                    }
                }
            }
            for (i, enemy) in game.expedition.enemies.iter().enumerate() {
                if enemy.hp == 0 {
                    continue;
                }
                ink.corners(enemy.pos, 0.054, [0.92, 0.14, 0.10, 0.80], 0.026);
                if let Some(target) = enemy.warn
                    && game.expedition.attack_reaches(i, target)
                {
                    ink.line(
                        enemy.pos.as_vec2(),
                        target.as_vec2(),
                        0.058,
                        0.048,
                        [1.0, 0.13, 0.065, 0.82],
                    );
                }
            }
        }
        ink.finish(mesh);
    }
    if !changed_area && !game.is_changed() && !ui.is_changed() && !hover.is_changed() {
        return;
    }
    let mesh = meshes.get_mut(&handles.cursor).unwrap();
    let mut ink = Ink::take(mesh);
    if matches!(ui.page, Page::Roam | Page::Aim | Page::Look) {
        // A quiet open foot bracket makes the actual occupied cell explicit.
        ink.corners(game.player, 0.048, [0.84, 0.80, 0.59, 0.46], 0.018);
        let mouse_cell = (hover.area == Some(game.area))
            .then_some(hover.cell)
            .flatten();
        let cell = mouse_cell.or_else(|| {
            (!hover.mouse_active && matches!(ui.page, Page::Aim | Page::Look)).then_some(ui.cursor)
        });
        if let Some(cell) = cell {
            let reachable = if ui.page == Page::Aim {
                game.can_toss(cell)
            } else if ui.page == Page::Look {
                true
            } else {
                hover.reachable
            };
            let interact = game.is_interactable(cell);
            let danger = game.danger_at(cell);
            let color = if danger || !reachable {
                [1.0, 0.20, 0.13, 0.93]
            } else if interact {
                [1.0, 0.72, 0.22, 0.94]
            } else if ui.page == Page::Look {
                [0.63, 0.77, 1.0, 0.94]
            } else {
                [0.40, 0.91, 0.81, 0.92]
            };
            let c = cell.as_vec2();
            let wall = map::glyph(game.area, cell) == b'#';
            let y = if wall {
                if mouse_cell.is_some() {
                    hover.surface_y + 0.04
                } else {
                    let ray = Ray3d::new(Vec3::new(cell.x as f32, 32., cell.y as f32), Dir3::NEG_Y);
                    sight.pick(game.area, ray).map_or(0., |(_, y)| y) + 0.04
                }
            } else {
                0.068
            };
            if !wall {
                ink.quad(
                    c - Vec2::splat(0.46),
                    c + Vec2::splat(0.46),
                    y,
                    [color[0], color[1], color[2], 0.32],
                );
            }
            ink.corners(cell, y + 0.003, [0.033, 0.026, 0.056, 0.86], 0.090);
            ink.corners(cell, y + 0.007, color, if interact { 0.065 } else { 0.048 });
            if interact {
                let h = c + Vec2::new(0.0, -0.28);
                ink.line(h - Vec2::X * 0.09, h, y + 0.005, 0.035, color);
                ink.line(h, h + Vec2::Y * 0.10, y + 0.005, 0.035, color);
            }
            if ui.page == Page::Roam && mouse_cell.is_some() {
                let mut from = game.player;
                let mut unsafe_route = game.threatened();
                for &step in &hover.path {
                    unsafe_route |= game.danger_at(step);
                    let tint = if unsafe_route {
                        [1.0, 0.34, 0.15, 0.97]
                    } else {
                        [0.24, 0.96, 0.76, 0.97]
                    };
                    let start = from.as_vec2();
                    let end = step.as_vec2();
                    let border = if unsafe_route {
                        [0.13, 0.026, 0.018, 0.92]
                    } else {
                        [0.018, 0.065, 0.068, 0.92]
                    };
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
                if interact && let Some(&end) = hover.path.last() {
                    ink.corners(end, 0.066, [1.0, 0.73, 0.27, 0.75], 0.025);
                }
            }
            if ui.page == Page::Aim {
                let from = ground(game.player) + Vec3::Y * 0.45;
                let to = ground(cell) + Vec3::Y * 0.10;
                for n in 0..16 {
                    let t = (n as f32 + 0.5) / 16.0;
                    let p = from.lerp(to, t) + Vec3::Y * (4.0 * t * (1.0 - t));
                    let center = Vec2::new(p.x, p.z);
                    ink.quad(
                        center - Vec2::splat(0.025),
                        center + Vec2::splat(0.025),
                        p.y,
                        color,
                    );
                }
            }
        }
    }
    ink.finish(mesh);
}
