mod book;
mod controls;
mod delve;
mod figures;
mod game;
mod lens;
mod map;
mod pixel_press;
mod runtime;
mod sight;
mod spring;
mod stage;
mod tileview;

use bevy::{
    prelude::*,
    window::{MonitorSelection, PresentMode, WindowMode, WindowResolution},
    winit::WinitSettings,
};
use std::{num::NonZeroU32, process::ExitCode};

#[derive(SystemSet, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Frame {
    Input,
    Act,
    Draw,
}

fn main() -> ExitCode {
    let opts = match runtime::Options::read() {
        Ok(Some(o)) => o,
        Ok(None) => return ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("{e}");
            return ExitCode::FAILURE;
        }
    };
    if std::env::var_os("WAYLAND_DISPLAY").is_none() && std::env::var_os("WAYLAND_SOCKET").is_none()
    {
        eprintln!("请在 Wayland 桌面内运行。此版本没有启用 X11 后端。");
        return ExitCode::FAILURE;
    }
    let journal = game::JournalFile::open(opts.ephemeral);
    let outing = journal.resume();
    let mut app = App::new();
    if let Err(e) = runtime::configure(&mut app, opts) {
        eprintln!("{e}");
        return ExitCode::FAILURE;
    }
    app.insert_resource(outing)
        .insert_resource(journal)
        .init_resource::<game::Interface>()
        .add_event::<game::Intent>()
        .insert_resource(WinitSettings::game())
        .add_plugins(
            DefaultPlugins
                .set(WindowPlugin {
                    primary_window: Some(Window {
                        title: "铜壶镇 · 第一股泉水 | Wanderstead".into(),
                        name: Some("Wanderstead".into()),
                        mode: WindowMode::BorderlessFullscreen(MonitorSelection::Current),
                        resolution: WindowResolution::new(2560., 1440.)
                            .with_scale_factor_override(1.),
                        present_mode: PresentMode::Fifo,
                        desired_maximum_frame_latency: NonZeroU32::new(2),
                        ..default()
                    }),
                    ..default()
                })
                .set(ImagePlugin::default_nearest()),
        )
        .configure_sets(Update, (Frame::Input, Frame::Act, Frame::Draw).chain())
        .add_plugins((
            controls::ControlsPlugin,
            lens::LensPlugin,
            tileview::TileViewPlugin,
            sight::SightPlugin,
            pixel_press::PixelPressPlugin,
            stage::StagePlugin,
            spring::SpringPlugin,
            figures::FiguresPlugin,
            book::BookPlugin,
            runtime::RuntimePlugin,
        ));
    if std::env::var_os("WANDERSTEAD_GPU_PROFILE").is_some() {
        app.add_plugins(bevy::render::diagnostic::RenderDiagnosticsPlugin);
    }
    match app.run() {
        AppExit::Success => ExitCode::SUCCESS,
        AppExit::Error(e) => ExitCode::from(e.get()),
    }
}
