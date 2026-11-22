//! Shared lit scenery, distortion-free pixel materials and diegetic interactions.
use std::collections::HashMap;

use bevy::{
    core_pipeline::Skybox,
    pbr::{ExtendedMaterial, MaterialExtension},
    prelude::*,
    render::render_resource::{
        AsBindGroup, Buffer, ShaderRef, TextureViewDescriptor, TextureViewDimension,
    },
};

use crate::sight::{MovingSolid, SightMask, part};
use crate::{Frame, game::*};
use crate::{delve::Quest, map::Area};

pub struct StagePlugin;

impl Plugin for StagePlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<PaletteState>()
            .init_resource::<SurfaceCache>()
            .init_resource::<StageAnimation>()
            .add_systems(PreStartup, create_regions)
            .add_plugins(MaterialPlugin::<WorldMaterial>::default())
            .add_systems(Startup, setup_stage)
            .add_systems(
                Update,
                (
                    toggle_palette,
                    switch_region,
                    bind_sky.run_if(resource_exists::<MorningSky>),
                    bind_stage_surfaces,
                    bind_tree_nodes,
                    animate_trees,
                    breakfast_feedback,
                    expedition_props,
                    sync_palette,
                )
                    .chain()
                    .in_set(Frame::Draw),
            );
    }
}

/// F7 compares the painted world and the uninked PBR scene under identical light and lens.
#[derive(Resource, Clone, Debug, bevy::render::extract_resource::ExtractResource)]
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
            world_texels: 32.0,
            light_steps: 3.25,
            dither_strength: 0.22,
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

#[derive(Asset, AsBindGroup, TypePath, Debug, Clone)]
pub struct WorldPalette {
    #[uniform(100)]
    pub settings: Vec4,
    /// 1 = town flags, 2 = cistern flags. Other surfaces keep their own patina.
    #[uniform(100)]
    pub floor_style: Vec4,
    #[uniform(100)]
    pub sunward: Vec4,
    /// The town GLB stores opaque color maps with hardware linear filtering.
    #[uniform(100)]
    single_tap: u32,
    #[texture(101)]
    pub site: Handle<Image>,
    #[storage(102, read_only, buffer, visibility(fragment))]
    hidden_models: Buffer,
    #[texture(105)]
    #[sampler(106)]
    shelter: Handle<Image>,
    #[texture(107)]
    #[sampler(108)]
    contact: Handle<Image>,
}

impl MaterialExtension for WorldPalette {
    fn fragment_shader() -> ShaderRef {
        "inks/world_palette.wgsl".into()
    }
}

type WorldMaterial = ExtendedMaterial<StandardMaterial, WorldPalette>;

#[derive(Resource, Default)]
struct SurfaceCache(HashMap<AssetId<StandardMaterial>, Handle<WorldMaterial>>);

#[derive(Component, Clone, Copy)]
enum StageRoot {
    District,
    Table,
}
#[derive(Component)]
struct Inked;
#[derive(Component)]
struct TreeNodeBound;
#[derive(Component)]
struct TreeSway {
    rest: Transform,
    kind: SwayKind,
    phase: f32,
}

