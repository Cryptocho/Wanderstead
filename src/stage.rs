//! The orchard's real PBR surfaces, daylight, and state-driven world feedback.
//! Props are original Blender exports; regenerate with art/inks/make_inks.py.
use std::collections::HashMap;

use bevy::{
    core_pipeline::{Skybox, bloom::Bloom},
    pbr::{
        CascadeShadowConfigBuilder, DirectionalLightShadowMap, ExtendedMaterial, MaterialExtension,
    },
    prelude::*,
    render::render_resource::{AsBindGroup, ShaderRef, TextureViewDescriptor, TextureViewDimension},
};

use crate::{Frame, game::*};

pub struct StagePlugin;

impl Plugin for StagePlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<PaletteState>()
            .init_resource::<SurfaceCache>()
            .init_resource::<StageAnimation>()
            .add_plugins(MaterialPlugin::<WorldMaterial>::default())
            .add_systems(Startup, setup_stage)
            .add_systems(
                Update,
                (
                    toggle_palette,
                    add_atmosphere,
                    bind_sky.run_if(resource_exists::<MorningSky>),
                    bind_stage_surfaces,
                    bind_tree_nodes,
                    animate_trees,
                    breakfast_feedback,
                    world_markers,
                    sync_palette,
                )
                    .chain()
                    .in_set(Frame::Draw),
            );
    }
}

/// F7 changes only the surface treatment; camera, light, geometry and fog stay fixed.
#[derive(Resource, Clone, Debug)]
pub struct PaletteState {
    pub enabled: bool,
    pub world_texels: f32,
    /// Soft illumination levels per exposure stop, not a limited HDR color range.
    pub light_steps: f32,
    pub dither_strength: f32,
}

impl Default for PaletteState {
    fn default() -> Self {
        Self {
            enabled: true,
            world_texels: 24.0,
            light_steps: 4.0,
            dither_strength: 0.18,
        }
    }
}

impl PaletteState {
    fn uniform(&self) -> Vec4 {
        Vec4::new(
            if self.enabled { 1.0 } else { 0.0 },
            self.world_texels,
            self.light_steps,
            self.dither_strength,
        )
    }
}

#[derive(Asset, AsBindGroup, Reflect, Debug, Clone)]
pub struct WorldPalette {
    #[uniform(100)]
    pub settings: Vec4,
    /// xyz = player chest, w = whether this is an occluding landscape surface.
    #[uniform(100)]
    pub focus: Vec4,
    #[uniform(100)]
    pub eye: Vec4,
}

impl MaterialExtension for WorldPalette {
    fn fragment_shader() -> ShaderRef {
        "inks/world_palette.wgsl".into()
    }
}

type WorldMaterial = ExtendedMaterial<StandardMaterial, WorldPalette>;

#[derive(Resource, Default)]
struct SurfaceCache(HashMap<AssetId<StandardMaterial>, Handle<WorldMaterial>>);

#[derive(Component)]
struct StageRoot(bool);
#[derive(Component)]
struct Inked;
#[derive(Component)]
struct TreeNodeBound;
#[derive(Component)]
struct TreeSway {
    rest: Transform,
    trunk: bool,
}
#[derive(Component)]
struct RipeFruit;

#[derive(Component)]
struct Curd;
#[derive(Component)]
struct Juice;
#[derive(Component)]
struct Spoon;
#[derive(Component)]
struct Portion {
    index: u8,
    age: f32,
}

#[derive(Component)]
enum WorldMarker {
    Position,
    Facing,
    Cursor,
    CursorWash,
    Range(IVec2),
    Arc(u8),
}

#[derive(Resource)]
struct StageProps {
    plum: Handle<Scene>,
    position: Handle<StandardMaterial>,
    inspect: Handle<StandardMaterial>,
    valid: Handle<StandardMaterial>,
    invalid: Handle<StandardMaterial>,
    range: Handle<StandardMaterial>,
    muted: Handle<StandardMaterial>,
    wash: Handle<StandardMaterial>,
}

