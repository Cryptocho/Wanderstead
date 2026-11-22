//! North-facing, near-orthographic exploration with a continuous eye-level dolly.
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
    daylight::{Optics, SQUARE},
    figures::PlayerVisual,
    map::Area,
    scene::{SceneState, ground},
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
            pitch_deg: 54.0,
            distance: 360.0,
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
const COMPOSITION: Vec3 = Vec3::new(0.0, 0.66, -2.0);

fn smooth(t: f32) -> f32 {
    t * t * (3.0 - 2.0 * t)
}

// Varying distance and FOV together gives a continuous orthographic-to-perspective
// transition without a projection-type threshold.
fn geometry(zoom: f32) -> (f32, f32, f32, f32) {
    if zoom <= DEFAULT_ZOOM {
        let t = smooth(zoom / DEFAULT_ZOOM);
        (
            54.0 * t,
            4.8 * (360.0_f32 / 4.8).powf(t),
            3.8 * (13.6_f32 / 3.8).powf(t),
            t,
        )
    } else {
        let t = smooth((zoom - DEFAULT_ZOOM) / (1.0 - DEFAULT_ZOOM));
        (
            54.0_f32.lerp(61.0, t),
            360.0_f32.lerp(500.0, t),
            13.6_f32.lerp(19.0, t),
            1.0,
        )
    }
}

fn target(foot: Vec3, wide: f32, area: Area) -> Vec3 {
    let follow = if area == Area::Cistern { 0.67 } else { 0.30 };
    let overview = COMPOSITION.lerp(foot + Vec3::new(0.0, 0.30, -1.1), follow);
    (foot + Vec3::Y * EYE_HEIGHT).lerp(overview, wide)
}

fn viewpoint(target: Vec3, pitch_deg: f32, distance: f32) -> Transform {
    let (rise, run) = pitch_deg.to_radians().sin_cos();
    Transform::from_translation(target + Vec3::new(0.0, rise, run) * distance)
        .looking_at(target, Vec3::Y)
}

fn aspect_span(span: f32, window: &Window) -> f32 {
    let aspect = window.width() / window.height().max(1.0);
    span * (1.50 / aspect.max(0.45)).max(1.0)
}

/// Work at the actor's actual view depth through the whole dolly, not its distance
/// from the look-at target. Scale the blur limit in the same units as an art pixel.
fn focus(
    foot: Vec3,
    eye: &Transform,
    area: Area,
    wide: f32,
    height_px: u32,
    on: bool,
) -> DepthOfField {
    let (sensor, stops, limit) = if area == Area::Town {
        let f = &SQUARE.focus;
        (
            f.sensor_m,
            f.f_stops,
            f.limit_px_at_1440 * height_px as f32 / 1440.,
        )
    } else {
        ([0.045, 0.180], [3.8, 1.65], 9.0)
    };
    DepthOfField {
        mode: DepthOfFieldMode::Gaussian,
        focal_distance: (foot - eye.translation)
            .dot(*eye.forward())
            .clamp(0.2, 1000.),
        sensor_height: sensor[0].lerp(sensor[1], wide),
        aperture_f_stops: stops[0].lerp(stops[1], wide),
        max_circle_of_confusion_diameter: if on { limit } else { 0.0 },
        max_depth: 1000.0,
    }
}

fn install_camera(
    mut commands: Commands,
    outing: Res<SceneState>,
    mut state: ResMut<LensState>,
    window: Single<&Window, With<PrimaryWindow>>,
) {
    let (pitch, distance, span, wide) = geometry(state.zoom);
    state.pitch_deg = pitch;
    state.distance = distance;
    let foot = ground(outing.player);
    let pose = viewpoint(target(foot, wide, outing.area), pitch, distance);
    commands.spawn((
        Name::new("North-facing continuous orthographic dolly"),
        Camera3d::default(),
        Camera {
            hdr: true,
            ..default()
        },
        Projection::Perspective(PerspectiveProjection {
            fov: (aspect_span(span, &window) / (2.0 * distance)).atan() * 2.0,
            near: 0.06,
            far: 1200.0,
            ..default()
        }),
        Exposure {
            ev100: SQUARE.film.ev100,
        },
        Tonemapping::AcesFitted,
        Msaa::Off,
        // spring::meter installs the active area's values before the first render.
        (
            EnvironmentMapLight::default(),
            DistanceFog::default(),
            bevy::pbr::ClusterConfig::default(),
            SQUARE.film.bloom(),
        ),
        ShadowFilteringMethod::Gaussian,
        focus(
            foot,
            &pose,
            outing.area,
            wide,
            window.physical_height(),
            true,
        ),
        pose,
    ));
}

fn move_camera(
    keys: Res<ButtonInput<KeyCode>>,
    scroll: Res<AccumulatedMouseScroll>,
    time: Res<Time>,
    outing: Res<SceneState>,
    optics: Res<Optics>,
    mut state: ResMut<LensState>,
    window: Single<&Window, With<PrimaryWindow>>,
    player: Query<&Transform, (With<PlayerVisual>, Without<Camera3d>)>,
    mut previous_area: Local<Option<Area>>,
    camera: Single<
        (&mut Transform, &mut Projection, &mut DepthOfField),
        (With<Camera3d>, Without<PlayerVisual>),
    >,
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
    let (pitch, distance, span, wide) = geometry(state.zoom);
    state.pitch_deg = pitch;
    state.distance = distance;
    let changed_area = *previous_area != Some(outing.area);
    *previous_area = Some(outing.area);
    let foot = if changed_area {
        ground(outing.player)
    } else {
        player
            .single()
            .map_or_else(|_| ground(outing.player), |pose| pose.translation)
    };
    let (mut pose, mut projection, mut dof) = camera.into_inner();
    *pose = viewpoint(target(foot, wide, outing.area), pitch, distance);
    if let Projection::Perspective(p) = &mut *projection {
        p.fov = (aspect_span(span, &window) / (2.0 * distance)).atan() * 2.0;
    }
    *dof = focus(
        foot,
        &pose,
        outing.area,
        wide,
        window.physical_height(),
        optics.dof,
    );
}
