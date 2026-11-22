//! Area-local light sources, falling water and the repaired fountain. No simulation lives here.
use crate::{
    Frame,
    delve::Quest,
    game::Outing,
    lens::{LensSet, LensState},
    map::{self, Area},
    stage::Regions,
};
use bevy::{
    asset::RenderAssetUsages,
    core_pipeline::{Skybox, bloom::Bloom},
    pbr::{
        CascadeShadowConfig, DirectionalLightShadowMap, ExtendedMaterial, MaterialExtension,
        NotShadowCaster, PointLightShadowMap,
    },
    prelude::*,
    render::{
        camera::Exposure,
        mesh::Indices,
        render_resource::{AsBindGroup, Buffer, PrimitiveTopology, ShaderRef},
    },
    transform::TransformSystem,
};
use serde::Deserialize;
use std::f32::consts::TAU;

pub struct SpringPlugin;
impl Plugin for SpringPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins((
            MaterialPlugin::<Current>::default(),
            MaterialPlugin::<AirInk>::default(),
        ))
        .insert_resource(DirectionalLightShadowMap { size: 2048 })
        .insert_resource(PointLightShadowMap { size: 512 })
        .add_systems(Startup, build)
        .add_systems(
            Update,
            (show_spring, breathe, release_steam).in_set(Frame::Draw),
        )
        .add_systems(
            PostUpdate,
            carry_lantern
                .after(LensSet)
                .before(TransformSystem::TransformPropagate),
        )
        .add_systems(
            PostUpdate,
            (meter, face_air)
                .after(LensSet)
                .before(TransformSystem::TransformPropagate),
        );
    }
}
#[derive(Asset, AsBindGroup, TypePath, Debug, Clone)]
struct WaterInk {
    #[uniform(100)]
    mode: Vec4,
    #[uniform(100)]
    lamps: [Vec4; 9],
    #[storage(102, read_only, buffer, visibility(fragment))]
    window: Buffer,
    #[texture(103)]
    contour: Handle<Image>,
    #[texture(104)]
    banks: Handle<Image>,
    #[texture(105)]
    #[sampler(106)]
    shelter: Handle<Image>,
}
impl MaterialExtension for WaterInk {
    fn fragment_shader() -> ShaderRef {
        "inks/spring_water.wgsl".into()
    }
}
type Current = ExtendedMaterial<StandardMaterial, WaterInk>;
#[derive(Asset, AsBindGroup, TypePath, Debug, Clone)]
struct AirInk {
    #[uniform(0)]
    settings: Vec4,
    #[storage(102, read_only, buffer, visibility(fragment))]
    window: Buffer,
    #[texture(103)]
    contour: Handle<Image>,
}
impl Material for AirInk {
    fn fragment_shader() -> ShaderRef {
        "inks/lantern_fx.wgsl".into()
    }
    fn alpha_mode(&self) -> AlphaMode {
        AlphaMode::Blend
    }
}
#[derive(Component)]
struct Sun;
#[derive(Component)]
pub(crate) struct Refilled;
#[derive(Component)]
struct Flicker {
    phase: f32,
    power: f32,
    area: Area,
}
#[derive(Component)]
struct AirCard;
#[derive(Component)]
struct Vent;
#[derive(Component)]
struct BeltLantern(bool);
#[derive(Deserialize, Debug, Clone, Copy, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
enum LampKind {
    Window,
    Sconce,
    Brazier,
    Candle,
    Crystal,
    Leak,
    Smoke,
}
#[derive(Deserialize)]
struct Lamp {
    pos: [f32; 3],
    kind: LampKind,
}