#[derive(Resource)]
struct StageAnimation {
    day: u32,
    turn: u32,
    shake_age: f32,
    meal_age: f32,
    portion_count: u8,
    meal_started: bool,
}

impl Default for StageAnimation {
    fn default() -> Self {
        Self {
            day: u32::MAX,
            turn: u32::MAX,
            shake_age: 100.0,
            meal_age: 100.0,
            portion_count: 0,
            meal_started: false,
        }
    }
}

const BOWL_POSITION: Vec3 = Vec3::new(-3.80, 0.79, 3.08);

#[derive(Resource)]
struct MorningSky(Handle<Image>);

fn setup_stage(
    mut commands: Commands,
    assets: Res<AssetServer>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    commands.insert_resource(MorningSky(assets.load("inks/morning-sky.png")));
    commands.insert_resource(ClearColor(Color::srgb(0.61, 0.76, 0.85)));
    commands.insert_resource(AmbientLight {
        color: Color::srgb(0.80, 0.89, 1.0),
        brightness: 7900.0,
        ..default()
    });
    commands.insert_resource(DirectionalLightShadowMap { size: 2048 });
    commands.spawn((
        Name::new("Late morning sun"),
        DirectionalLight {
            color: Color::srgb(1.0, 0.94, 0.82),
            illuminance: 65_000.0,
            shadows_enabled: true,
            shadow_depth_bias: 0.008,
            shadow_normal_bias: 0.65,
            ..default()
        },
        Transform::from_xyz(-18.0, 33.0, 23.0).looking_at(Vec3::ZERO, Vec3::Y),
        CascadeShadowConfigBuilder {
            num_cascades: 4,
            minimum_distance: 0.1,
            first_cascade_far_bound: 13.0,
            maximum_distance: 95.0,
            overlap_proportion: 0.2,
        }
        .build(),
    ));
    commands.spawn((
        Name::new("Orchard and roadside inn"),
        SceneRoot(assets.load("lanes/orchard.glb#Scene0")),
        StageRoot(true),
    ));
    commands.spawn((
        Name::new("Breakfast bowl on the table"),
        SceneRoot(assets.load("inks/bowl.glb#Scene0")),
        Transform::from_translation(BOWL_POSITION),
        StageRoot(false),
    ));
    commands.spawn((
        SceneRoot(assets.load("inks/curd.glb#Scene0")),
        Transform::from_translation(BOWL_POSITION + Vec3::Y * 0.153),
        StageRoot(false),
        Curd,
    ));
    commands.spawn((
        SceneRoot(assets.load("inks/juice.glb#Scene0")),
        Transform::from_translation(BOWL_POSITION + Vec3::Y * 0.16),
        Visibility::Hidden,
        StageRoot(false),
        Juice,
    ));
    commands.spawn((
        SceneRoot(assets.load("inks/spoon.glb#Scene0")),
        Transform::from_translation(BOWL_POSITION + Vec3::new(0.14, 0.153, 0.11))
            .with_rotation(Quat::from_rotation_y(-0.35) * Quat::from_rotation_z(0.22)),
        StageRoot(false),
        Spoon,
    ));
    let wash = materials.add(StandardMaterial {
        base_color: Color::srgba(0.10, 0.075, 0.16, 0.55),
        alpha_mode: AlphaMode::Blend,
        unlit: true,
        ..default()
    });
    let mut marker_material = |color: Color| {
        materials.add(StandardMaterial {
            base_color: color,
            unlit: true,
            cull_mode: None,
            ..default()
        })
    };
    let props = StageProps {
        plum: assets.load("inks/plum-cut.glb#Scene0"),
        position: marker_material(Color::srgb(0.85, 0.74, 0.46)),
        inspect: marker_material(Color::srgb(0.45, 0.82, 0.95)),
        valid: marker_material(Color::srgb(0.98, 0.91, 0.62)),
        invalid: marker_material(Color::srgb(0.96, 0.35, 0.29)),
        range: marker_material(Color::srgb(0.78, 0.72, 0.46)),
        muted: marker_material(Color::srgb(0.39, 0.43, 0.38)),
        wash,
    };
    let ring: Handle<Mesh> = assets.load("inks/ring.glb#Mesh0/Primitive0");
    let facing: Handle<Mesh> = assets.load("inks/facing.glb#Mesh0/Primitive0");
    let brackets: Handle<Mesh> = assets.load("inks/brackets.glb#Mesh0/Primitive0");
    let dash: Handle<Mesh> = assets.load("inks/dash.glb#Mesh0/Primitive0");
    let dot: Handle<Mesh> = assets.load("inks/dot.glb#Mesh0/Primitive0");
    for (kind, mesh) in [
        (WorldMarker::Position, ring),
        (WorldMarker::Facing, facing),
        (WorldMarker::Cursor, brackets),
        (WorldMarker::CursorWash, meshes.add(Plane3d::default().mesh().size(0.9, 0.9))),
    ] {
        commands.spawn((
            kind,
            Mesh3d(mesh),
            MeshMaterial3d(props.position.clone()),
            Transform::default(),
            Visibility::Hidden,
            bevy::pbr::NotShadowCaster,
            Inked,
        ));
    }
    // A sparse perimeter, not a painted grid. Only the selected cell has brackets.
    for x in -3_i32..=3 {
        for z in -3_i32..=3 {
            if x.abs() + z.abs() == 3 {
                commands.spawn((
                    WorldMarker::Range(IVec2::new(x, z)),
                    Mesh3d(dash.clone()),
                    MeshMaterial3d(props.range.clone()),
                    Transform::default(),
                    Visibility::Hidden,
                    Inked,
                ));
            }
        }
    }
    for i in 1..=11 {
        commands.spawn((
            WorldMarker::Arc(i),
            Mesh3d(dot.clone()),
            MeshMaterial3d(props.valid.clone()),
            Transform::default(),
            Visibility::Hidden,
            Inked,
        ));
    }
    commands.insert_resource(props);
}

