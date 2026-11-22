use crate::{Frame, map::Area, scene::SceneState, tileview::HoverTile};
use bevy::prelude::*;

pub struct ControlsPlugin;

/// Hover sampling is ordered between keyboard navigation and pointer dispatch.
#[derive(SystemSet, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ControlsSet {
    Keys,
    Pointer,
}

impl Plugin for ControlsPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<Navigation>()
            .configure_sets(
                Update,
                (ControlsSet::Keys, ControlsSet::Pointer)
                    .chain()
                    .in_set(Frame::Input),
            )
            .add_systems(Update, keys.in_set(ControlsSet::Keys))
            .add_systems(Update, pointer.in_set(ControlsSet::Pointer))
            .add_systems(Update, navigate.in_set(Frame::Act));
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

#[derive(Resource, Default)]
struct Navigation {
    manual: Option<IVec2>,
    path: Vec<IVec2>,
    step: usize,
    area: Option<Area>,
    next: f64,
    toggle_area: bool,
}

impl Navigation {
    fn cancel(&mut self) {
        self.path.clear();
        self.step = 0;
        self.area = None;
    }
}

fn movement_held(keys: &ButtonInput<KeyCode>) -> bool {
    use KeyCode::*;
    [
        KeyW, KeyS, KeyA, KeyD, ArrowUp, ArrowDown, ArrowLeft, ArrowRight, Numpad1, Numpad2,
        Numpad3, Numpad4, Numpad6, Numpad7, Numpad8, Numpad9,
    ]
    .into_iter()
    .any(|code| keys.pressed(code))
}

fn keys(
    keys: Res<ButtonInput<KeyCode>>,
    time: Res<Time<Real>>,
    mut held: Local<Held>,
    mut navigation: ResMut<Navigation>,
    mut quit: EventWriter<AppExit>,
) {
    use KeyCode::*;
    if (keys.pressed(ControlLeft) || keys.pressed(ControlRight)) && keys.just_pressed(KeyQ) {
        navigation.cancel();
        quit.write(AppExit::Success);
        return;
    }
    let direction = direction(&keys);
    let moving = movement_held(&keys);
    let now = time.elapsed_secs_f64();
    if keys.just_pressed(Escape) || keys.just_pressed(F6) {
        navigation.cancel();
        navigation.manual = None;
        navigation.toggle_area = keys.just_pressed(F6);
        held.pending = None;
        held.dir = direction;
        held.next = now + 0.29;
        return;
    }
    if moving {
        navigation.cancel();
    }
    if direction == IVec2::ZERO && moving {
        // Opposing held directions cancel, rather than releasing a pending tap.
        held.pending = None;
        held.dir = IVec2::ZERO;
        return;
    }
    let keypad = [
        Numpad1, Numpad2, Numpad3, Numpad4, Numpad6, Numpad7, Numpad8, Numpad9,
    ]
    .into_iter()
    .any(|code| keys.pressed(code));
    navigation.manual = held.sample(now, direction, keypad);
    if navigation.manual.is_some() {
        navigation.cancel();
    }
}

fn pointer(
    buttons: Res<ButtonInput<MouseButton>>,
    keys: Res<ButtonInput<KeyCode>>,
    hover: Res<HoverTile>,
    scene: Res<SceneState>,
    time: Res<Time<Real>>,
    mut navigation: ResMut<Navigation>,
) {
    if buttons.just_pressed(MouseButton::Right) {
        navigation.cancel();
        return;
    }
    if !buttons.just_pressed(MouseButton::Left)
        || movement_held(&keys)
        || keys.just_pressed(KeyCode::Escape)
        || keys.just_pressed(KeyCode::F6)
        || navigation.manual.is_some()
    {
        return;
    }
    navigation.cancel();
    if hover.cell.is_some() && !hover.path.is_empty() {
        navigation.path.extend_from_slice(&hover.path);
        navigation.area = Some(scene.area);
        navigation.next = time.elapsed_secs_f64();
    }
}

fn navigate(
    mut scene: ResMut<SceneState>,
    mut navigation: ResMut<Navigation>,
    time: Res<Time<Real>>,
) {
    if navigation.toggle_area {
        navigation.toggle_area = false;
        navigation.cancel();
        scene.toggle_area();
        return;
    }
    if let Some(direction) = navigation.manual.take() {
        navigation.cancel();
        scene.step(direction);
        return;
    }
    if navigation.path.is_empty() {
        return;
    }
    if navigation.area != Some(scene.area) {
        navigation.cancel();
        return;
    }
    let now = time.elapsed_secs_f64();
    if now < navigation.next {
        return;
    }
    let next = navigation.path[navigation.step];
    let direction = next - scene.player;
    if !scene.step(direction) {
        navigation.cancel();
        return;
    }
    navigation.step += 1;
    navigation.next = now + 0.29;
    if navigation.step == navigation.path.len() {
        navigation.cancel();
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
        let mut app = App::new();
        app.insert_resource(SceneState {
            player: IVec2::new(-2, 0),
            ..default()
        })
        .insert_resource(Time::<Real>::default())
        .init_resource::<Navigation>()
        .add_systems(Update, navigate);
        app
    }

    fn click(app: &mut App, target: IVec2) {
        let mut path = Vec::new();
        let scene = app.world().resource::<SceneState>();
        assert!(scene.path_to(target, &mut path));
        let area = scene.area;
        let mut navigation = app.world_mut().resource_mut::<Navigation>();
        navigation.path = path;
        navigation.area = Some(area);
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
        click(&mut app, target);
        app.update();
        let scene = app.world().resource::<SceneState>();
        assert_eq!((scene.player, scene.movement), (IVec2::new(-1, -1), 1));
        elapse(&mut app, 0.28);
        assert_eq!(app.world().resource::<SceneState>().movement, 1);
        elapse(&mut app, 0.02);
        let scene = app.world().resource::<SceneState>();
        assert_eq!((scene.player, scene.movement), (target, 2));
        elapse(&mut app, 0.30);
        assert_eq!(app.world().resource::<SceneState>().movement, 2);
    }

    #[test]
    fn manual_movement_cancels_the_click_route() {
        let mut app = click_app();
        click(&mut app, IVec2::new(0, -2));
        app.update();
        app.world_mut().resource_mut::<Navigation>().manual = Some(IVec2::NEG_X);
        elapse(&mut app, 0.30);
        elapse(&mut app, 0.30);
        let scene = app.world().resource::<SceneState>();
        assert_eq!((scene.player, scene.movement), (IVec2::new(-2, -1), 2));
    }
}