fn mesh(pos: Vec<[f32; 3]>, norm: Vec<[f32; 3]>, uv: Vec<[f32; 2]>, indices: Vec<u32>) -> Mesh {
    Mesh::new(
        PrimitiveTopology::TriangleList,
        RenderAssetUsages::RENDER_WORLD,
    )
    .with_inserted_attribute(Mesh::ATTRIBUTE_POSITION, pos)
    .with_inserted_attribute(Mesh::ATTRIBUTE_NORMAL, norm)
    .with_inserted_attribute(Mesh::ATTRIBUTE_UV_0, uv)
    .with_inserted_indices(Indices::U32(indices))
}
/// A shaft touches its high opening and its patch of floor in both camera views.
fn shaft(top: Vec3, toe: Vec3, width: f32) -> Mesh {
    let p = Vec3::X * width;
    let corners = [top - p * 0.22, top + p * 0.22, toe + p, toe - p];
    mesh(
        corners.map(|v| v.to_array()).to_vec(),
        vec![[0., 1., 0.]; 4],
        vec![[0., 0.], [1., 0.], [1., 1.], [0., 1.]],
        vec![0, 2, 1, 0, 3, 2, 0, 1, 2, 0, 2, 3],
    )
}
fn waterways() -> Mesh {
    let (mut pos, mut norm, mut uv, mut idx) = (Vec::new(), Vec::new(), Vec::new(), Vec::new());
    for z in map::MIN.y..=map::MAX.y {
        for x in map::MIN.x..=map::MAX.x {
            if map::glyph(Area::Cistern, IVec2::new(x, z)) != b'~' {
                continue;
            }
            let n = pos.len() as u32;
            for (dx, dz) in [(-0.5, -0.5), (-0.5, 0.5), (0.5, 0.5), (0.5, -0.5)] {
                pos.push([x as f32 + dx, -0.22, z as f32 + dz]);
                norm.push([0., 1., 0.]);
                uv.push([dx + 0.5, dz + 0.5]);
            }
            idx.extend_from_slice(&[n, n + 1, n + 2, n, n + 2, n + 3]);
        }
    }
    mesh(pos, norm, uv, idx)
}
fn jets() -> Mesh {
    let (mut pos, mut norm, mut uv, mut idx) = (Vec::new(), Vec::new(), Vec::new(), Vec::new());
    let center = Vec3::new(map::FOUNTAIN.x as f32, 0., map::FOUNTAIN.y as f32);
    const STEPS: u32 = 18;
    const SIDES: u32 = 7;
    for dir in [Vec3::Z, Vec3::X, -Vec3::Z, -Vec3::X] {
        let start = pos.len() as u32;
        let side = dir.cross(Vec3::Y);
        for i in 0..=STEPS {
            let t = i as f32 / STEPS as f32;
            let c = center + dir * (0.46 + 0.39 * t) + Vec3::Y * (1.285 + 0.18 * t - 0.965 * t * t);
            let tangent = (dir * 0.39 + Vec3::Y * (0.18 - 1.93 * t)).normalize();
            let up = side.cross(tangent);
            for j in 0..=SIDES {
                let a = TAU * j as f32 / SIDES as f32;
                let n = side * a.cos() + up * a.sin();
                pos.push((c + n * (0.039 - 0.014 * t)).to_array());
                norm.push(n.to_array());
                uv.push([j as f32 / SIDES as f32, t]);
                if i < STEPS && j < SIDES {
                    let a = start + i * (SIDES + 1) + j;
                    let b = a + SIDES + 1;
                    idx.extend_from_slice(&[a, b, a + 1, a + 1, b, b + 1]);
                }
            }
        }
    }
    mesh(pos, norm, uv, idx)
}
#[derive(Resource)]
struct SpringProbes {
    town: EnvironmentMapLight,
    cistern: EnvironmentMapLight,
    sky: Handle<Image>,
}

