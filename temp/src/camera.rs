use std::{
    io::Read,
    path::PathBuf,
    time::{SystemTime, UNIX_EPOCH},
};

use bevy::{
    app::AppExit,
    core_pipeline::{
        bloom::{Bloom, BloomCompositeMode, BloomPrefilter},
        dof::{DepthOfField, DepthOfFieldMode},
        tonemapping::Tonemapping,
    },
    input::mouse::{AccumulatedMouseScroll, MouseScrollUnit},
    pbr::{
        DistanceFog, FogFalloff, ScreenSpaceAmbientOcclusion,
        ScreenSpaceAmbientOcclusionQualityLevel, ShadowFilteringMethod, VolumetricFog,
    },
    prelude::*,
    render::{
        camera::Exposure,
        view::screenshot::{Screenshot, ScreenshotCaptured, save_to_disk},
    },
    window::PrimaryWindow,
};

use crate::{
    party::{Expedition, PartyMovement},
    world::{self, PLAN},
};

pub struct LensPlugin;

impl Plugin for LensPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, install_camera).add_systems(
            Update,
            (control_camera.after(PartyMovement), photograph, prove_lens.after(control_camera)),
        );
    }
}

#[derive(Component)]
struct Lens {
    zoom: f32,
    goal_zoom: f32,
    target: Vec3,
    pitch_deg: f32,
    distance: f32,
    focus_gate: bool,
}

// zoom=0: horizontal eye-level portrait; .72: exploration; 1: overview.
// The focal length and heading never change: this is a physical dolly/crane.
const DEFAULT_ZOOM: f32 = 0.72;
const COMPOSITION: Vec3 = Vec3::new(0.0, 1.02, -2.05);
const EYE_HEIGHT: f32 = 1.78;

fn smooth(t: f32) -> f32 {
    t * t * (3.0 - 2.0 * t)
}

fn lens_geometry(zoom: f32) -> (f32, f32, f32) {
    if zoom <= DEFAULT_ZOOM {
        let t = smooth(zoom / DEFAULT_ZOOM);
        (56.0 * t, 8.0_f32.lerp(38.8, t), t)
    } else {
        let t = smooth((zoom - DEFAULT_ZOOM) / (1.0 - DEFAULT_ZOOM));
        (56.0_f32.lerp(62.0, t), 38.8_f32.lerp(48.0, t), 1.0)
    }
}

fn viewpoint(target: Vec3, pitch_deg: f32, distance: f32) -> Transform {
    let (rise, run) = pitch_deg.to_radians().sin_cos();
    let eye = target + distance * Vec3::new(0.0, rise, run);
    Transform::from_translation(eye).looking_at(target, Vec3::Y)
}

#[derive(Clone, Copy, PartialEq)]
enum ProofPhase {
    Default,
    SavingDefault,
    Near,
    SavingNear,
    Finished,
}

#[derive(Resource)]
struct LensProof {
    directory: PathBuf,
    stamp: u128,
    phase: ProofPhase,
    settled: f32,
}