fn add_atmosphere(mut commands: Commands, cameras: Query<Entity, Added<Camera3d>>) {
    for entity in &cameras {
        commands.entity(entity).insert((
            DistanceFog {
                color: Color::srgb(0.61, 0.76, 0.85),
                directional_light_color: Color::srgba(1.0, 0.86, 0.65, 0.18),
                directional_light_exponent: 12.0,
                falloff: FogFalloff::Linear {
                    start: 33.0,
                    end: 105.0,
                },
            },
            Bloom {
                intensity: 0.035,
                ..Bloom::NATURAL
            },
        ));
    }
}

fn bind_sky(
    mut commands: Commands,
    sky: Res<MorningSky>,
    mut images: ResMut<Assets<Image>>,
    camera: Single<Entity, With<Camera3d>>,
) {
    let Some(image) = images.get_mut(&sky.0) else { return };
    image.reinterpret_stacked_2d_as_array(6);
    image.texture_view_descriptor = Some(TextureViewDescriptor {
        dimension: Some(TextureViewDimension::Cube),
        ..default()
    });
    commands.entity(*camera).insert(Skybox { image: sky.0.clone(), brightness: 6400.0, ..default() });
    commands.remove_resource::<MorningSky>();
}

fn stage_owner(
    mut entity: Entity,
    roots: &Query<&StageRoot>,
    parents: &Query<&ChildOf>,
) -> Option<bool> {
    loop {
        if let Ok(root) = roots.get(entity) {
            return Some(root.0);
        }
        let Ok(parent) = parents.get(entity) else {
            return None;
        };
        entity = parent.parent();
    }
}

