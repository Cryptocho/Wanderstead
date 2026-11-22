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
    pending: Option<IVec2>,
    deadline: f64,
}
impl Held {
    fn sample(&mut self, now: f64, d: IVec2, immediate: bool) -> Option<IVec2> {
        if d == IVec2::ZERO {
            self.dir = d;
            return self.pending.take();
        }
        if self.dir == IVec2::ZERO {
            self.dir = d;
            if immediate || (d.x != 0 && d.y != 0) {
                self.next = now + 0.29;
                return Some(d);
            }
            self.pending = Some(d);
            self.deadline = now + 0.065;
        }
        self.dir = d;
        if self.pending.is_some() {
            self.pending = Some(d);
            if immediate || (d.x != 0 && d.y != 0) || now >= self.deadline {
                self.next = now + 0.29;
                return self.pending.take();
            }
        } else if now >= self.next {
            self.next = now + 0.145;
            return Some(d);
        }
        None
    }
}

pub(crate) fn direction(k: &ButtonInput<KeyCode>) -> IVec2 {
    use KeyCode::*;
    let mut d = IVec2::ZERO;
    for (codes, delta) in [
        ([ArrowUp, KeyW, Numpad8], IVec2::NEG_Y),
        ([ArrowDown, KeyS, Numpad2], IVec2::Y),
        ([ArrowLeft, KeyA, Numpad4], IVec2::NEG_X),
        ([ArrowRight, KeyD, Numpad6], IVec2::X),
    ] {
        if codes.into_iter().any(|code| k.pressed(code)) {
            d += delta;
        }
    }
    for (code, delta) in [
        (Numpad7, IVec2::new(-1, -1)),
        (Numpad9, IVec2::new(1, -1)),
        (Numpad1, IVec2::new(-1, 1)),
        (Numpad3, IVec2::new(1, 1)),
    ] {
        if k.pressed(code) {
            d += delta;
        }
    }
    d.signum()
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
        (Numpad5, Intent::Wait),
        (KeyE, Intent::Interact),
        (Enter, Intent::Interact),
        (NumpadEnter, Intent::Interact),
    ] {
        if k.just_pressed(code) {
            command = Some(action);
            break;
        }
    }
    let d = direction(&k);
    let keypad = [
        Numpad1, Numpad2, Numpad3, Numpad4, Numpad6, Numpad7, Numpad8, Numpad9,
    ]
    .into_iter()
    .any(|code| k.pressed(code));
    if command.is_some() {
        held.pending = None;
        held.dir = d;
        held.next = t.elapsed_secs_f64() + 0.29;
    } else if d == IVec2::ZERO
        && (keypad
            || [
                KeyW, KeyS, KeyA, KeyD, ArrowUp, ArrowDown, ArrowLeft, ArrowRight,
            ]
            .into_iter()
            .any(|code| k.pressed(code)))
    {
        // Opposing held directions cancel; this is not a released single-key tap.
        held.pending = None;
        held.dir = IVec2::ZERO;
    } else if let Some(d) = held.sample(t.elapsed_secs_f64(), d, keypad) {
        command = match (ui.page, d.x) {
            (Page::Bowl | Page::Book | Page::Gear, -1) => Some(Intent::Less),
            (Page::Bowl | Page::Book | Page::Gear, 1) => Some(Intent::More),
            _ => Some(Intent::Dir(d)),
        };
    }
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn near_sequential_chord_commits_one_diagonal_without_release_steps() {
        let mut held = Held::default();
        assert_eq!(held.sample(0.0, IVec2::NEG_Y, false), None);
        assert_eq!(
            held.sample(0.04, IVec2::new(1, -1), false),
            Some(IVec2::new(1, -1))
        );
        assert_eq!(held.sample(0.08, IVec2::X, false), None);
        assert_eq!(held.sample(0.09, IVec2::ZERO, false), None);
    }

    #[test]
    fn single_taps_flush_on_release_and_holds_repeat_at_action_cadence() {
        let mut held = Held::default();
        assert_eq!(held.sample(0.0, IVec2::X, false), None);
        assert_eq!(held.sample(0.03, IVec2::ZERO, false), Some(IVec2::X));
        assert_eq!(held.sample(0.04, IVec2::ZERO, false), None);
        assert_eq!(held.sample(0.10, IVec2::Y, false), None);
        assert_eq!(held.sample(0.166, IVec2::Y, false), Some(IVec2::Y));
        assert_eq!(held.sample(0.40, IVec2::new(1, 1), false), None);
        assert_eq!(
            held.sample(0.457, IVec2::new(1, 1), false),
            Some(IVec2::new(1, 1))
        );
        assert_eq!(
            held.sample(0.603, IVec2::new(1, 1), false),
            Some(IVec2::new(1, 1))
        );
    }

    #[test]
    fn keypad_and_alias_chords_have_all_eight_directions() {
        use KeyCode::*;
        for (code, expected) in [
            (Numpad1, IVec2::new(-1, 1)),
            (Numpad2, IVec2::Y),
            (Numpad3, IVec2::new(1, 1)),
            (Numpad4, IVec2::NEG_X),
            (Numpad6, IVec2::X),
            (Numpad7, IVec2::new(-1, -1)),
            (Numpad8, IVec2::NEG_Y),
            (Numpad9, IVec2::new(1, -1)),
        ] {
            let mut keys = ButtonInput::default();
            keys.press(code);
            assert_eq!(direction(&keys), expected);
            assert_eq!(Held::default().sample(0.0, expected, true), Some(expected));
        }
        let mut keys = ButtonInput::default();
        keys.press(KeyW);
        keys.press(ArrowUp);
        keys.press(KeyD);
        assert_eq!(direction(&keys), IVec2::new(1, -1));
        keys.press(KeyS);
        assert_eq!(direction(&keys), IVec2::X);
    }

    fn click_app() -> App {
        let mut g = Outing::new(vec![]);
        g.player = IVec2::new(-2, 0);
        g.fruit.clear();
        let mut app = App::new();
        app.insert_resource(g)
            .insert_resource(Interface::default())
            .insert_resource(Time::<Real>::default())
            .insert_resource(JournalFile::open(true))
            .add_event::<Intent>()
            .add_systems(Update, actions);
        app
    }

    fn elapse(app: &mut App, seconds: f64) {
        app.world_mut()
            .resource_mut::<Time<Real>>()
            .advance_by(std::time::Duration::from_secs_f64(seconds));
        app.update();
    }

    #[test]
    fn click_walk_executes_the_previewed_shortest_diagonals_and_stops() {
        let mut app = click_app();
        let target = IVec2::new(0, -2);
        app.world_mut().send_event(Intent::Point(target));
        app.update();
        let g = app.world().resource::<Outing>();
        assert_eq!((g.player, g.turn), (IVec2::new(-1, -1), 1));
        elapse(&mut app, 0.30);
        let g = app.world().resource::<Outing>();
        assert_eq!((g.player, g.turn), (target, 2));
        elapse(&mut app, 0.30);
        assert_eq!(app.world().resource::<Outing>().turn, 2);
    }

    #[test]
    fn new_keyboard_action_cancels_click_route_and_wait_spends_one_turn() {
        let mut app = click_app();
        app.world_mut().send_event(Intent::Point(IVec2::new(0, -2)));
        app.update();
        app.world_mut().send_event(Intent::Wait);
        elapse(&mut app, 0.30);
        elapse(&mut app, 0.30);
        let g = app.world().resource::<Outing>();
        assert_eq!(
            (g.player, g.turn, g.action),
            (IVec2::new(-1, -1), 2, ActionKind::Wait)
        );
    }
}
