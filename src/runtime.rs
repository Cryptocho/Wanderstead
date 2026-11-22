//! Frame pacing, engine screenshots, and opt-in local desktop playtesting.
use crate::{
    game::{Interface, Outing},
    lens::LensState,
    stage::PaletteState,
    tileview::HoverTile,
};
use bevy::{
    input::{
        ButtonState, InputSystem,
        keyboard::{Key, KeyboardInput},
        mouse::{MouseButtonInput, MouseScrollUnit, MouseWheel},
    },
    prelude::*,
    render::view::screenshot::{Screenshot, ScreenshotCaptured},
    window::{PrimaryWindow, RawHandleWrapper},
};
use serde_json::json;
use std::{
    ffi::OsString,
    fs, io,
    os::unix::net::UnixDatagram,
    path::{Path, PathBuf},
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};

pub struct Options {
    qa: Option<PathBuf>,
    fps: Option<u16>,
    pub ephemeral: bool,
}
impl Options {
    pub fn read() -> Result<Option<Self>, String> {
        let mut o = Self {
            qa: None,
            fps: None,
            ephemeral: false,
        };
        let mut args = std::env::args_os().skip(1);
        while let Some(arg) = args.next() {
            match arg.to_str() {
                Some("--help" | "-h") => {
                    println!(
                        "铜壶镇 · 第一股泉水\n  cargo run --locked -- [--fps auto|100|50] [--fresh]\n  --fresh     从第一天开始，不读写手记\n  --qa PATH   启用本地桌面测试接口\n  左键点格走路、调查、交战；Q 守势，F 技艺，R 喝药，J 手记。滚轮或 +/- 变焦，Home 复位，F12 截图，Ctrl+Q 退出。"
                    );
                    return Ok(None);
                }
                Some("--qa") => {
                    o.qa = Some(
                        args.next()
                            .map(PathBuf::from)
                            .ok_or("--qa 需要 socket 路径")?,
                    );
                }
                Some("--fresh") => o.ephemeral = true,
                Some("--fps") => {
                    o.fps = match args.next().as_deref().and_then(|a| a.to_str()) {
                        Some("100") => Some(100),
                        Some("50") => Some(50),
                        Some("auto") => None,
                        _ => return Err("--fps 可选 auto、100 或 50".into()),
                    }
                }
                _ => {
                    return Err(format!(
                        "未知参数 {}。用 --help 查看操作。",
                        arg.to_string_lossy()
                    ));
                }
            }
        }
        Ok(Some(o))
    }
}
#[derive(Resource)]
struct Pace {
    hz: u16,
    auto: bool,
    slow: u16,
    considered: u16,
    started: Instant,
    prev: Instant,
    deadline: Option<Instant>,
    measuring: bool,
    samples: Vec<f32>,
}
#[derive(Resource)]
struct Wire {
    sock: UnixDatagram,
    path: PathBuf,
    dir: PathBuf,
}
impl Drop for Wire {
    fn drop(&mut self) {
        let _ = fs::remove_file(&self.path);
    }
}
pub fn configure(app: &mut App, opts: Options) -> io::Result<()> {
    let now = Instant::now();
    app.insert_resource(Pace {
        hz: opts.fps.unwrap_or(100),
        auto: opts.fps.is_none(),
        slow: 0,
        considered: 0,
        started: now,
        prev: now,
        deadline: None,
        measuring: false,
        samples: Vec::new(),
    });
    if let Some(path) = opts.qa {
        let dir = path
            .parent()
            .filter(|p| !p.as_os_str().is_empty())
            .unwrap_or(Path::new("."))
            .to_owned();
        let sock = UnixDatagram::bind(&path)?;
        sock.set_nonblocking(true)?;
        app.insert_resource(Wire { sock, path, dir });
    }
    Ok(())
}
pub struct RuntimePlugin;
impl Plugin for RuntimePlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(First, pace)
            .add_systems(Update, manual_capture);
        if app.world().contains_resource::<Wire>() {
            app.add_systems(PreUpdate, wire.before(InputSystem));
        }
    }
}
fn pace(mut p: ResMut<Pace>, w: Query<&Window, With<PrimaryWindow>>) {
    let start = Instant::now();
    let period = Duration::from_millis(if p.hz == 50 { 20 } else { 10 });
    if let Some(deadline) = p.deadline
        && let Some(left) = deadline.checked_duration_since(start)
    {
        std::thread::sleep(left);
    }
    let now = Instant::now();
    let delta = (now - p.prev).as_secs_f32() * 1000.;
    p.prev = now;
    p.deadline = Some(
        p.deadline
            .filter(|d| *d + period > now)
            .map_or(now + period, |d| d + period),
    );
    if p.measuring {
        p.samples.push(delta);
        if p.samples.len() == 600 {
            p.measuring = false;
        }
    }
    if p.auto
        && now.duration_since(p.started) > Duration::from_secs(12)
        && w.single().is_ok_and(|w| w.focused)
    {
        p.considered += 1;
        if delta > 15. {
            p.slow += 1;
        }
        if p.considered == 400 {
            if p.slow >= 32 {
                p.hz = 50;
                p.auto = false;
                info!("帧节奏切换到 50 FPS");
            }
            p.considered = 0;
            p.slow = 0;
        }
    }
}
fn capture(c: &mut Commands, path: PathBuf, reply: Option<(UnixDatagram, PathBuf)>) {
    c.spawn(Screenshot::primary_window())
        .observe(move |trigger: Trigger<ScreenshotCaptured>| {
            let result = trigger
                .event()
                .0
                .clone()
                .try_into_dynamic()
                .map_err(|e| format!("{e:?}"))
                .and_then(|image| image.into_rgb8().save(&path).map_err(|e| e.to_string()));
            match &result {
                Ok(_) => info!("截图 {}", path.display()),
                Err(e) => error!("截图失败：{e}"),
            };
            if let Some((s, p)) = &reply {
                let answer = match result {
                    Ok(_) => json!({"image":path}),
                    Err(e) => json!({"error":e}),
                };
                let _ = s.send_to(answer.to_string().as_bytes(), p);
            }
        });
}
fn manual_capture(k: Res<ButtonInput<KeyCode>>, mut c: Commands) {
    if !k.just_pressed(KeyCode::F12) {
        return;
    }
    let dir = Path::new("captures");
    if let Err(e) = fs::create_dir_all(dir) {
        error!("截图目录：{e}");
        return;
    }
    let n = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis();
    capture(&mut c, dir.join(format!("plum-{n}.png")), None);
}
fn key(s: &str) -> Option<KeyCode> {
    use KeyCode::*;
    Some(match s {
        "up" => ArrowUp,
        "down" => ArrowDown,
        "left" => ArrowLeft,
        "right" => ArrowRight,
        "e" => KeyE,
        "q" => KeyQ,
        "f" => KeyF,
        "r" => KeyR,
        "n" => KeyN,
        "t" => KeyT,
        "v" => KeyV,
        "j" => KeyJ,
        "h" => KeyH,
        "w" => KeyW,
        "a" => KeyA,
        "s" => KeyS,
        "d" => KeyD,
        "space" => Space,
        "kp1" => Numpad1,
        "kp2" => Numpad2,
        "kp3" => Numpad3,
        "kp4" => Numpad4,
        "kp5" => Numpad5,
        "kp6" => Numpad6,
        "kp7" => Numpad7,
        "kp8" => Numpad8,
        "kp9" => Numpad9,
        "enter" => Enter,
        "esc" => Escape,
        "plus" => Equal,
        "minus" => Minus,
        "home" => Home,
        "f7" => F7,
        "f10" => F10,
        "f12" => F12,
        _ => return None,
    })
}
fn button(s: Option<&str>) -> Option<ButtonState> {
    match s {
        Some("down") => Some(ButtonState::Pressed),
        Some("up") => Some(ButtonState::Released),
        _ => None,
    }
}
#[allow(clippy::too_many_arguments)]
fn wire(
    wire: Res<Wire>,
    mut c: Commands,
    window: Single<(Entity, &mut Window, Option<&RawHandleWrapper>), With<PrimaryWindow>>,
    game: Res<Outing>,
    ui: Res<Interface>,
    lens: Res<LensState>,
    palette: Res<PaletteState>,
    sight: (ResMut<crate::sight::SightWindow>, Res<Assets<Image>>),
    camera: Single<(&Camera, &GlobalTransform), With<Camera3d>>,
    hover: Res<HoverTile>,
    passes: Res<bevy::diagnostic::DiagnosticsStore>,
    mut pace: ResMut<Pace>,
    mut keyboard: EventWriter<KeyboardInput>,
    mut mouse: EventWriter<MouseButtonInput>,
    mut wheel: EventWriter<MouseWheel>,
    mut quit: EventWriter<AppExit>,
) {
    let (entity, mut win, raw) = window.into_inner();
    let mut buf = [0_u8; 256];
    // A reply followed immediately by another input must not merge the two turns.
    let (n, peer) = match wire.sock.recv_from(&mut buf) {
        Ok(p) => p,
        Err(e) if e.kind() == io::ErrorKind::WouldBlock => return,
        Err(e) => {
            warn!("IPC: {e}");
            return;
        }
    };
    let Some(peer) = peer.as_pathname() else {
        return;
    };
    let mut words = std::str::from_utf8(&buf[..n])
        .unwrap_or("")
        .split_whitespace();
    let reply = match words.next() {
        Some("state") => {
            json!({"day":game.day,"turn":game.turn,"player":game.player.to_array(),"facing":game.facing.to_array(),"jelly":game.jelly.to_array(),"target":game.target.map(|p|p.to_array()),"fruit":game.fruit.iter().map(|p|p.to_array()).collect::<Vec<_>>(),"bag":game.plums,"eaten":game.eaten,"shaken":game.tree_shaken,"meal":game.breakfast,"last":format!("{:?}",game.action),"history":game.history,"page":format!("{:?}",ui.page),"cursor":ui.cursor.to_array(),"hud":ui.hud,"filling":ui.filling,"notice":game.notice,"lens":{"zoom":lens.zoom,"pitch":lens.pitch_deg,"distance":lens.distance},"ink":palette.enabled,"window":[win.physical_width(),win.physical_height()],"present":format!("{:?}",win.present_mode),"native":raw.map(|r|format!("{:?}",r.get_window_handle())).and_then(|s|s.split(['(',' ']).next().map(str::to_owned)),"focus":win.focused,"fps_target":pace.hz})
        }
        Some("expedition") => {
            let e = &game.expedition;
            json!({"area":format!("{:?}",game.area),"hp":e.hp,"max_hp":e.max_hp,"tonics":e.tonics,"kit":format!("{:?}",e.kit),"quest":format!("{:?}",e.quest),
                "beat":e.beat,"attempts":e.attempts,"kills":e.kills,"deaths":e.deaths,"retreats":e.retreats,"coins":e.loot,"banked":e.banked,"ready":e.skill_ready,
                "chest":e.seal_open,"remedy":e.tonic_open,"enemies":e.enemies.iter().map(|a|json!([a.id,format!("{:?}",a.kind),a.pos.to_array(),a.hp,a.warn.map(|p|p.to_array())])).collect::<Vec<_>>(),
                "hover":hover.cell.map(|p|p.to_array()),"path":hover.path.iter().map(|p|p.to_array()).collect::<Vec<_>>()})
        }
        Some("sight") => {
            let mut window = sight.0;
            match words.next() {
                Some("on") => window.enabled = true,
                Some("off") => window.enabled = false,
                _ => {}
            }
            let (rect, meta) = window.description();
            json!({"enabled":window.enabled,"rect":rect,"meta":meta})
        }
        Some("pick") => {
            let point = words
                .next()
                .and_then(|v| v.parse::<f32>().ok())
                .zip(words.next().and_then(|v| v.parse::<f32>().ok()))
                .map(|(x, y)| Vec2::new(x, y));
            let cell = point
                .filter(|&p| {
                    !crate::book::pointer_over_book(
                        p,
                        win.resolution.size(),
                        ui.page,
                        ui.hud,
                        game.area,
                    )
                })
                .and_then(|p| {
                    let ray = camera.0.viewport_to_world(camera.1, p).ok()?;
                    let near = sight
                        .0
                        .near(p * win.scale_factor(), ray, camera.1, &sight.1);
                    crate::tileview::pick_cell(&game, ray, camera.1, near)
                });
            json!(cell.map(|p| p.to_array()))
        }
        Some("project") => {
            let x = words.next().and_then(|v| v.parse::<f32>().ok());
            let z = words.next().and_then(|v| v.parse::<f32>().ok());
            let y = words
                .next()
                .and_then(|v| v.parse::<f32>().ok())
                .unwrap_or(0.);
            match x.zip(z).and_then(|(x, z)| {
                camera
                    .0
                    .world_to_viewport(camera.1, Vec3::new(x, y, z))
                    .ok()
            }) {
                Some(p) => json!(p.to_array()),
                None => json!({"error":"project X Z [Y] must be in front of the camera"}),
            }
        }
        Some("key") => {
            if let (Some(k), Some(state)) = (words.next().and_then(key), button(words.next())) {
                keyboard.write(KeyboardInput {
                    key_code: k,
                    logical_key: Key::Unidentified(bevy::input::keyboard::NativeKey::Unidentified),
                    state,
                    text: None,
                    repeat: false,
                    window: entity,
                });
                json!(true)
            } else {
                json!({"error":"key NAME down|up"})
            }
        }
        Some("scroll") => {
            if let Some(y) = words
                .next()
                .and_then(|x| x.parse::<f32>().ok())
                .filter(|y| y.is_finite())
            {
                wheel.write(MouseWheel {
                    unit: MouseScrollUnit::Line,
                    x: 0.,
                    y: y.clamp(-100., 100.),
                    window: entity,
                });
                json!(true)
            } else {
                json!({"error":"scroll LINES"})
            }
        }
        Some("pointer") => {
            if let (Some(x), Some(y)) = (
                words.next().and_then(|s| s.parse().ok()),
                words.next().and_then(|s| s.parse().ok()),
            ) {
                // Feed Bevy UI hit testing without requesting a Wayland pointer warp.
                win.bypass_change_detection()
                    .set_cursor_position(Some(Vec2::new(x, y)));
                json!(true)
            } else {
                json!({"error":"pointer X Y"})
            }
        }
        Some("mouse") => {
            if let Some(state) = button(words.next()) {
                mouse.write(MouseButtonInput {
                    button: MouseButton::Left,
                    state,
                    window: entity,
                });
                json!(true)
            } else {
                json!({"error":"mouse down|up"})
            }
        }
        Some("measure") => {
            pace.samples.clear();
            pace.samples.reserve(600);
            pace.measuring = true;
            json!({"collecting":600})
        }
        Some("passes") => json!(
            passes
                .iter()
                .filter(|d| d.path().as_str().ends_with("elapsed_gpu"))
                .filter_map(|d| d.average().map(|v| (d.path().as_str(), v)))
                .collect::<std::collections::BTreeMap<_, _>>()
        ),
        Some("stats") => {
            let mut a = pace.samples.clone();
            a.sort_by(f32::total_cmp);
            let n = a.len();
            if n == 0 {
                json!({"n":0})
            } else {
                json!({"n":n,"fps":1000.*n as f32/a.iter().sum::<f32>(),"p50":a[n/2],"p95":a[n*95/100],"p99":a[n*99/100],"target":pace.hz})
            }
        }
        Some("capture") => {
            if let Some(name) = words.next().filter(|n| {
                n.len() < 65
                    && n.bytes()
                        .all(|c| c.is_ascii_alphanumeric() || matches!(c, b'_' | b'-'))
            }) {
                match wire.sock.try_clone() {
                    Ok(s) => {
                        let mut file = OsString::from(name);
                        file.push(".png");
                        capture(&mut c, wire.dir.join(file), Some((s, peer.to_owned())));
                        return;
                    }
                    Err(e) => json!({"error":e.to_string()}),
                }
            } else {
                json!({"error":"capture NAME"})
            }
        }
        Some("exit") => {
            quit.write(AppExit::Success);
            json!(true)
        }
        _ => {
            json!({"error":"state/expedition/project/key/scroll/pointer/mouse/measure/stats/capture/exit"})
        }
    };
    let _ = wire.sock.send_to(reply.to_string().as_bytes(), peer);
}