fn bind_stage_surfaces(
    mut commands: Commands,
    surfaces: Query<(Entity, &MeshMaterial3d<StandardMaterial>), Without<Inked>>,
    roots: Query<&StageRoot>,
    parents: Query<&ChildOf>,
    originals: Res<Assets<StandardMaterial>>,
    mut materials: ResMut<Assets<WorldMaterial>>,
    mut cache: ResMut<SurfaceCache>,
    palette: Res<PaletteState>,
) {
    for (entity, source) in &surfaces {
        let Some(cutaway) = stage_owner(entity, &roots, &parents) else {
            continue;
        };
        let Some(base) = originals.get(&source.0) else {
            continue;
        };
        let material = cache
            .0
            .entry(source.0.id())
            .or_insert_with(|| {
                // Clone every actual glTF surface parameter. This preserves alpha clip,
                // copper's metal/roughness, texture maps and double-sided leaves.
                materials.add(ExtendedMaterial {
                    base: base.clone(),
                    extension: WorldPalette {
                        settings: palette.uniform(),
                        focus: Vec4::new(0.0, 0.85, 0.0, if cutaway { 1.0 } else { 0.0 }),
                        eye: Vec4::ZERO,
                    },
                })
            })
            .clone();
        commands
            .entity(entity)
            .remove::<MeshMaterial3d<StandardMaterial>>()
            .insert((MeshMaterial3d(material), Inked));
    }
}

fn toggle_palette(keys: Res<ButtonInput<KeyCode>>, mut palette: ResMut<PaletteState>) {
    if keys.just_pressed(KeyCode::F7) {
        palette.enabled = !palette.enabled;
    }
}

fn sync_palette(
    palette: Res<PaletteState>,
    outing: Res<Outing>,
    cameras: Query<&Transform, With<Camera3d>>,
    cache: Res<SurfaceCache>,
    mut materials: ResMut<Assets<WorldMaterial>>,
) {
    let Ok(camera) = cameras.single() else {
        return;
    };
    let focus = ground(outing.player) + Vec3::Y * 0.85;
    let eye = camera.translation.extend(0.0);
    let settings = palette.uniform();
    for handle in cache.0.values() {
        let changed = materials.get(handle).is_some_and(|material| {
            let extension = &material.extension;
            extension.settings != settings
                || extension.focus.truncate() != focus
                || extension.eye != eye
        });
        if !changed {
            continue;
        }
        if let Some(material) = materials.get_mut(handle) {
            material.extension.settings = settings;
            material.extension.focus = focus.extend(material.extension.focus.w);
            material.extension.eye = eye;
        }
    }
}

fn bind_tree_nodes(
    mut commands: Commands,
    nodes: Query<(Entity, &Name, &Transform), Without<TreeNodeBound>>,
    roots: Query<&StageRoot>,
    parents: Query<&ChildOf>,
) {
    for (entity, name, transform) in &nodes {
        if stage_owner(entity, &roots, &parents).is_none() {
            continue;
        }
        commands.entity(entity).insert(TreeNodeBound);
        match name.as_str() {
            "canopy_main" => {
                commands.entity(entity).insert(TreeSway {
                    rest: *transform,
                    trunk: false,
                });
            }
            "trunk_main" => {
                commands.entity(entity).insert(TreeSway {
                    rest: *transform,
                    trunk: true,
                });
            }
            name if name.starts_with("ripe_plum_main_") => {
                commands.entity(entity).insert(RipeFruit);
            }
            _ => {}
        }
    }
}

