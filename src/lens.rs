//! Fixed north-facing perspective camera: reversible physical dolly and crane.
use bevy::{
    core_pipeline::{
        dof::{DepthOfField, DepthOfFieldMode},
        tonemapping::Tonemapping,
    },
    input::mouse::{AccumulatedMouseScroll, MouseScrollUnit},
    pbr::ShadowFilteringMethod,
    prelude::*,
    render::camera::Exposure,
    transform::TransformSystem,
    window::PrimaryWindow,
};

use crate::{
    figures::PlayerVisual,
    game::{Outing, ground},
};

pub struct LensPlugin;

/// Actual smoothed lens values, in metres and degrees below horizontal.
/// zoom=0 is eye level, .72 is the default exploration view, 1 is overview.
#[derive(Resource, Reflect)]
#[reflect(Resource)]
pub struct LensState {
    pub zoom: f32,
    pub pitch_deg: f32,
    pub distance: f32,
    goal_zoom: f32,
}

impl Default for LensState {
    fn default() -> Self {
        Self {
            zoom: DEFAULT_ZOOM,
            pitch_deg: 56.0,
            distance: 27.0,
            goal_zoom: DEFAULT_ZOOM,
        }
    }
}

/// Billboard orientation must run after this set and before transform propagation.
#[derive(SystemSet, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct LensSet;

impl Plugin for LensPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<LensState>()
            .register_type::<LensState>()
            .configure_sets(
                PostUpdate,
                LensSet.before(TransformSystem::TransformPropagate),
            )
            .add_systems(Startup, install_camera)
            .add_systems(PostUpdate, move_camera.in_set(LensSet));
    }
}

const DEFAULT_ZOOM: f32 = 0.72;
const EYE_HEIGHT: f32 = 1.34;
const COMPOSITION: Vec3 = Vec3::new(0.0, 0.60, -0.40);

fn smooth(t: f32) -> f32 {
    t * t * (3.0 - 2.0 * t)
}

fn geometry(zoom: f32) -> (f32, f32, f32) {
    if zoom <= DEFAULT_ZOOM {
        let t = smooth(zoom / DEFAULT_ZOOM);
        (56.0 * t, 5.2_f32.lerp(27.0, t), t)
    } else {
        let t = smooth((zoom - DEFAULT_ZOOM) / (1.0 - DEFAULT_ZOOM));
        (56.0_f32.lerp(62.0, t), 27.0_f32.lerp(37.0, t), 1.0)
    }
}

fn target(foot: Vec3, wide: f32) -> Vec3 {
    // The inn and orchard share the default frame; approaching hands tracking
    // completely to the animated foot, then raises the aim to the traveler's eye.
    let overview = COMPOSITION.lerp(foot + Vec3::new(0.0, 0.55, -1.0), 0.28);
    (foot + Vec3::Y * EYE_HEIGHT).lerp(overview, wide)
}

fn viewpoint(target: Vec3, pitch_deg: f32, distance: f32) -> Transform {
    let (rise, run) = pitch_deg.to_radians().sin_cos();
    Transform::from_translation(target + Vec3::new(0.0, rise, run) * distance)
        .looking_at(target, Vec3::Y)
}

fn aspect_distance(distance: f32, window: &Window) -> f32 {
    let aspect = window.width() / window.height().max(1.0);
    distance * (1.40 / aspect.max(0.45)).max(1.0)
}

fn install_camera(
    mut commands: Commands,
    outing: Res<Outing>,
    mut state: ResMut<LensState>,
    window: Single<&Window, With<PrimaryWindow>>,
) {
    let (pitch, distance, wide) = geometry(state.zoom);
    state.pitch_deg = pitch;
    state.distance = aspect_distance(distance, &window);
    let foot = ground(outing.player);
    let pose = viewpoint(target(foot, wide), pitch, state.distance);
    let focal_distance = (foot - pose.translation).dot(*pose.forward()).max(0.2);
    // StagePlugin owns the lighting, bloom and distance fog on this camera.
    commands.spawn((
        Name::new("Fixed north-facing dolly/crane"),
        Camera3d::default(),
        Camera {
            hdr: true,
            ..default()
        },
        IsDefaultUiCamera,
        Projection::Perspective(PerspectiveProjection {
            fov: 34.0_f32.to_radians(),
            near: 0.08,
            far: 220.0,
            ..default()
        }),
        // Meter the painted mids, including the cloth and the shaded inn porch.
        Exposure { ev100: 12.0 },
        Tonemapping::AcesFitted,
        Msaa::Off,
        ShadowFilteringMethod::Gaussian,
        DepthOfField {
            mode: DepthOfFieldMode::Gaussian,
            focal_distance,
            sensor_height: 0.072,
            aperture_f_stops: 4.0,
            max_circle_of_confusion_diameter: 4.0,
            max_depth: 180.0,
        },
        pose,
    ));
}

fn move_camera(
    keys: Res<ButtonInput<KeyCode>>,
    scroll: Res<AccumulatedMouseScroll>,
    time: Res<Time>,
    outing: Res<Outing>,
    mut state: ResMut<LensState>,
    window: Single<&Window, With<PrimaryWindow>>,
    player: Query<&Transform, (With<PlayerVisual>, Without<Camera3d>)>,
    camera: Single<(&mut Transform, &mut DepthOfField), (With<Camera3d>, Without<PlayerVisual>)>,
) {
    let lines = match scroll.unit {
        MouseScrollUnit::Line => scroll.delta.y,
        MouseScrollUnit::Pixel => scroll.delta.y / 40.0,
    };
    let nearer = keys.pressed(KeyCode::Equal) || keys.pressed(KeyCode::NumpadAdd);
    let farther = keys.pressed(KeyCode::Minus) || keys.pressed(KeyCode::NumpadSubtract);
    let key_delta = (farther as u8 as f32 - nearer as u8 as f32) * time.delta_secs() * 0.45;
    state.goal_zoom = (state.goal_zoom - lines * 0.07 + key_delta).clamp(0.0, 1.0);
    if keys.just_pressed(KeyCode::Home) {
        state.goal_zoom = DEFAULT_ZOOM;
    }
    state.zoom = state
        .zoom
        .lerp(state.goal_zoom, 1.0 - (-8.0 * time.delta_secs()).exp());
    if (state.zoom - state.goal_zoom).abs() < 0.0002 {
        state.zoom = state.goal_zoom;
    }
    let (pitch, distance, wide) = geometry(state.zoom);
    state.pitch_deg = pitch;
    state.distance = aspect_distance(distance, &window);
    // Read the rendered interpolation, not the destination tile: this keeps the
    // traveler in the same place in frame throughout an action at close range.
    let foot = player
        .single()
        .map_or_else(|_| ground(outing.player), |pose| pose.translation);
    let (mut pose, mut dof) = camera.into_inner();
    *pose = viewpoint(target(foot, wide), state.pitch_deg, state.distance);
    // Screen-aligned painted bodies share their foot's view-space depth.
    dof.focal_distance = (foot - pose.translation)
        .dot(*pose.forward())
        .clamp(0.2, 180.0);
}