#[derive(Clone, Copy)]
enum SwayKind {
    Crown { shake: bool },
    Trunk,
    Awning,
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

/// Lights, meshes and effects for both floors share local tile coordinates.
#[derive(Resource, Clone, Copy)]
pub(crate) struct Regions {
    pub town: Entity,
    pub cistern: Entity,
}
impl Regions {
    pub fn root(self, area: Area) -> Entity {
        match area {
            Area::Town => self.town,
            Area::Cistern => self.cistern,
        }
    }
}
#[derive(Resource)]
struct StageProps {
    plum: Handle<Scene>,
}

#[derive(Resource)]
pub(crate) struct SiteEdges(pub Handle<Image>);

#[derive(Resource)]
struct GroundContact(Handle<Image>);

#[derive(Resource)]
pub(crate) struct SkyShelter {
    town: Handle<Image>,
    cistern: Handle<Image>,
}
impl SkyShelter {
    pub fn at(&self, area: Area) -> &Handle<Image> {
        match area {
            Area::Town => &self.town,
            Area::Cistern => &self.cistern,
        }
    }
}

/// Four edge bits per tile: masonry contact, water, and the fountain apron.
/// This is baked once from the same topology used by the movers and the mouse.
fn site_edges(images: &mut Assets<Image>) -> SiteEdges {
    use crate::map::{self, HEIGHT, WIDTH};
    use bevy::{
        asset::RenderAssetUsages,
        render::render_resource::{Extent3d, TextureDimension, TextureFormat},
    };
    let mut data = Vec::with_capacity(WIDTH * HEIGHT * 8);
    for area in [Area::Town, Area::Cistern] {
        for z in map::MIN.y..=map::MAX.y {
            for x in map::MIN.x..=map::MAX.x {
                let mut wall = 0u8;
                let mut water = 0;
                let mut rim = 0;
                let mut shore = 0u8;
                for (i, d) in [IVec2::NEG_X, IVec2::X, IVec2::NEG_Y, IVec2::Y]
                    .into_iter()
                    .enumerate()
                {
                    let v = map::glyph(area, IVec2::new(x, z) + d);
                    if v != b'~' {
                        shore |= 1 << i;
                    }
                    if matches!(
                        v,
                        b'#' | b'C' | b'!' | b'R' | b'b' | b'T' | b't' | b'F' | b'f'
                    ) {
                        wall |= 1 << i;
                    }
                    if v == b'~' {
                        water |= 1 << i;
                    }
                    if matches!(v, b'f' | b'F') {
                        rim |= 1 << i;
                    }
                }
                data.extend_from_slice(&[wall, water, rim, shore]);
            }
        }
    }
    SiteEdges(images.add(Image::new(
        Extent3d {
            width: WIDTH as u32,
            height: (2 * HEIGHT) as u32,
            depth_or_array_layers: 1,
        },
        TextureDimension::D2,
        data,
        TextureFormat::Rgba8Unorm,
        RenderAssetUsages::RENDER_WORLD,
    )))
}

#[derive(Component)]
enum OldWorks {
    Wheel,
    ToolLid(Quat),
    TonicLid(Quat),
}

fn create_regions(
    mut commands: Commands,
    mut images: ResMut<Assets<Image>>,
    assets: Res<AssetServer>,
) {
    use bevy::image::{ImageLoaderSettings, ImageSampler};
    let field = |path| {
        assets.load_with_settings(path, |s: &mut ImageLoaderSettings| {
            s.is_srgb = false;
            s.sampler = ImageSampler::linear();
        })
    };
    commands.insert_resource(SkyShelter {
        town: field("inks/square-shelter.png"),
        cistern: field("inks/cistern-shelter.png"),
    });
    commands.insert_resource(GroundContact(field("inks/square-contact.png")));
    commands.insert_resource(site_edges(&mut images));
    let town = commands
        .spawn((
            Name::new("Copper Kettle Square"),
            Transform::default(),
            Visibility::Inherited,
        ))
        .id();
    let cistern = commands
        .spawn((
            Name::new("Spring cistern B1"),
            Transform::default(),
            Visibility::Hidden,
        ))
        .id();
    commands.insert_resource(Regions { town, cistern });
}

fn switch_region(
    g: Res<Outing>,
    roots: Res<Regions>,
    mut visible: Query<&mut Visibility>,
    mut old: Local<Option<Area>>,
) {
    if *old == Some(g.area) {
        return;
    }
    *old = Some(g.area);
    for area in [Area::Town, Area::Cistern] {
        if let Ok(mut v) = visible.get_mut(roots.root(area)) {
            *v = if g.area == area {
                Visibility::Inherited
            } else {
                Visibility::Hidden
            };
        }
    }
}

fn expedition_props(
    g: Res<Outing>,
    mut props: Query<(&OldWorks, &mut Transform, &mut Visibility)>,
) {
    for (part, mut t, mut vis) in &mut props {
        match part {
            OldWorks::Wheel => {
                let show = if g.expedition.quest == Quest::Seeking {
                    Visibility::Inherited
                } else {
                    Visibility::Hidden
                };
                vis.set_if_neq(show);
            }
            OldWorks::ToolLid(rest) | OldWorks::TonicLid(rest) => {
                let open = match part {
                    OldWorks::ToolLid(_) => g.expedition.seal_open,
                    _ => g.expedition.tonic_open,
                };
                let q = if open {
                    *rest * Quat::from_rotation_x(-1.65)
                } else {
                    *rest
                };
                if t.rotation != q {
                    t.rotation = q;
                }
            }
        }
    }
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

fn setup_stage(mut commands: Commands, assets: Res<AssetServer>, roots: Res<Regions>) {
    commands.insert_resource(MorningSky(assets.load("inks/morning-sky.png")));
    commands.spawn((
        Name::new("Town street and shops"),
        ChildOf(roots.town),
        SceneRoot(assets.load("lanes/orchard.glb#Scene0")),
        StageRoot::District,
    ));
    commands.spawn((
        Name::new("Old springworks"),
        ChildOf(roots.cistern),
        SceneRoot(assets.load("cistern/cistern.glb#Scene0")),
        StageRoot::District,
    ));
    for (area, path) in [
        (Area::Town, "market/market-trimmings.glb#Scene0"),
        (Area::Cistern, "market/spring-trimmings.glb#Scene0"),
    ] {
        commands.spawn((
            SceneRoot(assets.load(path)),
            ChildOf(roots.root(area)),
            StageRoot::District,
        ));
    }
    commands.spawn((
        Name::new("Breakfast bowl on the table"),
        ChildOf(roots.town),
        SceneRoot(assets.load("inks/bowl.glb#Scene0")),
        Transform::from_translation(BOWL_POSITION),
        StageRoot::Table,
    ));
    commands.spawn((
        SceneRoot(assets.load("inks/curd.glb#Scene0")),
        ChildOf(roots.town),
        Transform::from_translation(BOWL_POSITION + Vec3::Y * 0.153),
        StageRoot::Table,
        Curd,
    ));
    commands.spawn((
        SceneRoot(assets.load("inks/juice.glb#Scene0")),
        ChildOf(roots.town),
        Transform::from_translation(BOWL_POSITION + Vec3::Y * 0.16),
        Visibility::Hidden,
        StageRoot::Table,
        Juice,
    ));
    commands.spawn((
        ChildOf(roots.town),
        SceneRoot(assets.load("inks/spoon.glb#Scene0")),
        Transform::from_translation(BOWL_POSITION + Vec3::new(0.14, 0.153, 0.11))
            .with_rotation(Quat::from_rotation_y(-0.35) * Quat::from_rotation_z(0.22)),
        StageRoot::Table,
        Spoon,
    ));
    commands.insert_resource(StageProps {
        plum: assets.load("inks/plum-cut.glb#Scene0"),
    });
}

fn bind_sky(
    mut commands: Commands,
    sky: Res<MorningSky>,
    mut images: ResMut<Assets<Image>>,
    camera: Single<Entity, With<Camera3d>>,
) {
    let Some(image) = images.get_mut(&sky.0) else {
        return;
    };
    image.reinterpret_stacked_2d_as_array(6);
    image.texture_view_descriptor = Some(TextureViewDescriptor {
        dimension: Some(TextureViewDimension::Cube),
        ..default()
    });
    commands.entity(*camera).insert(Skybox {
        image: sky.0.clone(),
        brightness: crate::daylight::SQUARE.sky.skybox_brightness,
        ..default()
    });
    commands.remove_resource::<MorningSky>();
}

fn stage_of(
    mut entity: Entity,
    roots: &Query<&StageRoot>,
    parents: &Query<&ChildOf>,
) -> Option<StageRoot> {
    loop {
        if let Ok(&root) = roots.get(entity) {
            return Some(root);
        }
        let Ok(parent) = parents.get(entity) else {
            return None;
        };
        entity = parent.parent();
    }
}

fn bind_stage_surfaces(
    mut commands: Commands,
    surfaces: Query<
        (
            Entity,
            &MeshMaterial3d<StandardMaterial>,
            &Mesh3d,
            Option<&bevy::gltf::GltfMaterialName>,
        ),
        Without<Inked>,
    >,
    roots: Query<&StageRoot>,
    parents: Query<&ChildOf>,
    originals: Res<Assets<StandardMaterial>>,
    mut materials: ResMut<Assets<WorldMaterial>>,
    mut cache: ResMut<SurfaceCache>,
    palette: Res<PaletteState>,
    site: Res<SiteEdges>,
    shelter: Res<SkyShelter>,
    contact: Res<GroundContact>,
    sight: Res<SightMask>,
    names: Query<&Name>,
    flowing: Single<Entity, With<crate::spring::Refilled>>,
) {
    for (entity, source, mesh, name) in &surfaces {
        let Some(stage) = stage_of(entity, &roots, &parents) else {
            // Figure sheets and effects use their own material families.
            commands.entity(entity).insert(Inked);
            continue;
        };
        let Some(base) = originals.get(&source.0) else {
            continue;
        };
        let material = cache
            .0
            .entry(source.0.id())
            .or_insert_with(|| {
                let mut base = base.clone();
                if name.is_some_and(|n| n.0 == "Lane_Glass") {
                    // The authored emission map contains only two narrow warm
                    // cores; the surrounding panes keep their normal PBR.
                    base.emissive = LinearRgba::rgb(5.2, 3.1, 1.4);
                }
                materials.add(ExtendedMaterial {
                    base,
                    extension: WorldPalette {
                        sunward: crate::daylight::SQUARE
                            .to_sun()
                            .extend(crate::daylight::SQUARE.film.bloom_threshold),
                        single_tap: u32::from(name.is_some_and(|m| m.0.starts_with("Lane_"))),
                        settings: palette.uniform(),
                        hidden_models: sight.buffer.clone(),
                        shelter: shelter.town.clone(),
                        contact: contact.0.clone(),
                        floor_style: Vec4::new(
                            match name.map(|m| m.0.as_str()) {
                                Some("Lane_Paving") => 1.,
                                Some("Wet paving" | "Ivory mosaic") => 2.,
                                _ => 0.,
                            },
                            0.,
                            match name.map(|m| m.0.as_str()) {
                                Some("Lane_CrownFresh" | "Lane_Leaves") => 1.,
                                Some("Lane_CrownBlossom" | "Lane_Petals") => 2.,
                                _ => 0.,
                            },
                            if name.is_some_and(|m| m.0.starts_with("Lane_")) {
                                1.
                            } else {
                                0.
                            },
                        ),
                        site: site.0.clone(),
                    },
                })
            })
            .clone();
        if matches!(stage, StageRoot::Table) {
            commands.entity(entity).insert(part(Area::Town, "table"));
        }
        // The workshop's actual wheel fits the bronze socket: reuse its mesh
        // and material, so taking it and reinstalling it cannot change its design.
        let mut ancestor = entity;
        while let Ok(parent) = parents.get(ancestor) {
            ancestor = parent.parent();
            let Ok(name) = names.get(ancestor) else {
                continue;
            };
            let owner = match name.as_str() {
                "tool_lid" => "tool",
                "tonic_lid" => "tonic",
                "wheel_pickup" => "relic",
                _ => continue,
            };
            commands
                .entity(entity)
                .insert(MovingSolid(part(Area::Cistern, owner).0));
            if owner == "relic" {
                let fountain = part(Area::Town, "fountain");
                commands.spawn((
                    Name::new("Returned bronze handwheel"),
                    Mesh3d(mesh.0.clone()),
                    fountain,
                    MovingSolid(fountain.0),
                    MeshMaterial3d(material.clone()),
                    Transform::from_xyz(4., 1.03, 2.40).with_rotation(Quat::from_rotation_z(0.23)),
                    ChildOf(*flowing),
                    Inked,
                ));
            }
            break;
        }
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
    cache: Res<SurfaceCache>,
    mut materials: ResMut<Assets<WorldMaterial>>,
    shelter: Res<SkyShelter>,
    mut last: Local<Option<(Area, bool)>>,
) {
    let wet = outing.expedition.quest == Quest::Restored;
    let mark = (outing.area, wet);
    if *last == Some(mark) && !palette.is_changed() && !cache.is_changed() {
        return;
    }
    *last = Some(mark);
    let settings = palette.uniform();
    let field = shelter.at(outing.area);
    for handle in cache.0.values() {
        if materials.get(handle).is_some_and(|m| {
            m.extension.settings == settings
                && m.extension.shelter == *field
                && m.extension.floor_style.y == wet as u8 as f32
        }) {
            continue;
        }
        if let Some(material) = materials.get_mut(handle) {
            material.extension.settings = settings;
            material.extension.floor_style.y = wet as u8 as f32;
            material.extension.shelter = field.clone();
        }
    }
}

fn bind_tree_nodes(
    mut commands: Commands,
    nodes: Query<(Entity, &Name, &Transform), (Without<TreeNodeBound>, Without<Mesh3d>)>,
    roots: Query<&StageRoot>,
    parents: Query<&ChildOf>,
) {
    for (entity, name, transform) in &nodes {
        if stage_of(entity, &roots, &parents).is_none() {
            continue;
        }
        commands.entity(entity).insert(TreeNodeBound);
        match name.as_str() {
            "wheel_pickup" => {
                commands.entity(entity).insert(OldWorks::Wheel);
            }
            "tool_lid" => {
                commands
                    .entity(entity)
                    .insert(OldWorks::ToolLid(transform.rotation));
            }
            "tonic_lid" => {
                commands
                    .entity(entity)
                    .insert(OldWorks::TonicLid(transform.rotation));
            }
            name if name.starts_with("canopy_")
                || name.starts_with("awning_")
                || name == "trunk_main" =>
            {
                let kind = if name.starts_with("awning_") {
                    SwayKind::Awning
                } else if name == "trunk_main" {
                    SwayKind::Trunk
                } else {
                    SwayKind::Crown {
                        shake: name == "canopy_main",
                    }
                };
                commands.entity(entity).insert(TreeSway {
                    rest: *transform,
                    kind,
                    phase: transform.translation.x * 0.71 + transform.translation.z * 1.13,
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
    let now = time.elapsed_secs();
    for (tree, mut transform) in &mut trees {
        let wave = (now * 1.63 + tree.phase).sin() + 0.36 * (now * 2.71 + tree.phase * 1.7).sin();
        let (x, z) = match tree.kind {
            SwayKind::Crown { shake: responds } => (
                wave * 0.008,
                wave * 0.022 + if responds { shake * 0.065 } else { 0. },
            ),
            SwayKind::Trunk => (0., shake * 0.014),
            SwayKind::Awning => (wave * 0.027, 0.),
        };
        transform.rotation =
            tree.rest.rotation * Quat::from_rotation_x(x) * Quat::from_rotation_z(z);
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
    roots: Res<Regions>,
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
                ChildOf(roots.town),
                SceneRoot(props.plum.clone()),
                Transform::from_translation(portion_position(index, count) + Vec3::Y * 0.30),
                Portion { index, age: 0.0 },
                StageRoot::Table,
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