fn build(
    mut c: Commands,
    roots: Res<Regions>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut waters: ResMut<Assets<Current>>,
    mut airs: ResMut<Assets<AirInk>>,
    server: Res<AssetServer>,
    sight: Res<crate::sight::SightWindow>,
    banks: Res<crate::stage::SiteEdges>,
    shelter: Res<crate::stage::SkyShelter>,
) {
    let probe = |diffuse, specular, intensity| EnvironmentMapLight {
        diffuse_map: server.load(diffuse),
        specular_map: server.load(specular),
        intensity,
        ..default()
    };
    c.insert_resource(SpringProbes {
        town: probe(
            "inks/square-skyfill.dds",
            "inks/square-reflections.dds",
            1200.,
        ),
        cistern: probe(
            "inks/cistern-skyfill.dds",
            "inks/cistern-reflections.dds",
            225.,
        ),
        sky: server.load("inks/morning-sky.png"),
    });
    let mut air = |settings| {
        airs.add(AirInk {
            settings,
            window: sight.buffer.clone(),
            contour: sight.contour.clone(),
        })
    };
    c.spawn((
        Name::new("Sun above the copper roofs"),
        Sun,
        DirectionalLight {
            illuminance: 30_000.,
            color: Color::srgb(1., 0.86, 0.67),
            shadows_enabled: true,
            shadow_depth_bias: 0.008,
            shadow_normal_bias: 0.55,
            ..default()
        },
        CascadeShadowConfig {
            bounds: vec![365., 410.],
            minimum_distance: 321.,
            overlap_proportion: 0.06,
        },
        Transform::from_xyz(-12., 20., 10.).looking_at(Vec3::ZERO, Vec3::Y),
    ));
    let notes = [
        (Area::Town, include_str!("../assets/lanes/lights.json")),
        (Area::Cistern, include_str!("../assets/cistern/lights.json")),
    ]
    .map(|(a, s)| {
        (
            a,
            serde_json::from_str::<Vec<Lamp>>(s).expect("authored lamp manifest"),
        )
    });
    let mut reflections = [Vec4::ZERO; 9];
    for (slot, l) in reflections.iter_mut().zip(&notes[1].1) {
        *slot = Vec3::from_array(l.pos).extend(
            if matches!(l.kind, LampKind::Leak | LampKind::Crystal) {
                -1.
            } else {
                1.
            },
        );
    }
    let mut material = |mode: f32| {
        waters.add(ExtendedMaterial {
            base: StandardMaterial {
                base_color: Color::srgb(0.14, 0.49, 0.52),
                metallic: 0.27,
                perceptual_roughness: 0.24,
                cull_mode: None,
                ..default()
            },
            extension: WaterInk {
                mode: Vec4::new(mode, 0., 0., 0.),
                lamps: reflections,
                window: sight.buffer.clone(),
                contour: sight.contour.clone(),
                banks: banks.0.clone(),
                shelter: shelter
                    .at(if mode == 0. {
                        Area::Cistern
                    } else {
                        Area::Town
                    })
                    .clone(),
            },
        })
    };
    c.spawn((
        Name::new("Cistern channels"),
        Mesh3d(meshes.add(waterways())),
        MeshMaterial3d(material(0.)),
        NotShadowCaster,
        ChildOf(roots.cistern),
    ));
    let flowing = c
        .spawn((
            Name::new("Fountain in service"),
            Refilled,
            Transform::default(),
            Visibility::Hidden,
            ChildOf(roots.town),
        ))
        .id();
    let spring_surface = material(1.);
    let upward = Quat::from_rotation_x(-std::f32::consts::FRAC_PI_2);
    let vessels: [[f32; 4]; 3] =
        serde_json::from_str(include_str!("../assets/lanes/water-vessels.json"))
            .expect("authored fountain vessels");
    let meniscus = meshes.add(Circle::new(1.).mesh().resolution(16));
    for [x, z, r, h] in vessels {
        c.spawn((
            Mesh3d(meniscus.clone()),
            MeshMaterial3d(spring_surface.clone()),
            Transform::from_xyz(x, h - 0.044, z)
                .with_rotation(upward)
                .with_scale(Vec3::splat(r * 0.86)),
            NotShadowCaster,
            ChildOf(flowing),
        ));
    }
    c.spawn((
        Mesh3d(meshes.add(Circle::new(1.035).mesh().resolution(32))),
        MeshMaterial3d(spring_surface),
        Transform::from_xyz(4., 0.50, 2.).with_rotation(upward),
        NotShadowCaster,
        ChildOf(flowing),
    ));
    c.spawn((
        Mesh3d(meshes.add(jets())),
        MeshMaterial3d(material(2.)),
        NotShadowCaster,
        ChildOf(flowing),
    ));
    let quad = meshes.add(Rectangle::new(1., 1.));
    for (area, head, foot, width, power) in [
        (Area::Town, [-6.1, 3.8, 1.8], [-2.6, 0.06, 3.4], 1.12, 0.095),
        (Area::Town, [3.0, 3.9, -6.8], [5.6, 0.05, -4.5], 0.85, 0.075),
        (
            Area::Cistern,
            [-1.4, 5.8, 6.4],
            [0.3, 0.10, 3.4],
            1.30,
            0.16,
        ),
        (
            Area::Cistern,
            [-7.5, 3.2, -4.9],
            [-6.2, 0.02, -3.2],
            0.62,
            0.10,
        ),
    ] {
        c.spawn((
            Name::new("Light through an opening"),
            Mesh3d(meshes.add(shaft(head.into(), foot.into(), width))),
            MeshMaterial3d(air(Vec4::new(
                5.,
                head[0],
                power,
                if area == Area::Town { 1. } else { 0. },
            ))),
            NotShadowCaster,
            ChildOf(roots.root(area)),
        ));
    }
    c.spawn((
        Name::new("Traveler's underground lamp"),
        BeltLantern(true),
        PointLight {
            color: Color::srgb(1., 0.82, 0.56),
            intensity: 11_000.,
            range: 3.8,
            radius: 0.22,
            ..default()
        },
        ChildOf(roots.cistern),
    ));
    c.spawn((
        BeltLantern(false),
        Mesh3d(quad.clone()),
        MeshMaterial3d(air(Vec4::new(4., 0., 1., 0.))),
        Transform::default(),
        NotShadowCaster,
        ChildOf(roots.cistern),
    ));
    // The open stair is the one place below ground where daylight can reach a traveler.
    c.spawn((
        Name::new("Skylight over the return stair"),
        SpotLight {
            color: Color::srgb(0.74, 0.86, 1.),
            intensity: 26_000.,
            range: 11.,
            radius: 0.3,
            inner_angle: 0.21,
            outer_angle: 0.52,
            ..default()
        },
        Transform::from_xyz(-1.4, 5.8, 6.4).looking_at(Vec3::new(0., 0., 4.3), Vec3::Y),
        ChildOf(roots.cistern),
    ));
    for z in map::MIN.y..=map::MAX.y {
        for x in map::MIN.x..=map::MAX.x {
            let p = IVec2::new(x, z);
            if map::glyph(Area::Cistern, p) != b'P' {
                continue;
            }
            c.spawn((
                Vent,
                AirCard,
                Mesh3d(quad.clone()),
                MeshMaterial3d(air(Vec4::new(1., x as f32, 1., 0.))),
                Transform::from_xyz(x as f32, 0.7, z as f32),
                Visibility::Hidden,
                NotShadowCaster,
                ChildOf(roots.cistern),
            ));
        }
    }
    for (area, data) in notes {
        let mut casting = 0;
        let mut pools: Vec<Vec3> = Vec::with_capacity(12);
        for (i, l) in data.into_iter().enumerate() {
            let p = Vec3::from_array(l.pos);
            let phase = i as f32 * 1.73;
            let town = area == Area::Town;
            let (raw, range, hue): (f32, f32, Color) = match l.kind {
                LampKind::Window => (9800., 3.4, Color::srgb(1., 0.57, 0.25)),
                LampKind::Sconce => (4100., 5.5, Color::srgb(1., 0.66, 0.32)),
                LampKind::Brazier => (7600., 7.1, Color::srgb(1., 0.625, 0.285)),
                LampKind::Candle => (1850., 3.7, Color::srgb(1., 0.77, 0.51)),
                LampKind::Crystal => (4200., 5.5, Color::srgb(0.30, 0.79, 0.94)),
                LampKind::Leak => (2550., 4.4, Color::srgb(0.29, 0.76, 1.)),
                LampKind::Smoke => (0., 0., Color::WHITE),
            };
            let in_square = p.x.abs() < 9.2 && (-7.7..=5.0).contains(&p.z);
            let key_lamp =
                !town || (in_square && !pools.iter().any(|&q| q.distance_squared(p) < 4.0));
            if raw > 0. && key_lamp {
                if town {
                    pools.push(p);
                }
                let power = if town { raw * 2.8 } else { raw * 9.5 };
                let cast = !town && l.kind == LampKind::Brazier && casting < 2;
                if cast {
                    casting += 1;
                }
                c.spawn((
                    Name::new(format!("{:?} {:?} {i}", area, l.kind)),
                    PointLight {
                        color: hue,
                        intensity: power,
                        range: if town { range.min(4.2_f32) } else { range },
                        radius: 0.11,
                        shadows_enabled: cast,
                        shadow_normal_bias: 0.40,
                        ..default()
                    },
                    Transform::from_translation(p + Vec3::Y * 0.075),
                    Flicker { phase, power, area },
                    ChildOf(roots.root(area)),
                ));
            }
            let (kind, size, offset, strength) = match l.kind {
                LampKind::Brazier => (0., Vec2::new(0.54, 0.77), 0.23, 1.7),
                LampKind::Candle => (0., Vec2::new(0.14, 0.25), 0.07, 1.0),
                LampKind::Sconce => (0., Vec2::new(0.20, 0.36), 0.07, 1.1),
                LampKind::Leak => (
                    3.,
                    Vec2::new(0.38, (p.y + 0.22).max(0.3)),
                    -(p.y + 0.22) * 0.45,
                    0.8,
                ),
                LampKind::Smoke => (1., Vec2::new(1.2, 2.4), 1.16, 1.),
                _ => continue,
            };
            c.spawn((
                AirCard,
                Mesh3d(quad.clone()),
                MeshMaterial3d(air(Vec4::new(kind, phase, strength, 0.))),
                Transform::from_translation(p + Vec3::Y * offset).with_scale(size.extend(1.)),
                NotShadowCaster,
                ChildOf(roots.root(area)),
            ));
        }
    }
    for p in [Vec3::new(-7.69, 5.75, -0.12), Vec3::new(-7.45, 6.00, -5.02)] {
        c.spawn((
            AirCard,
            Mesh3d(quad.clone()),
            MeshMaterial3d(air(Vec4::new(1., p.x, 1., 0.))),
            Transform::from_translation(p).with_scale(Vec3::new(1.25, 2.1, 1.)),
            NotShadowCaster,
            ChildOf(roots.town),
        ));
    }
    for (p, a) in [
        (Vec3::new(-3.25, 1.6, -2.2), Area::Town),
        (Vec3::new(6.9, 1.7, -3.0), Area::Cistern),
    ] {
        c.spawn((
            AirCard,
            Mesh3d(quad.clone()),
            MeshMaterial3d(air(Vec4::new(
                2.,
                p.x,
                1.,
                if a == Area::Cistern { 1. } else { 0. },
            ))),
            Transform::from_translation(p).with_scale(Vec3::new(2., 2.2, 1.)),
            NotShadowCaster,
            ChildOf(roots.root(a)),
        ));
    }
    for (p, size, kind) in [
        (Vec3::new(-7.9, 1.55, 4.), Vec2::new(4., 3.1), 0.),
        (Vec3::new(6.1, 1.6, -3.3), Vec2::new(3.4, 3.2), 1.),
    ] {
        c.spawn((
            AirCard,
            Mesh3d(quad.clone()),
            MeshMaterial3d(air(Vec4::new(6., p.x, 1., kind))),
            Transform::from_translation(p).with_scale(size.extend(1.)),
            NotShadowCaster,
            ChildOf(roots.town),
        ));
    }
    for (p, size) in [
        (Vec3::new(-8.85, 0.31, -1.6), Vec2::new(3.3, 0.92)),
        (Vec3::new(5.6, 0.35, -5.8), Vec2::new(5.4, 1.18)),
    ] {
        c.spawn((
            AirCard,
            Mesh3d(quad.clone()),
            MeshMaterial3d(air(Vec4::new(7., p.x, 1., 0.))),
            Transform::from_translation(p).with_scale(size.extend(1.)),
            NotShadowCaster,
            ChildOf(roots.cistern),
        ));
    }
}
fn show_spring(g: Res<Outing>, mut fountain: Single<&mut Visibility, With<Refilled>>) {
    fountain.set_if_neq(if g.expedition.quest == Quest::Restored {
        Visibility::Inherited
    } else {
        Visibility::Hidden
    });
}
fn breathe(
    t: Res<Time>,
    g: Res<Outing>,
    mut lights: Query<(&Flicker, &mut PointLight)>,
    mut last: Local<u32>,
) {
    let tick = (t.elapsed_secs() * 14.) as u32;
    if *last == tick {
        return;
    }
    *last = tick;
    let t = tick as f32 / 14.;
    for (l, mut light) in &mut lights {
        if l.area == g.area {
            light.intensity = l.power
                * (1.
                    + 0.025 * (t * 4.3 + l.phase).sin()
                    + 0.015 * (t * 8.1 + l.phase * 0.47).sin());
        }
    }
}
fn carry_lantern(
    camera: Single<&Transform, (With<Camera3d>, Without<BeltLantern>)>,
    actor: Single<&Transform, (With<crate::figures::PlayerVisual>, Without<BeltLantern>)>,
    mut pieces: Query<(&BeltLantern, &mut Transform)>,
    g: Res<Outing>,
) {
    if g.area != Area::Cistern {
        return;
    }
    let side = if g.facing.x < 0 { -1. } else { 1. };
    let belt = actor.translation + camera.rotation * Vec3::new(side * 0.25, 0.54, 0.06);
    for (piece, mut tr) in &mut pieces {
        if piece.0 {
            tr.translation = belt + *camera.back() * 0.60 + Vec3::Y * 0.28;
        } else {
            *tr = Transform::from_translation(belt)
                .with_rotation(camera.rotation)
                .with_scale(Vec3::new(0.19, 0.27, 1.));
        }
    }
}
fn release_steam(
    t: Res<Time>,
    g: Res<Outing>,
    mut last: Local<u32>,
    mut started: Local<Option<f32>>,
    mut vents: Query<(&mut Visibility, &mut Transform), With<Vent>>,
) {
    if g.area != Area::Cistern || g.expedition.beat == 0 {
        *last = 0;
        *started = None;
    } else if *last != g.expedition.beat {
        *last = g.expedition.beat;
        if *last % 4 == 0 {
            *started = Some(t.elapsed_secs());
        }
    }
    let since = started.map(|v| t.elapsed_secs() - v).unwrap_or(99.);
    for (mut vis, mut tr) in &mut vents {
        if since < 0.63 {
            vis.set_if_neq(Visibility::Inherited);
            let u = since / 0.63;
            tr.translation.y = 0.12 + 0.62 * u;
            tr.scale = Vec3::new((0.35 + u) * 0.85, (1. - u) * 1.6, 1.);
        } else {
            vis.set_if_neq(Visibility::Hidden);
        }
    }
}
fn face_air(
    camera: Single<&Transform, With<Camera3d>>,
    mut cards: Query<&mut Transform, (With<AirCard>, Without<Camera3d>)>,
) {
    // Keep smoke attached to its chimney, and flames rising vertically rather than lying on the paving.
    let pitch = (camera.forward().y.abs().clamp(0., 1.).asin() * 0.72).min(0.78);
    let rot = Quat::from_rotation_x(-pitch);
    for mut tr in &mut cards {
        if tr.rotation != rot {
            tr.rotation = rot;
        }
    }
}
#[allow(clippy::type_complexity)]
fn meter(
    g: Res<Outing>,
    lens: Res<LensState>,
    mut ambient: ResMut<AmbientLight>,
    mut clear: ResMut<ClearColor>,
    probes: Res<SpringProbes>,
    camera: Single<
        (
            &mut Exposure,
            &mut DistanceFog,
            &mut Bloom,
            Option<&mut Skybox>,
            &mut bevy::pbr::ClusterConfig,
            &mut EnvironmentMapLight,
            &mut bevy::render::view::ColorGrading,
        ),
        With<Camera3d>,
    >,
    sun: Single<(&mut DirectionalLight, &mut CascadeShadowConfig), With<Sun>>,
    mut previous: Local<Option<(Area, i32)>>,
) {
    let town = g.area == Area::Town;
    let d = (lens.distance * 2.).round() as i32;
    let (mut exp, mut fog, mut bloom, sky, mut clusters, mut environment, mut grade) =
        camera.into_inner();
    if let Some(mut s) = sky {
        let map = if town {
            &probes.sky
        } else {
            &probes.cistern.diffuse_map
        };
        if s.image != *map {
            s.image = map.clone();
        }
        let b = if town { 5100. } else { 29. };
        if s.brightness != b {
            s.brightness = b;
        }
    }
    if *previous == Some((g.area, d)) {
        return;
    }
    let switched = previous.is_none_or(|(a, _)| a != g.area);
    *previous = Some((g.area, d));
    let distance = d as f32 * 0.5;
    let (mut sun, mut cascades) = sun.into_inner();
    // With the near-parallel overview the camera is far away. Allocate shadow map
    // resolution to occupied ground, not the empty hundreds of metres in front of it.
    if distance > 60. {
        cascades.minimum_distance = distance - 23.;
        cascades.bounds.truncate(1);
        cascades.bounds[0] = distance + 24.;
    } else {
        cascades.minimum_distance = (distance - 23.).max(0.08);
        cascades.bounds.resize(2, 0.);
        cascades.bounds[0] = (distance + 3.).max(12.);
        cascades.bounds[1] = distance + 46.;
    }
    cascades.overlap_proportion = (2. / cascades.bounds[0]).min(0.12);
    *clusters = bevy::pbr::ClusterConfig::FixedZ {
        total: 576,
        z_slices: 4,
        z_config: bevy::pbr::ClusterZConfig {
            first_slice_depth: (distance - 10.).max(1.5),
            far_z_mode: bevy::pbr::ClusterFarZMode::Constant(distance + 28.),
        },
        dynamic_resizing: true,
    };
    fog.falloff = FogFalloff::Linear {
        start: (distance + if town { 7. } else { 5. }).max(12.),
        end: distance + if town { 65. } else { 32. },
    };
    if switched {
        *environment = if town {
            probes.town.clone()
        } else {
            probes.cistern.clone()
        };
        grade.shadows.lift = if town { 0.003 } else { 0.001 };
        grade.midtones.saturation = if town { 1.04 } else { 1.10 };
        grade.global.post_saturation = 1.045;
        sun.illuminance = if town { 37_500. } else { 475. };
        sun.shadows_enabled = town;
        sun.color = if town {
            Color::srgb(1., 0.935, 0.805)
        } else {
            Color::srgb(0.61, 0.76, 0.98)
        };
        ambient.color = if town {
            Color::srgb(0.72, 0.86, 0.99)
        } else {
            Color::srgb(0.60, 0.78, 0.98)
        };
        ambient.brightness = if town { 4100. } else { 230. };
        exp.ev100 = if town { 11.62 } else { 8.70 };
        bloom.intensity = if town { 0.07 } else { 0.14 };
        fog.color = if town {
            Color::srgb(0.56, 0.65, 0.73)
        } else {
            Color::srgb(0.054, 0.066, 0.12)
        };
        fog.directional_light_color = Color::NONE;
        clear.0 = fog.color;
    }
}
