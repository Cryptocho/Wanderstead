//! The same morning swatch drives live sunlight, local shafts and the off-line reflection bake.
use bevy::{
    core_pipeline::bloom::{Bloom, BloomCompositeMode, BloomPrefilter},
    prelude::*,
    render::view::{ColorGrading, ColorGradingGlobal, ColorGradingSection},
};
use serde::Deserialize;
use std::sync::LazyLock;

pub(crate) static SQUARE: LazyLock<Daylight> = LazyLock::new(|| {
    serde_json::from_str(include_str!("../assets/inks/market-morning.json"))
        .expect("authored market daylight profile")
});

#[derive(Deserialize)]
pub(crate) struct Daylight {
    pub sun: Sunlight,
    pub sky: Sky,
    pub film: Film,
    pub focus: Focus,
}

#[derive(Deserialize)]
pub(crate) struct Sunlight {
    bearing_deg: f32,
    elevation_deg: f32,
    pub srgb: [f32; 3],
    pub lux: f32,
}

#[derive(Deserialize)]
pub(crate) struct Sky {
    pub fill_srgb: [f32; 3],
    pub fill_brightness: f32,
    pub probe_intensity: f32,
    pub skybox_brightness: f32,
    pub haze_srgb: [f32; 3],
}

#[derive(Deserialize)]
pub(crate) struct Film {
    pub ev100: f32,
    tint: f32,
    shadow_lift: f32,
    highlight_gain: f32,
    highlight_saturation: f32,
    pub bloom_threshold: f32,
    bloom_softness: f32,
    bloom_intensity: f32,
}

#[derive(Deserialize)]
pub(crate) struct Focus {
    /// Eye-level and overhead endpoints; the lens interpolates between them.
    pub sensor_m: [f32; 2],
    pub f_stops: [f32; 2],
    pub limit_px_at_1440: f32,
}

pub(crate) fn srgb([r, g, b]: [f32; 3]) -> Color {
    Color::srgb(r, g, b)
}

impl Daylight {
    /// Artist bearings are clockwise from north (-Z). Rays and cast shadows go the other way.
    pub fn to_sun(&self) -> Vec3 {
        let (east, north) = self.sun.bearing_deg.to_radians().sin_cos();
        let (up, flat) = self.sun.elevation_deg.to_radians().sin_cos();
        Vec3::new(east * flat, up, -north * flat)
    }
}

impl Film {
    pub fn grade(&self) -> ColorGrading {
        ColorGrading {
            global: ColorGradingGlobal {
                tint: self.tint,
                ..default()
            },
            shadows: ColorGradingSection {
                lift: self.shadow_lift,
                ..default()
            },
            highlights: ColorGradingSection {
                gain: self.highlight_gain,
                saturation: self.highlight_saturation,
                ..default()
            },
            ..default()
        }
    }

    pub fn bloom(&self) -> Bloom {
        Bloom {
            intensity: self.bloom_intensity,
            prefilter: BloomPrefilter {
                threshold: self.bloom_threshold,
                threshold_softness: self.bloom_softness,
            },
            composite_mode: BloomCompositeMode::Additive,
            // Favor tight halos on a wick, blossom tip or wave instead of a broad veil.
            low_frequency_boost: 0.18,
            low_frequency_boost_curvature: 0.60,
            high_pass_frequency: 0.86,
            ..Bloom::NATURAL
        }
    }
}

/// Opt-in QA A/B controls. No gameplay time, exposure, or camera motion changes.
#[derive(Resource)]
pub(crate) struct Optics {
    pub bloom: bool,
    pub dof: bool,
    pub air: bool,
}

impl Default for Optics {
    fn default() -> Self {
        Self {
            bloom: true,
            dof: true,
            air: true,
        }
    }
}