fn install_camera(mut commands: Commands, expedition: Res<Expedition>) {
    let mut zoom = DEFAULT_ZOOM;
    let mut proof_directory = None;
    // Optional native validation: --lens-zoom=0 or --lens-proof=/path/to/captures.
    for arg in std::env::args().skip(1) {
        if let Some(value) = arg.strip_prefix("--lens-zoom=") {
            match value.parse::<f32>() {
                Ok(value) if value.is_finite() => zoom = value.clamp(0.0, 1.0),
                _ => warn!("Invalid --lens-zoom value: {value}"),
            }
        } else if let Some(value) = arg.strip_prefix("--lens-proof=") {
            proof_directory = Some(PathBuf::from(value));
        }
    }
    if let Some(directory) = proof_directory {
        zoom = DEFAULT_ZOOM;
        commands.insert_resource(LensProof {
            directory,
            stamp: timestamp(),
            phase: ProofPhase::Default,
            settled: 0.0,
        });
    }
    let (pitch_deg, distance, wide) = lens_geometry(zoom);
    let foot = world::ground(expedition.leader);
    let target = composed_target(foot, wide);
    commands
        .spawn((
            Name::new("Courtyard · fixed north-facing dolly/crane"),
            Camera3d::default(),
            Camera { hdr: true, ..default() },
            IsDefaultUiCamera,
            Projection::Perspective(PerspectiveProjection {
                fov: 31.0_f32.to_radians(),
                near: 0.08,
                far: 180.0,
                ..default()
            }),
            Exposure { ev100: 10.0 },
            Msaa::Off,
            Tonemapping::AcesFitted,
            ShadowFilteringMethod::Gaussian,
            ScreenSpaceAmbientOcclusion {
                quality_level: ScreenSpaceAmbientOcclusionQualityLevel::Medium,
                constant_object_thickness: 0.40,
            },
            Bloom {
                intensity: 0.15,
                low_frequency_boost: 0.35,
                prefilter: BloomPrefilter {
                    threshold: 1.20,
                    threshold_softness: 0.35,
                },
                composite_mode: BloomCompositeMode::Additive,
                ..Bloom::NATURAL
            },
            viewpoint(target, pitch_deg, distance),
            Lens {
                zoom,
                goal_zoom: zoom,
                target,
                pitch_deg,
                distance,
                focus_gate: false,
            },
        ))
        .insert((
            DepthOfField {
                mode: DepthOfFieldMode::Gaussian,
                focal_distance: distance,
                sensor_height: 0.024,
                aperture_f_stops: 16.0,
                max_circle_of_confusion_diameter: 1.5,
                max_depth: 150.0,
            },
            VolumetricFog {
                ambient_intensity: 0.0,
                step_count: 32,
                jitter: 0.0,
                ..default()
            },
            DistanceFog {
                color: Color::srgba(0.16, 0.28, 0.30, 0.68),
                directional_light_color: Color::NONE,
                falloff: FogFalloff::Linear { start: 60.0, end: 140.0 },
                ..default()
            },
        ));
}

fn composed_target(foot: Vec3, wide: f32) -> Vec3 {
    let overview = COMPOSITION.lerp(foot + Vec3::new(0.0, 0.75, -1.1), 0.28);
    (foot + Vec3::Y * EYE_HEIGHT).lerp(overview, wide)
}

fn control_camera(
    keys: Res<ButtonInput<KeyCode>>,
    scroll: Res<AccumulatedMouseScroll>,
    time: Res<Time>,
    expedition: Res<Expedition>,
    window: Single<&Window, With<PrimaryWindow>>,
    camera: Single<(&mut Lens, &mut Transform, &mut DepthOfField)>,
) {
    let (mut lens, mut pose, mut dof) = camera.into_inner();
    let lines = match scroll.unit {
        MouseScrollUnit::Line => scroll.delta.y,
        MouseScrollUnit::Pixel => scroll.delta.y / 40.0,
    };
    let nearer = keys.pressed(KeyCode::Equal) || keys.pressed(KeyCode::NumpadAdd);
    let farther = keys.pressed(KeyCode::Minus) || keys.pressed(KeyCode::NumpadSubtract);
    let key_delta = (farther as u8 as f32 - nearer as u8 as f32) * time.delta_secs() * 0.45;
    lens.goal_zoom = (lens.goal_zoom - lines * 0.07 + key_delta).clamp(0.0, 1.0);
    if keys.just_pressed(KeyCode::KeyC) {
        lens.goal_zoom = if lens.goal_zoom > 0.05 { 0.0 } else { DEFAULT_ZOOM };
    }
    if keys.just_pressed(KeyCode::KeyR) || keys.just_pressed(KeyCode::Home) {
        lens.goal_zoom = DEFAULT_ZOOM;
        lens.focus_gate = false;
    }
    if keys.just_pressed(KeyCode::KeyF) {
        lens.focus_gate = !lens.focus_gate;
    }
    let k = 1.0 - (-8.0 * time.delta_secs()).exp();
    lens.zoom = lens.zoom.lerp(lens.goal_zoom, k);
    if (lens.zoom - lens.goal_zoom).abs() < 0.0002 {
        lens.zoom = lens.goal_zoom;
    }
    let (pitch, distance, wide) = lens_geometry(lens.zoom);
    let aspect = window.width() / window.height().max(1.0);
    let foot = world::ground(expedition.leader);
    lens.pitch_deg = pitch;
    lens.distance = distance * (1.40 / aspect.max(0.45)).max(1.0);
    lens.target = composed_target(foot, wide);
    *pose = viewpoint(lens.target, lens.pitch_deg, lens.distance);
    let subject = if lens.focus_gate {
        let [x, z] = PLAN.gate;
        Vec3::new(x, 3.30, z)
    } else {
        // A screen-aligned card's whole image shares its foot's view depth.
        foot
    };
    // Follow actual view-space depth immediately, including steps and zoom.
    dof.focal_distance = (subject - pose.translation).dot(*pose.forward()).clamp(0.2, 150.0);
}