fn animate_trees(
    outing: Res<Outing>,
    time: Res<Time>,
    mut animation: ResMut<StageAnimation>,
    mut trees: Query<(&TreeSway, &mut Transform)>,
    mut fruit: Query<&mut Visibility, With<RipeFruit>>,
) {
    if animation.day != outing.day {
        animation.day = outing.day;
        animation.turn = u32::MAX;
        animation.shake_age = 100.0;
        animation.meal_age = 100.0;
        animation.meal_started = false;
    }
    if animation.turn != outing.turn {
        animation.turn = outing.turn;
        if outing.action == ActionKind::Shake {
            animation.shake_age = 0.0;
        }
    }
    animation.shake_age = (animation.shake_age + time.delta_secs()).min(100.0);
    let t = animation.shake_age;
    let shake = if t < 1.7 {
        (t * 27.0).sin() * (-t * 3.4).exp()
    } else {
        0.0
    };
    let breeze = (time.elapsed_secs() * 1.2).sin() * 0.0025;
    for (tree, mut transform) in &mut trees {
        let amplitude = if tree.trunk { 0.014 } else { 0.065 };
        transform.rotation = tree.rest.rotation
            * Quat::from_rotation_z(shake * amplitude + if tree.trunk { 0.0 } else { breeze });
    }
    for mut visible in &mut fruit {
        *visible = if outing.tree_shaken {
            Visibility::Hidden
        } else {
            Visibility::Inherited
        };
    }
}

fn portion_position(index: u8, count: u8) -> Vec3 {
    let angle = index as f32 * 2.399_963;
    let radius = 0.208 * ((index as f32 + 0.5) / count.max(1) as f32).sqrt();
    BOWL_POSITION + Vec3::new(radius * angle.cos(), 0.158, radius * angle.sin())
}

fn breakfast_feedback(
    mut commands: Commands,
    outing: Res<Outing>,
    interface: Res<Interface>,
    props: Res<StageProps>,
    time: Res<Time>,
    mut animation: ResMut<StageAnimation>,
    mut portions: Query<
        (Entity, &mut Portion, &mut Transform),
        (Without<Curd>, Without<Juice>, Without<Spoon>),
    >,
    mut curd: Query<&mut Transform, (With<Curd>, Without<Juice>, Without<Spoon>)>,
    mut juice: Query<
        (&mut Transform, &mut Visibility),
        (With<Juice>, Without<Curd>, Without<Spoon>),
    >,
    mut spoon: Query<&mut Transform, (With<Spoon>, Without<Curd>, Without<Juice>)>,
) {
    let count = if interface.page == Page::Bowl {
        interface.filling
    } else {
        outing.breakfast.unwrap_or(0)
    };
    if outing.breakfast.is_some() && !animation.meal_started {
        animation.meal_started = true;
        animation.meal_age = 0.0;
    }
    animation.meal_age = (animation.meal_age + time.delta_secs()).min(100.0);
    if count > animation.portion_count {
        for index in animation.portion_count..count {
            commands.spawn((
                Name::new(format!("Fresh cut plum {}", index + 1)),
                SceneRoot(props.plum.clone()),
                Transform::from_translation(portion_position(index, count) + Vec3::Y * 0.30),
                Portion { index, age: 0.0 },
                StageRoot(false),
            ));
        }
    }
    animation.portion_count = count;
    let eaten = if outing.breakfast.is_some() {
        ((animation.meal_age - 0.75) / 0.85).clamp(0.0, 1.0)
    } else {
        0.0
    };
    let crowded_scale = (8.0 / count.max(8) as f32).sqrt();
    for (entity, mut portion, mut transform) in &mut portions {
        if portion.index >= count {
            commands.entity(entity).despawn();
            continue;
        }
        portion.age += time.delta_secs();
        let falling = (1.0 - portion.age / 0.42).clamp(0.0, 1.0);
        transform.translation = portion_position(portion.index, count)
            + Vec3::Y * (0.30 * falling * falling - 0.052 * eaten);
        transform.rotation = Quat::from_rotation_y(portion.index as f32 * 2.399_963);
        transform.scale = Vec3::splat(crowded_scale * (1.0 - 0.65 * eaten));
    }
    for mut transform in &mut curd {
        transform.translation.y = BOWL_POSITION.y + 0.153 - 0.052 * eaten;
        transform.scale = Vec3::new(1.0 - 0.22 * eaten, 1.0, 1.0 - 0.22 * eaten);
    }
    for (mut transform, mut visibility) in &mut juice {
        *visibility = if count > 0 {
            Visibility::Inherited
        } else {
            Visibility::Hidden
        };
        transform.translation.y = BOWL_POSITION.y + 0.160 - 0.052 * eaten;
        let spread = (0.30 + count as f32 * 0.13).min(1.10) * (1.0 - 0.22 * eaten);
        transform.scale = Vec3::new(spread, 1.0, spread);
    }
    let scoop = if outing.breakfast.is_some() && animation.meal_age < 1.6 {
        ((animation.meal_age - 0.30) / 1.2 * std::f32::consts::PI)
            .clamp(0.0, std::f32::consts::PI)
            .sin()
    } else {
        0.0
    };
    for mut transform in &mut spoon {
        transform.translation =
            BOWL_POSITION + Vec3::new(0.14 + 0.045 * scoop, 0.153 + 0.115 * scoop, 0.11);
        transform.rotation =
            Quat::from_rotation_y(-0.35) * Quat::from_rotation_z(0.22 - 0.35 * scoop);
    }
}

