use crate::{Frame, game::*, tileview::HoverTile};
use bevy::{prelude::*, window::PrimaryWindow};

pub struct ControlsPlugin;

/// Hover sampling is ordered between keyboard navigation and pointer dispatch.
#[derive(SystemSet, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ControlsSet {
    Keys,
    Pointer,
}

impl Plugin for ControlsPlugin {
    fn build(&self, app: &mut App) {
        app.configure_sets(
            Update,
            (ControlsSet::Keys, ControlsSet::Pointer)
                .chain()
                .in_set(Frame::Input),
        )
        .add_systems(Update, keys.in_set(ControlsSet::Keys))
        .add_systems(Update, pointer.in_set(ControlsSet::Pointer))
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
        (KeyQ, Intent::Brace),
        (KeyF, Intent::Skill),
        (KeyR, Intent::Drink),
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
            (Page::Bowl | Page::Book | Page::Gear, -1) => Some(Intent::Less),
            (Page::Bowl | Page::Book | Page::Gear, 1) => Some(Intent::More),
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
    keys: Res<ButtonInput<KeyCode>>,
    hover: Res<HoverTile>,
    window: Single<&Window, With<PrimaryWindow>>,
    ui: Res<Interface>,
    outing: Res<Outing>,
    mut out: EventWriter<Intent>,
) {
    if buttons.just_pressed(MouseButton::Right) {
        out.write(Intent::Cancel);
        return;
    }
    if !buttons.just_pressed(MouseButton::Left)
        || !matches!(ui.page, Page::Roam | Page::Aim | Page::Look)
        || keys.get_just_pressed().next().is_some()
    {
        return;
    }
    let Some(pos) = window.cursor_position() else {
        return;
    };
    if crate::book::pointer_over_book(pos, window.resolution.size(), ui.page, ui.hud, outing.area) {
        return;
    }
    if let Some(cell) = hover.cell {
        out.write(Intent::Point(cell));
    }
}

#[derive(Default)]
struct WalkTo {
    dest: Option<IVec2>,
    path: Vec<IVec2>,
    next: f64,
    single_step: bool,
    confirmed_hazard: bool,
    interaction: bool,
    area: Option<crate::map::Area>,
}

fn actions(
    mut list: EventReader<Intent>,
    mut g: ResMut<Outing>,
    mut ui: ResMut<Interface>,
    mut route: Local<WalkTo>,
    t: Res<Time<Real>>,
    journal: Res<JournalFile>,
) {
    let revision = g.save_revision;
    let now = t.elapsed_secs_f64();
    let mut acted = false;
    for &i in list.read() {
        route.dest = None;
        if acted {
            continue;
        }
        if let Intent::Point(p) = i {
            if ui.page == Page::Roam && !g.adjacent_object(p) {
                if g.path_to(p, &mut route.path) && !route.path.is_empty() {
                    route.dest = Some(p);
                    route.single_step = g.threatened();
                    route.confirmed_hazard = route.path.len() == 1;
                    route.interaction = g.is_interactable(p);
                    route.area = Some(g.area);
                    route.next = now;
                }
                continue;
            }
            if matches!(ui.page, Page::Aim | Page::Look) {
                // The hovered cell is already selected: one click confirms it.
                ui.cursor = p;
                g.act(&mut ui, Intent::Interact);
                acted = true;
                continue;
            }
        }
        g.act(&mut ui, i);
        acted = true;
    }
    if !acted
        && let Some(target) = route.dest
        && now >= route.next
    {
        if ui.page != Page::Roam
            || route.area != Some(g.area)
            || (route.interaction && !g.is_interactable(target))
            || !g.path_to(target, &mut route.path)
        {
            route.dest = None;
        } else if route.path.is_empty() {
            route.dest = None;
            if g.adjacent_object(target) {
                g.act(&mut ui, Intent::Point(target));
            }
        } else if !route.single_step
            && (g.threatened() || (g.danger_at(route.path[0]) && !route.confirmed_hazard))
        {
            route.dest = None;
        } else {
            let previous = g.player;
            let area = g.area;
            let hp = g.expedition.hp;
            let d = route.path[0] - previous;
            g.act(&mut ui, Intent::Dir(d));
            route.next = now + 0.29;
            if route.single_step
                || g.area != area
                || g.expedition.hp < hp
                || g.player != previous + d
                || g.action != ActionKind::Step
                || ui.page != Page::Roam
                || g.threatened()
            {
                route.dest = None;
            }
        }
    }
    if g.save_revision != revision
        && let Err(e) = journal.save(&g)
    {
        error!("写入旅行手记失败：{e}");
        g.notice = "手记没能存到磁盘，当前旅程仍然保留。";
    }
}