fn timestamp() -> u128 {
    SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default().as_millis()
}

fn photograph(mut commands: Commands, keys: Res<ButtonInput<KeyCode>>) {
    if !keys.just_pressed(KeyCode::F12) {
        return;
    }
    if let Err(error) = std::fs::create_dir_all("captures") {
        warn!("Cannot create captures directory: {error}");
        return;
    }
    commands.spawn(Screenshot::primary_window())
        .observe(save_to_disk(format!("captures/sanctum-{}.png", timestamp())));
}

fn prove_lens(
    mut commands: Commands,
    proof: Option<ResMut<LensProof>>,
    time: Res<Time>,
    assets: Res<AssetServer>,
    scenes: Query<&SceneRoot>,
    camera: Single<(&mut Lens, &Transform, &DepthOfField)>,
    mut exit: EventWriter<AppExit>,
) {
    let Some(mut proof) = proof else { return };
    if !matches!(proof.phase, ProofPhase::Default | ProofPhase::Near) {
        return;
    }
    let (mut lens, pose, dof) = camera.into_inner();
    let near = proof.phase == ProofPhase::Near;
    lens.goal_zoom = if near { 0.0 } else { DEFAULT_ZOOM };
    let ready = !scenes.is_empty()
        && scenes.iter().all(|scene| assets.is_loaded_with_dependencies(scene.0.id()))
        && [
            "party/travelers.png", "party/body-surface.png",
            "water/spring_bed.png", "water/impact_foam.png",
            "relic/seal.png", "fonts/Inscription.ttf", "fonts/FieldPixel.ttf",
        ].iter().all(|path| {
            assets.get_path_id(*path).is_some_and(|id| assets.is_loaded_with_dependencies(id))
        });
    if !ready || (lens.zoom - lens.goal_zoom).abs() > 0.0002 {
        proof.settled = 0.0;
        return;
    }
    proof.settled += time.delta_secs();
    if proof.settled < if near { 2.0 } else { 4.0 } {
        return;
    }
    if let Err(error) = std::fs::create_dir_all(&proof.directory) {
        error!("Cannot create lens proof directory: {error}");
        proof.phase = ProofPhase::Finished;
        exit.write(AppExit::error());
        return;
    }
    let label = if near { "near" } else { "default" };
    let path = proof.directory.join(format!("lens-{label}-{}.png", proof.stamp));
    info!(
        "LENS_PROOF {label}: zoom={:.5} pitch_deg={:.3} distance={:.3} eye={:?} target={:?} focal_distance={:.3} output={}",
        lens.zoom, lens.pitch_deg, lens.distance, pose.translation,
        lens.target, dof.focal_distance, path.display()
    );
    proof.phase = if near { ProofPhase::SavingNear } else { ProofPhase::SavingDefault };
    let mut save = save_to_disk(path.clone());
    commands.spawn(Screenshot::primary_window()).observe(
        move |trigger: Trigger<ScreenshotCaptured>, mut proof: ResMut<LensProof>, mut exit: EventWriter<AppExit>| {
            save(trigger);
            let mut signature = [0_u8; 8];
            let saved = std::fs::File::open(&path)
                .and_then(|mut file| file.read_exact(&mut signature))
                .is_ok() && signature == *b"\x89PNG\r\n\x1a\n";
            if !saved {
                error!("Lens proof PNG could not be read: {}", path.display());
                proof.phase = ProofPhase::Finished;
                exit.write(AppExit::error());
            } else if near {
                info!("LENS_PROOF complete: both native camera captures saved");
                proof.phase = ProofPhase::Finished;
                exit.write(AppExit::Success);
            } else {
                proof.phase = ProofPhase::Near;
                proof.settled = 0.0;
            }
        },
    );
}