fn world_markers(
    outing: Res<Outing>,
    interface: Res<Interface>,
    props: Res<StageProps>,
    mut markers: Query<(
        &WorldMarker,
        &mut Transform,
        &mut Visibility,
        &mut MeshMaterial3d<StandardMaterial>,
    )>,
) {
    if !outing.is_changed() && !interface.is_changed() { return; }
    let aim = interface.page == Page::Aim;
    let look = interface.page == Page::Look;
    let cursor = interface.cursor;
    let legal = outing.can_toss(cursor);
    let target_material = if look {
        &props.inspect
    } else if legal {
        &props.valid
    } else {
        &props.invalid
    };
    for (kind, mut transform, mut visibility, mut material) in &mut markers {
        transform.rotation = Quat::IDENTITY;
        let visible = match kind {
            WorldMarker::Position => {
                transform.translation = ground(outing.player) + Vec3::Y * 0.012;
                material.0 = props.position.clone();
                interface.page == Page::Roam || aim || look
            }
            WorldMarker::Facing => {
                transform.translation = ground(outing.player) + Vec3::Y * 0.014;
                let facing = outing.facing;
                transform.rotation =
                    Quat::from_rotation_y(-(facing.x as f32).atan2(-facing.y as f32));
                material.0 = props.position.clone();
                interface.page == Page::Roam || aim || look
            }
            WorldMarker::Cursor => {
                transform.translation = ground(cursor) + Vec3::Y * 0.067;
                material.0 = target_material.clone();
                aim || look
            }
            WorldMarker::CursorWash => {
                transform.translation = ground(cursor) + Vec3::Y * 0.060;
                material.0 = props.wash.clone();
                aim || look
            }
            WorldMarker::Range(offset) => {
                let cell = outing.player + *offset;
                transform.translation = ground(cell) + Vec3::Y * 0.014;
                transform.rotation = Quat::from_rotation_y(if offset.x == 0 {
                    0.0
                } else if offset.y == 0 {
                    std::f32::consts::FRAC_PI_2
                } else {
                    (offset.x * offset.y).signum() as f32 * std::f32::consts::FRAC_PI_4
                });
                material.0 = if outing.can_toss(cell) {
                    props.range.clone()
                } else {
                    props.muted.clone()
                };
                aim && walkable(cell)
            }
            WorldMarker::Arc(index) => {
                let t = *index as f32 / 12.0;
                let start = ground(outing.player) + Vec3::Y * 0.45;
                let end = ground(cursor) + Vec3::Y * 0.12;
                transform.translation = start.lerp(end, t) + Vec3::Y * (4.0 * t * (1.0 - t) * 1.1);
                material.0 = target_material.clone();
                aim && (cursor - outing.player).abs().element_sum() <= 3 && cursor != outing.player
            }
        };
        *visibility = if visible {
            Visibility::Inherited
        } else {
            Visibility::Hidden
        };
    }
}
