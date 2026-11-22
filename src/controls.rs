use crate::{Frame, game::*};
use bevy::{prelude::*, window::PrimaryWindow};

pub struct ControlsPlugin;
impl Plugin for ControlsPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, (keys, pointer).in_set(Frame::Input))
            .add_systems(Update, actions.in_set(Frame::Act));
    }
}
#[derive(Default)]
struct Held {
    dir: IVec2,
    next: f64,
}
fn keys(
    k: Res<ButtonInput<KeyCode>>,
    t: Res<Time<Real>>,
    mut held: Local<Held>,
    mut ui: ResMut<Interface>,
    mut intents: EventWriter<Intent>,
    mut quit: EventWriter<AppExit>,
) {
    use KeyCode::*;
    if (k.pressed(ControlLeft) || k.pressed(ControlRight)) && k.just_pressed(KeyQ) {
        quit.write(AppExit::Success);
        return;
    }
    if k.just_pressed(KeyH) {
        ui.hud = !ui.hud;
    }
    let mut command = None;
    for (code, action) in [
        (Escape, Intent::Cancel),
        (KeyJ, Intent::Book),
        (Tab, Intent::Book),
        (KeyV, Intent::Look),
        (KeyT, Intent::Toss),
        (Space, Intent::Wait),
        (KeyE, Intent::Interact),
        (Enter, Intent::Interact),
        (NumpadEnter, Intent::Interact),
    ] {
        if k.just_pressed(code) {
            command = Some(action);
            break;
        }
    }
    let bindings = [
        (ArrowUp, KeyW, IVec2::NEG_Y),
        (ArrowDown, KeyS, IVec2::Y),
        (ArrowLeft, KeyA, IVec2::NEG_X),
        (ArrowRight, KeyD, IVec2::X),
    ];
    let d = bindings
        .iter()
        .find(|(a, b, _)| k.just_pressed(*a) || k.just_pressed(*b))
        .or_else(|| {
            bindings
                .iter()
                .find(|(a, b, _)| k.pressed(*a) || k.pressed(*b))
        })
        .map_or(IVec2::ZERO, |(_, _, d)| *d);
    let now = t.elapsed_secs_f64();
    let fresh = bindings
        .iter()
        .any(|(a, b, v)| *v == d && (k.just_pressed(*a) || k.just_pressed(*b)));
    if command.is_none() && d != IVec2::ZERO && (fresh || held.dir != d || now >= held.next) {
        held.next = now + if fresh || held.dir != d { 0.29 } else { 0.145 };
        command = match (ui.page, d.x) {
            (Page::Bowl | Page::Book, -1) => Some(Intent::Less),
            (Page::Bowl | Page::Book, 1) => Some(Intent::More),
            _ => Some(Intent::Dir(d)),
        };
    }
    held.dir = d;
    if let Some(i) = command {
        intents.write(i);
    }
}
fn pointer(
    buttons: Res<ButtonInput<MouseButton>>,
    window: Single<&Window, With<PrimaryWindow>>,
    camera: Single<(&Camera, &GlobalTransform), With<Camera3d>>,
    ui: Res<Interface>,
    mut out: EventWriter<Intent>,
) {
    if !buttons.just_pressed(MouseButton::Left)
        || !matches!(ui.page, Page::Roam | Page::Aim | Page::Look)
    {
        return;
    }
    let Some(pos) = window.cursor_position() else {
        return;
    };
    if crate::book::pointer_over_book(pos, window.resolution.size(), ui.page, ui.hud) {
        return;
    }
    let Ok(ray) = camera.0.viewport_to_world(camera.1, pos) else {
        return;
    };
    if ray.direction.y >= -0.005 {
        return;
    }
    let p = ray.origin + *ray.direction * (-ray.origin.y / ray.direction.y);
    let cell = IVec2::new(p.x.round() as i32, p.z.round() as i32);
    if on_map(cell) {
        out.write(Intent::Point(cell));
    }
}
#[derive(Default)]
struct WalkTo {
    dest: Option<IVec2>,
    next: f64,
}
fn actions(
    mut list: EventReader<Intent>,
    mut g: ResMut<Outing>,
    mut ui: ResMut<Interface>,
    mut route: Local<WalkTo>,
    t: Res<Time<Real>>,
    journal: Res<JournalFile>,
) {
    let before = g.history.len();
    let now = t.elapsed_secs_f64();
    for &i in list.read() {
        route.dest = None;
        if let Intent::Point(p) = i
            && ui.page == Page::Roam
            && !g.adjacent_object(p)
        {
            route.dest = Some(p);
            route.next = now;
            continue;
        }
        g.act(&mut ui, i);
    }
    if let Some(target) = route.dest
        && now >= route.next
    {
        if ui.page == Page::Roam
            && let Some(d) = g.route(target)
        {
            g.act(&mut ui, Intent::Dir(d));
            route.next = now + 0.15;
            if g.player == target || g.adjacent_object(target) {
                route.dest = None;
            }
        } else {
            route.dest = None;
        }
    }
    if g.history.len() != before
        && let Err(e) = journal.save(&g.history)
    {
        error!("写入旅行手记失败：{e}");
        g.notice = "手记没能存到磁盘，这次早餐仍在当前书页里。";
    }
}
