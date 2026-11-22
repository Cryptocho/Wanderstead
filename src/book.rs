use crate::{
    delve::{EnemyKind, Kit, Quest},
    game::*,
    map::{self, Area},
};
use bevy::{
    prelude::*,
    text::{FontSmoothing, LineHeight},
    ui::widget::NodeImageMode,
    window::PrimaryWindow,
};
use std::fmt::Write;

/// Small expedition instruments and a bound, modal travel journal.
pub struct BookPlugin;
impl Plugin for BookPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<Folio>()
            .add_systems(Startup, bind_book)
            .add_systems(Update, (scale_book, click_book).in_set(crate::Frame::Input))
            .add_systems(
                Update,
                (survey, turn_leaf, typeset_book)
                    .chain()
                    .in_set(crate::Frame::Draw),
            );
    }
}

const INK: Color = Color::srgb(0.25, 0.19, 0.15);
const FADED: Color = Color::srgb(0.43, 0.33, 0.23);
const IVORY: Color = Color::srgb(0.96, 0.91, 0.79);
const MUTED: Color = Color::srgb(0.75, 0.78, 0.66);
const CELLS: usize = map::WIDTH * map::HEIGHT;

fn modal(page: Page) -> bool {
    matches!(
        page,
        Page::Book
            | Page::Bowl
            | Page::Served
            | Page::Gear
            | Page::Barter
            | Page::Down
            | Page::Homecoming
    )
}

/// One geometry source for both the actual UI nodes and ground-pointer exclusion.
/// Dimensions are in window-height / 900 units. Bottom-center is always open.
#[derive(Component, Clone, Copy)]
enum HudPart {
    Place,
    Journal,
    Vitals,
    Notice,
    LeftActions,
    TownActions,
    DelveActions,
    Mini,
    Field,
}
const HUD_PARTS: [HudPart; 9] = [
    HudPart::Place,
    HudPart::Journal,
    HudPart::Vitals,
    HudPart::Notice,
    HudPart::LeftActions,
    HudPart::TownActions,
    HudPart::DelveActions,
    HudPart::Mini,
    HudPart::Field,
];
impl HudPart {
    fn rect(self, width: f32) -> [f32; 4] {
        match self {
            Self::Place => [22.0, 20.0, 350.0, 72.0],
            Self::Journal => [width - 194.0, 20.0, 172.0, 48.0],
            Self::Vitals => [22.0, 742.0, 324.0, 58.0],
            Self::Notice => [22.0, 102.0, 350.0, 112.0],
            Self::LeftActions => [22.0, 810.0, 324.0, 68.0],
            Self::TownActions => [width - 306.0, 810.0, 284.0, 68.0],
            Self::DelveActions => [width - 306.0, 754.0, 284.0, 124.0],
            Self::Mini => [width - 208.0, 84.0, 186.0, 164.0],
            Self::Field => [22.0, 454.0, 458.0, 254.0],
        }
    }
    fn node(self) -> Node {
        let [x, y, width, height] = self.rect(0.0);
        let mut node = at(x, y, width, height);
        if matches!(
            self,
            Self::Journal | Self::TownActions | Self::DelveActions | Self::Mini
        ) {
            node.left = Val::Auto;
            node.right = Val::Px(-x - width);
        }
        node
    }
    fn visible(self, page: Page, hud: bool, area: Area) -> bool {
        if modal(page) || (!hud && page == Page::Roam) {
            return false;
        }
        match self {
            Self::LeftActions => page == Page::Roam,
            Self::TownActions => page == Page::Roam && area == Area::Town,
            Self::DelveActions => page == Page::Roam && area == Area::Cistern,
            Self::Mini => area == Area::Cistern,
            Self::Field => matches!(page, Page::Aim | Page::Look),
            _ => true,
        }
    }
}
/// Window-logical coordinates. Modal pages also consume their closing click.
pub fn pointer_over_book(p: Vec2, window: Vec2, page: Page, hud: bool, area: Area) -> bool {
    if modal(page) {
        return true;
    }
    let scale = (window.y / 900.0).max(0.01);
    let p = p / scale;
    let width = window.x / scale;
    HUD_PARTS.into_iter().any(|part| {
        let [x, y, w, h] = part.rect(width);
        part.visible(page, hud, area) && p.x >= x && p.x <= x + w && p.y >= y && p.y <= y + h
    })
}

#[derive(Resource)]
struct Folio {
    leaf: usize,
    seen: [bool; CELLS],
}
impl Default for Folio {
    fn default() -> Self {
        Self {
            leaf: 0,
            seen: [false; CELLS],
        }
    }
}
#[derive(Component)]
enum Panel {
    Folio,
    JournalControls,
    ModalControls,
    BowlChoices,
    GearChoices,
    ServedChoices,
    Illustration,
    Keepsake(Page),
}
#[derive(Component, Clone, Copy)]
enum CopySlot {
    Place,
    Objective,
    Vitals,
    Notice,
    Heading,
    Summary,
    Body,
    FieldHeading,
    FieldBody,
    Leaf,
    Context,
    Skill,
    Confirm,
    Close,
}
#[derive(Component, Clone, Copy)]
enum Control {
    Context,
    Toss,
    Look,
    Book,
    Wait,
    Brace,
    Skill,
    Drink,
    Cancel,
    Less,
    More,
    Buckler,
    Hook,
    Confirm,
    NewMorning,
}
#[derive(Component)]
struct Available(bool);
#[derive(Component)]
struct HpFill;
#[derive(Component)]
struct MapCell(IVec2);
#[derive(Component)]
struct BreakfastArt;
#[derive(Resource)]
struct Illustrations {
    plain: Handle<Image>,
    fruit: Handle<Image>,
}

struct Stationery {
    font: Handle<Font>,
    button: Handle<Image>,
    glass: Handle<Image>,
    buckler: Handle<Image>,
    hook: Handle<Image>,
}
impl Stationery {
    fn text(&self, value: &str, size: f32, color: Color) -> impl Bundle + use<> {
        (
            Text::new(value),
            TextFont {
                font: self.font.clone(),
                font_size: size,
                font_smoothing: FontSmoothing::None,
                line_height: LineHeight::Px(size * 7.0 / 6.0),
                ..default()
            },
            TextColor(color),
        )
    }
    fn button(
        &self,
        parent: &mut ChildSpawnerCommands,
        caption: &str,
        action: Control,
        width: f32,
    ) {
        self.choice(parent, caption, action, width, 48.0);
    }
    fn choice(
        &self,
        parent: &mut ChildSpawnerCommands,
        caption: &str,
        action: Control,
        width: f32,
        height: f32,
    ) {
        parent
            .spawn((
                Button,
                action,
                Available(true),
                sliced(self.button.clone()),
                Node {
                    width: Val::Px(width),
                    height: Val::Px(height),
                    justify_content: JustifyContent::Center,
                    align_items: AlignItems::Center,
                    padding: UiRect::horizontal(Val::Px(8.0)),
                    flex_shrink: 0.0,
                    ..default()
                },
            ))
            .with_children(|b| {
                let item = match action {
                    Control::Buckler => Some(&self.buckler),
                    Control::Hook => Some(&self.hook),
                    _ => None,
                };
                if let Some(icon) = item {
                    b.spawn((ImageNode::new(icon.clone()), at(16., 19., 86., 86.)));
                    b.spawn((self.text(caption, 24., IVORY), at(119., 19., 357., 89.)));
                    return;
                }
                let mut text = b.spawn(self.text(caption, 24.0, IVORY));
                match action {
                    Control::Context => {
                        text.insert(CopySlot::Context);
                    }
                    Control::Skill => {
                        text.insert(CopySlot::Skill);
                    }
                    Control::Cancel => {
                        text.insert(CopySlot::Close);
                    }
                    Control::Confirm => {
                        text.insert(CopySlot::Confirm);
                    }
                    _ => {}
                }
            });
    }
}
fn sliced(image: Handle<Image>) -> ImageNode {
    ImageNode::new(image).with_mode(NodeImageMode::Sliced(TextureSlicer {
        border: BorderRect::all(14.0),
        center_scale_mode: SliceScaleMode::Stretch,
        sides_scale_mode: SliceScaleMode::Stretch,
        max_corner_scale: 1.0,
    }))
}
fn at(x: f32, y: f32, width: f32, height: f32) -> Node {
    Node {
        position_type: PositionType::Absolute,
        left: Val::Px(x),
        top: Val::Px(y),
        width: Val::Px(width),
        height: Val::Px(height),
        ..default()
    }
}
fn bind_book(mut commands: Commands, assets: Res<AssetServer>) {
    let s = Stationery {
        font: assets.load("book/FusionPixel.ttf"),
        button: assets.load("book/expedition-button.png"),
        glass: assets.load("book/expedition-glass.png"),
        buckler: assets.load("book/buckler-plate.png"),
        hook: assets.load("book/hook-plate.png"),
    };
    commands.insert_resource(Illustrations {
        plain: assets.load("book/plain-breakfast.png"),
        fruit: assets.load("book/breakfast.png"),
    });
    commands
        .spawn((
            Name::new("Location and next expedition goal"),
            HudPart::Place,
            sliced(s.glass.clone()),
            HudPart::Place.node(),
            GlobalZIndex(10),
        ))
        .with_children(|p| {
            p.spawn((
                s.text("", 24.0, IVORY),
                CopySlot::Place,
                at(14.0, 8.0, 322.0, 28.0),
            ));
            p.spawn((
                s.text("", 20.0, MUTED),
                CopySlot::Objective,
                at(14.0, 40.0, 322.0, 26.0),
            ));
        });
    commands
        .spawn((
            Name::new("Travel journal clasp"),
            HudPart::Journal,
            Button,
            Control::Book,
            Available(true),
            sliced(s.button.clone()),
            HudPart::Journal.node(),
            GlobalZIndex(10),
        ))
        .with_children(|p| {
            p.spawn((
                s.text("J  旅行手记", 24.0, IVORY),
                at(19.0, 9.0, 144.0, 30.0),
            ));
        });
    commands
        .spawn((
            Name::new("Actual expedition health"),
            HudPart::Vitals,
            sliced(s.glass.clone()),
            HudPart::Vitals.node(),
            GlobalZIndex(10),
        ))
        .with_children(|p| {
            p.spawn((
                ImageNode::new(assets.load("figures/traveler-portrait.png")),
                at(8.0, 5.0, 48.0, 48.0),
            ));
            p.spawn((
                s.text("", 20.0, IVORY),
                CopySlot::Vitals,
                at(64.0, 9.0, 244.0, 26.0),
            ));
            p.spawn((
                BackgroundColor(Color::srgb(0.10, 0.16, 0.14)),
                at(64.0, 42.0, 244.0, 6.0),
            ));
            p.spawn((
                HpFill,
                BackgroundColor(Color::srgb(0.42, 0.73, 0.60)),
                at(64.0, 42.0, 244.0, 6.0),
            ));
        });
    commands
        .spawn((
            Name::new("Resolved action and next hostile strike"),
            HudPart::Notice,
            sliced(s.glass.clone()),
            HudPart::Notice.node(),
            GlobalZIndex(10),
        ))
        .with_children(|p| {
            p.spawn((
                s.text("", 20.0, IVORY),
                CopySlot::Notice,
                at(14.0, 8.0, 322.0, 96.0),
            ));
        });
    commands
        .spawn((
            Name::new("Left corner interaction and investigation"),
            HudPart::LeftActions,
            sliced(s.glass.clone()),
            HudPart::LeftActions.node(),
            GlobalZIndex(10),
        ))
        .with_children(|p| {
            p.spawn(Node {
                column_gap: Val::Px(8.0),
                ..at(12.0, 10.0, 300.0, 48.0)
            })
            .with_children(|row| {
                s.button(row, "", Control::Context, 196.0);
                s.button(row, "V 调查", Control::Look, 96.0);
            });
        });
    commands
        .spawn((
            Name::new("Town fruit and waiting actions"),
            HudPart::TownActions,
            sliced(s.glass.clone()),
            HudPart::TownActions.node(),
            GlobalZIndex(10),
        ))
        .with_children(|p| {
            p.spawn(Node {
                column_gap: Val::Px(8.0),
                ..at(12.0, 10.0, 260.0, 48.0)
            })
            .with_children(|row| {
                s.button(row, "T 投果", Control::Toss, 120.0);
                s.button(row, "等候", Control::Wait, 132.0);
            });
        });
    commands
        .spawn((
            Name::new("Cistern mouse combat instruments"),
            HudPart::DelveActions,
            sliced(s.glass.clone()),
            HudPart::DelveActions.node(),
            GlobalZIndex(10),
        ))
        .with_children(|p| {
            p.spawn(Node {
                column_gap: Val::Px(8.0),
                ..at(12.0, 10.0, 260.0, 48.0)
            })
            .with_children(|row| {
                s.button(row, "Q 护卫", Control::Brace, 120.0);
                s.button(row, "F 器物技", Control::Skill, 132.0);
            });
            p.spawn(Node {
                column_gap: Val::Px(8.0),
                ..at(12.0, 66.0, 260.0, 48.0)
            })
            .with_children(|row| {
                s.button(row, "R 药水", Control::Drink, 120.0);
                s.button(row, "等候", Control::Wait, 132.0);
            });
        });
    commands
        .spawn((
            Name::new("Surveyed B1 map from the authored grid"),
            HudPart::Mini,
            sliced(s.glass.clone()),
            HudPart::Mini.node(),
            GlobalZIndex(10),
        ))
        .with_children(|p| {
            p.spawn((s.text("B1   北 ↑", 20.0, MUTED), at(14.0, 7.0, 158.0, 25.0)));
            for z in 0..map::HEIGHT {
                for x in 0..map::WIDTH {
                    let cell = map::MIN + IVec2::new(x as i32, z as i32);
                    p.spawn((
                        MapCell(cell),
                        BackgroundColor(Color::NONE),
                        at(14.0 + x as f32 * 7.0, 34.0 + z as f32 * 7.0, 6.0, 6.0),
                    ));
                }
            }
            p.spawn((
                s.text("青：你  金：出口", 18.0, MUTED),
                at(14.0, 140.0, 164.0, 22.0),
            ));
        });
    commands
        .spawn((
            Name::new("Field investigation paper"),
            HudPart::Field,
            sliced(assets.load("book/field-tag.png")),
            HudPart::Field.node(),
            GlobalZIndex(20),
        ))
        .with_children(|p| {
            p.spawn((
                s.text("", 24.0, INK),
                CopySlot::FieldHeading,
                at(24.0, 14.0, 410.0, 32.0),
            ));
            p.spawn((
                s.text("", 24.0, INK),
                CopySlot::FieldBody,
                at(24.0, 56.0, 410.0, 132.0),
            ));
            p.spawn(Node {
                column_gap: Val::Px(12.0),
                ..at(24.0, 194.0, 410.0, 48.0)
            })
            .with_children(|row| {
                s.button(row, "确认", Control::Confirm, 248.0);
                s.button(row, "取消", Control::Cancel, 140.0);
            });
        });
    commands
        .spawn((
            Name::new("Modal journal and expedition outcomes"),
            Panel::Folio,
            ImageNode::new(assets.load("book/folio.png")),
            Node {
                left: Val::Percent(50.0),
                margin: UiRect {
                    left: Val::Px(-310.0),
                    ..default()
                },
                display: Display::None,
                ..at(0.0, 136.0, 620.0, 650.0)
            },
            GlobalZIndex(30),
        ))
        .with_children(|p| {
            p.spawn((
                s.text("", 28.0, INK),
                CopySlot::Heading,
                at(64.0, 32.0, 420.0, 36.0),
            ));
            p.spawn(Node {
                ..at(500.0, 26.0, 88.0, 48.0)
            })
            .with_children(|b| s.button(b, "合上", Control::Cancel, 88.0));
            p.spawn((
                s.text("", 24.0, FADED),
                CopySlot::Summary,
                at(64.0, 98.0, 490.0, 108.0),
            ));
            p.spawn((
                Panel::Illustration,
                BreakfastArt,
                ImageNode::new(assets.load("book/breakfast.png")),
                at(64.0, 92.0, 490.0, 116.0),
            ));
            for (page, path) in [
                (Page::Barter, "book/tonic-plate.png"),
                (Page::Homecoming, "book/handwheel-plate.png"),
            ] {
                p.spawn((
                    Panel::Keepsake(page),
                    ImageNode::new(assets.load(path)),
                    at(468., 99., 83., 83.),
                ));
            }
            p.spawn((
                s.text("", 24.0, INK),
                CopySlot::Body,
                at(64.0, 226.0, 490.0, 336.0),
            ));
            p.spawn((
                Panel::GearChoices,
                Node {
                    flex_direction: FlexDirection::Column,
                    row_gap: Val::Px(14.0),
                    ..at(64.0, 220.0, 490.0, 270.0)
                },
            ))
            .with_children(|row| {
                s.choice(
                    row,
                    "圆盾 · 守住窄路\nQ 反击 2；F 盾击 2、打断\n让守卫先出手，你能还它一下",
                    Control::Buckler,
                    490.0,
                    126.0,
                );
                s.choice(
                    row,
                    "钩索 · 隔格先手\nF 两格钩击 2、拉近、打断\n磷蛾隔着一格也够得着",
                    Control::Hook,
                    490.0,
                    126.0,
                );
            });
            p.spawn((
                Panel::ModalControls,
                Node {
                    column_gap: Val::Px(12.0),
                    justify_content: JustifyContent::FlexEnd,
                    ..at(64.0, 574.0, 490.0, 48.0)
                },
            ))
            .with_children(|row| {
                row.spawn((
                    Panel::BowlChoices,
                    Node {
                        column_gap: Val::Px(8.0),
                        ..default()
                    },
                ))
                .with_children(|b| {
                    s.button(b, "少一颗", Control::Less, 108.0);
                    s.button(b, "多一颗", Control::More, 108.0);
                });
                row.spawn((Panel::ServedChoices, Node::default()))
                    .with_children(|b| {
                        s.button(b, "住到明天", Control::NewMorning, 212.0);
                    });
                s.button(row, "确认", Control::Confirm, 248.0);
            });
            p.spawn((
                Panel::JournalControls,
                Node {
                    justify_content: JustifyContent::SpaceBetween,
                    align_items: AlignItems::Center,
                    ..at(64.0, 574.0, 490.0, 48.0)
                },
            ))
            .with_children(|row| {
                s.button(row, "上一页", Control::Less, 126.0);
                row.spawn((s.text("", 24.0, FADED), CopySlot::Leaf));
                s.button(row, "下一页", Control::More, 126.0);
            });
        });
}

fn scale_book(window: Query<&Window, With<PrimaryWindow>>, mut scale: ResMut<UiScale>) {
    let Ok(window) = window.single() else { return };
    let value = (window.height() / 900.0).max(0.01);
    if scale.0 != value {
        scale.0 = value;
    }
}
fn click_book(
    mut buttons: Query<
        (&Interaction, &Control, &Available, &mut ImageNode),
        (With<Button>, Changed<Interaction>),
    >,
    mut intents: EventWriter<Intent>,
    ui: Res<Interface>,
) {
    for (interaction, action, available, mut image) in &mut buttons {
        let selected = ui.page == Page::Gear
            && matches!(
                (action, ui.filling),
                (Control::Buckler, 0) | (Control::Hook, 1)
            );
        image.color = if !available.0 {
            Color::srgb(0.42, 0.44, 0.40)
        } else {
            match interaction {
                Interaction::Hovered => Color::srgb(1.20, 1.16, 1.04),
                Interaction::Pressed => Color::srgb(0.75, 0.86, 0.78),
                Interaction::None if selected => Color::srgb(1.25, 1.16, 0.78),
                Interaction::None => Color::WHITE,
            }
        };
        if *interaction != Interaction::Pressed || !available.0 {
            continue;
        }
        intents.write(match action {
            Control::Context | Control::Confirm => Intent::Interact,
            Control::Toss => Intent::Toss,
            Control::Look => Intent::Look,
            Control::Book => Intent::Book,
            Control::Wait => Intent::Wait,
            Control::Brace => Intent::Brace,
            Control::Skill => Intent::Skill,
            Control::Drink => Intent::Drink,
            Control::Cancel => Intent::Cancel,
            Control::Less | Control::Buckler => Intent::Less,
            Control::More | Control::Hook => Intent::More,
            Control::NewMorning => Intent::NewMorning,
        });
    }
}
fn visible(origin: IVec2, destination: IVec2) -> bool {
    let delta = destination - origin;
    if delta.length_squared() > 20 {
        return false;
    }
    let steps = delta.x.abs().max(delta.y.abs());
    for step in 1..steps {
        let p = origin
            + (delta.as_vec2() * (step as f32 / steps as f32))
                .round()
                .as_ivec2();
        if map::glyph(Area::Cistern, p) == b'#' {
            return false;
        }
    }
    true
}
fn survey(outing: Res<Outing>, mut folio: ResMut<Folio>) {
    if outing.area != Area::Cistern || !outing.is_changed() {
        return;
    }
    for z in map::MIN.y..=map::MAX.y {
        for x in map::MIN.x..=map::MAX.x {
            let p = IVec2::new(x, z);
            if !folio.seen[map::cell_index(p)] && visible(outing.player, p) {
                folio.seen[map::cell_index(p)] = true;
            }
        }
    }
}
fn leaf_count(outing: &Outing) -> usize {
    3 + outing.history.len().div_ceil(2).max(1)
}
fn turn_leaf(
    mut events: EventReader<Intent>,
    ui: Res<Interface>,
    outing: Res<Outing>,
    mut folio: ResMut<Folio>,
) {
    for intent in events.read() {
        if ui.page != Page::Book {
            continue;
        }
        match intent {
            Intent::Less => folio.leaf = folio.leaf.saturating_sub(1),
            Intent::More => folio.leaf = (folio.leaf + 1).min(leaf_count(&outing) - 1),
            _ => {}
        }
    }
}
fn kit_name(kit: Kit) -> &'static str {
    match kit {
        Kit::None => "徒手",
        Kit::Buckler => "圆盾",
        Kit::Hook => "钩索",
    }
}
fn goal(o: &Outing) -> &'static str {
    match (o.area, o.expedition.quest) {
        (Area::Town, Quest::Seeking) => "北水门下井 · 寻回手轮",
        (Area::Cistern, Quest::Seeking) if o.expedition.kit == Kit::None => {
            "西侧选器物 · 东侧取手轮"
        }
        (Area::Cistern, Quest::Seeking) => "东侧取手轮 · 留意红格",
        (Area::Cistern, Quest::Carrying) => "手轮在身 · 回南侧阶梯",
        (Area::Town, Quest::Carrying) => "右侧泉台 · 安装青铜手轮",
        (Area::Town, Quest::Restored) => "泉水已归 · 自由行走",
        (Area::Cistern, Quest::Restored) => "收集铜片 · 南阶回镇",
    }
}
fn short(value: &str, max: usize) -> String {
    let mut chars = value.chars();
    let mut result: String = chars.by_ref().take(max).collect();
    if chars.next().is_some() {
        result.push('…');
    }
    result
}
fn tactical_notice(o: &Outing, folio: &Folio) -> String {
    if o.area == Area::Cistern {
        let mut result = String::new();
        for enemy in &o.expedition.enemies {
            if enemy.hp == 0 || !folio.seen[map::cell_index(enemy.pos)] {
                continue;
            }
            if let Some(p) = enemy.warn {
                let name = match enemy.kind {
                    EnemyKind::Sentry => "守卫",
                    EnemyKind::Moth => "飞蛾",
                };
                let _ = write!(result, "{}→({},{})  ", name, p.x, p.y);
            }
        }
        if !result.is_empty() {
            return format!(
                "下一拍：{}\n离开红格 / Q护卫\n喷口2伤，护卫无效",
                short(&result, 22)
            );
        }
        if o.danger_at(o.player) {
            return "脚下喷口下一拍涌出！\n离开红格；喷口造成2伤。\n护卫挡不住喷口。".to_owned();
        }
    }
    short(
        if o.notice.is_empty() {
            o.prompt()
        } else {
            o.notice
        },
        44,
    )
}
fn map_color(o: &Outing, folio: &Folio, p: IVec2) -> Color {
    if p == o.player {
        return Color::srgb(0.48, 1.0, 0.86);
    }
    if !folio.seen[map::cell_index(p)] {
        return Color::srgb(0.07, 0.11, 0.11);
    }
    if o.danger_at(p) {
        return Color::srgb(0.95, 0.36, 0.24);
    }
    match map::glyph(Area::Cistern, p) {
        b'#' => Color::srgb(0.22, 0.28, 0.27),
        b'~' => Color::srgb(0.16, 0.40, 0.45),
        b'<' => Color::srgb(0.96, 0.78, 0.37),
        b'C' if !o.expedition.seal_open => Color::srgb(0.72, 0.48, 0.25),
        b'!' if !o.expedition.tonic_open => Color::srgb(0.74, 0.73, 0.42),
        b'R' if o.expedition.quest == Quest::Seeking => Color::srgb(0.90, 0.59, 0.26),
        _ => Color::srgb(0.50, 0.57, 0.53),
    }
}
fn typeset_book(
    outing: Res<Outing>,
    ui: Res<Interface>,
    folio: Res<Folio>,
    mut texts: Query<(&CopySlot, &mut Text)>,
    mut panels: Query<
        (&mut Node, Option<&Panel>, Option<&HudPart>),
        (Or<(With<Panel>, With<HudPart>)>, Without<HpFill>),
    >,
    mut hp: Query<(&mut Node, &mut BackgroundColor), (With<HpFill>, Without<Panel>)>,
    mut cells: Query<(&MapCell, &mut BackgroundColor), Without<HpFill>>,
    illustrations: Res<Illustrations>,
    mut illustration: Query<&mut ImageNode, (With<BreakfastArt>, Without<Button>)>,
    mut controls: Query<
        (&Control, &mut Available, &mut ImageNode),
        (With<Button>, Without<BreakfastArt>),
    >,
) {
    if !outing.is_changed() && !ui.is_changed() && !folio.is_changed() {
        return;
    }
    let o = &*outing;
    let ex = &o.expedition;
    for (mut node, panel, part) in &mut panels {
        let show = if let Some(part) = part {
            part.visible(ui.page, ui.hud, o.area)
        } else if let Some(panel) = panel {
            match panel {
                Panel::Folio => modal(ui.page),
                Panel::JournalControls => ui.page == Page::Book,
                Panel::ModalControls => ui.page != Page::Book,
                Panel::BowlChoices => ui.page == Page::Bowl,
                Panel::GearChoices => ui.page == Page::Gear,
                Panel::ServedChoices => ui.page == Page::Served,
                Panel::Illustration => matches!(ui.page, Page::Bowl | Page::Served),
                Panel::Keepsake(page) => *page == ui.page,
            }
        } else {
            false
        };
        node.display = if show { Display::Flex } else { Display::None };
    }
    for (mut node, mut color) in &mut hp {
        node.width = Val::Px(244.0 * ex.hp as f32 / ex.max_hp.max(1) as f32);
        color.0 = if ex.hp <= 3 {
            Color::srgb(0.86, 0.35, 0.25)
        } else {
            Color::srgb(0.42, 0.73, 0.60)
        };
    }
    if o.area == Area::Cistern {
        for (MapCell(p), mut color) in &mut cells {
            color.0 = map_color(o, &folio, *p);
        }
    }
    let fruit = if ui.page == Page::Bowl {
        ui.filling > 0
    } else {
        o.breakfast.unwrap_or(0) > 0
    };
    for mut image in &mut illustration {
        image.image = if fruit {
            illustrations.fruit.clone()
        } else {
            illustrations.plain.clone()
        };
    }
    for (control, mut available, mut image) in &mut controls {
        let roam = ui.page == Page::Roam;
        let yes = match control {
            Control::Toss => roam && o.area == Area::Town && o.plums > 0,
            Control::Look | Control::Wait | Control::Context => roam,
            Control::Brace => roam && o.area == Area::Cistern,
            Control::Skill => {
                roam && o.area == Area::Cistern && ex.kit != Kit::None && ex.skill_ready
            }
            Control::Drink => roam && ex.tonics > 0 && ex.hp < ex.max_hp,
            Control::Book => !matches!(ui.page, Page::Down | Page::Gear),
            Control::Cancel => ui.page != Page::Down,
            Control::Less => match ui.page {
                Page::Book => folio.leaf > 0,
                Page::Bowl => ui.filling > 0,
                _ => false,
            },
            Control::More => match ui.page {
                Page::Book => folio.leaf + 1 < leaf_count(o),
                Page::Bowl => ui.filling < o.plums,
                _ => false,
            },
            Control::Buckler | Control::Hook => ui.page == Page::Gear,
            Control::Confirm => match ui.page {
                Page::Aim => o.can_toss(ui.cursor),
                Page::Barter => o.plums > 0 && ex.tonics < 3,
                Page::Look
                | Page::Bowl
                | Page::Served
                | Page::Gear
                | Page::Down
                | Page::Homecoming => true,
                _ => false,
            },
            Control::NewMorning => ui.page == Page::Served,
        };
        let selected = ui.page == Page::Gear
            && matches!(
                (control, ui.filling),
                (Control::Buckler, 0) | (Control::Hook, 1)
            );
        if available.0 != yes || matches!(control, Control::Buckler | Control::Hook) {
            available.0 = yes;
            image.color = if selected {
                Color::srgb(1.25, 1.16, 0.78)
            } else if yes {
                Color::WHITE
            } else {
                Color::srgb(0.42, 0.44, 0.40)
            };
        }
    }
    for (slot, mut text) in &mut texts {
        let value = match slot {
            CopySlot::Place => format!(
                "{}  ·  第{}天",
                if o.area == Area::Town {
                    "铜壶镇"
                } else {
                    "旧泉回水廊 B1"
                },
                o.day
            ),
            CopySlot::Objective => goal(o).to_owned(),
            CopySlot::Vitals => format!("HP {}/{}  药×{}", ex.hp, ex.max_hp, ex.tonics),
            CopySlot::Notice => tactical_notice(o, &folio),
            CopySlot::Context => format!(
                "E  {}",
                short(
                    o.prompt()
                        .trim_start_matches("E ")
                        .split(['·', '/'])
                        .next()
                        .unwrap_or("")
                        .trim(),
                    5
                )
            ),
            CopySlot::Skill => match ex.kit {
                Kit::None => "F 无器物",
                Kit::Buckler if ex.skill_ready => "F 盾击",
                Kit::Hook if ex.skill_ready => "F 钩击",
                _ => "F 待蓄势",
            }
            .to_owned(),
            CopySlot::Heading => match ui.page {
                Page::Bowl => "铜壶旅店 · 早餐",
                Page::Served => "把这个早晨记下来",
                Page::Gear => "工具箱 · 选一套器物",
                Page::Barter => "泉边药师 · 行囊补给",
                Page::Down => "灯火接你回家",
                Page::Homecoming => "第一股泉水",
                Page::Book => match folio.leaf {
                    0 => "旅行手记 · 出征",
                    1 => "器物与操作",
                    2 => "铜壶镇 · 地点",
                    _ => "留在纸上的早晨",
                },
                _ => "",
            }
            .to_owned(),
            CopySlot::Summary => summary(o, &ui, folio.leaf),
            CopySlot::Body => body(o, &ui, folio.leaf),
            CopySlot::FieldHeading => format!(
                "{}  ({}, {})",
                if ui.page == Page::Aim {
                    "投果落点"
                } else {
                    "调查格子"
                },
                ui.cursor.x,
                ui.cursor.y
            ),
            CopySlot::FieldBody => {
                if ui.page == Page::Aim {
                    format!(
                        "手里{}颗，三步内可越墙。\n点地面选落点，再点投出。\n合上纸卡会放回口袋。",
                        o.plums
                    )
                } else {
                    investigation(o, ui.cursor)
                }
            }
            CopySlot::Leaf => format!("{} / {}", folio.leaf + 1, leaf_count(o)),
            CopySlot::Close => if matches!(ui.page, Page::Aim | Page::Look | Page::Gear) {
                "取消"
            } else {
                "合上"
            }
            .to_owned(),
            CopySlot::Confirm => match ui.page {
                Page::Aim => "E  投出一颗",
                Page::Look => "E  看好了",
                Page::Bowl => "E  端上早餐",
                Page::Barter => "E  换一瓶药",
                Page::Served => "E  继续探索",
                Page::Gear if ui.filling == 0 => "E  带上圆盾",
                Page::Gear => "E  带上钩索",
                Page::Down => "E  回到铜壶镇",
                Page::Homecoming => "E  继续旅行",
                _ => "确认",
            }
            .to_owned(),
        };
        if text.0 != value {
            text.0 = value;
        }
    }
}
fn investigation(o: &Outing, p: IVec2) -> String {
    if o.area == Area::Cistern {
        if let Some(enemy) = o.expedition.enemies.iter().find(|e| e.hp > 0 && e.pos == p) {
            let name = match enemy.kind {
                EnemyKind::Sentry => "回水廊守卫",
                EnemyKind::Moth => "铜翅飞蛾",
            };
            return format!(
                "{} · HP {}\n{}\n普攻1伤；器物技可打断。",
                name,
                enemy.hp,
                if let Some(w) = enemy.warn {
                    format!("下拍击向 ({}, {})，快离开。", w.x, w.y)
                } else {
                    "还未锁定落击点。".to_owned()
                }
            );
        }
        if o.danger_at(p) {
            return format!(
                "{}\n下一行动这格将受击。\n离开红格；喷口无视护卫。",
                short(o.describe(p), 28)
            );
        }
    }
    short(o.describe(p), 68)
}
fn summary(o: &Outing, ui: &Interface, leaf: usize) -> String {
    let ex = &o.expedition;
    match ui.page {
        Page::Gear => format!("旧箱子里躺着一面圆盾，\n旁边是盘好的绳钩。\n挑一样带走：{}", if ui.filling == 0 { "圆盾" } else { "钩索" }),
        Page::Barter => "「给我一颗熟李子。\n我有给走夜路的人喝的药。」\n—— 泉边药师".to_owned(),
        Page::Down => "回程的灯提到了你身边。\n这一次，先回镇里喘口气。\n你仍可以再进北水门。".to_owned(),
        Page::Homecoming => "起初只有几滴水，带着铁锈。\n接着，池边空了一早上的水桶\n一只一只接满了。".to_owned(),
        Page::Book => match leaf {
            0 => format!("{}\n{}", if o.area == Area::Town { "所在：铜壶镇" } else { "所在：旧泉回水廊 B1" }, goal(o)),
            1 => format!("装备：{}   药水：{}瓶\n器物技：{}\n普攻、移动、护卫和等候都走一拍。", kit_name(ex.kit), ex.tonics, if ex.skill_ready { "已蓄势" } else { "待蓄势，Q或等候恢复" }),
            2 => "上北下南，东西沿石路走。\n点远处地面会沿可走路线前行。\n靠近物件，点互动或直接点它。".to_owned(),
            _ => format!("第{}天 · 旅店门廊\n包里有{}颗李子。\n早餐与分享都留在这里。", o.day, o.plums),
        },
        _ => String::new(),
    }
}
fn body(o: &Outing, ui: &Interface, leaf: usize) -> String {
    let ex = &o.expedition;
    match ui.page {
        Page::Barter => format!("1颗熟李子 → 1瓶药水\n\n包里熟李子：{}颗\n药囊：{} / 3瓶\n\n{}",
            o.plums, ex.tonics,
            if ex.tonics >= 3 { "药囊已经装满了。\n先在冒险中用掉一瓶，再来换吧。" }
            else if o.plums == 0 { "还缺一颗熟李子。\n旅店旁可以拾果，东边李树也可摇。" }
            else { "点铜扣，把果子交给药师。\n药水在冒险中可恢复4点生命。" }),
        Page::Bowl => format!("{}\n\n手里{}颗，这碗用{}颗。\n吃完还留{}颗。\n\n点「少一颗 / 多一颗」调配。\n空手也有一碗原味的。\n吃完可以继续今天的冒险。",
            if ui.filling == 0 { "原味白凝乳".to_owned() } else { format!("白凝乳，加{}颗李子", ui.filling) },
            o.plums, ui.filling, o.plums.saturating_sub(ui.filling)),
        Page::Served => format!("{}\n\n这碗用了{}颗李子。\n包里留着{}颗。\n小家伙今天吃了{}颗。\n\n「继续探索」回到今天。\n「住到明天」开启新的一天。",
            if o.breakfast.unwrap_or(0) == 0 { "白凝乳，入口微酸。" } else { "碗沿挂着紫红的果汁。" },
            o.breakfast.unwrap_or(0), o.plums, o.eaten),
        Page::Down => format!("这是第{}次被抬回旅店。\n生命 {}/{} · 已存铜片 {}\n\n这趟散币和没带上台阶的手轮掉了。\n寄存在镇里的东西都还在。\n\n店主替你收着装备。\n歇好以后，可以再去北门。", ex.deaths, ex.hp, ex.max_hp, ex.banked),
        Page::Homecoming => format!("池底的灰先浮了起来。店主等它流清，\n把一直搁在台阶上的铜壶递了过去。\n\n铜壶镇的泉水重新流了。\n\n出征 {} 次 · 此行击退 {} 名敌人\n已存铜片 {} · 倒下 {} 次\n带回的器物：{}\n\n手记已记下。现在还可以再走走。", ex.attempts, ex.kills, ex.banked, ex.deaths, kit_name(ex.kit)),
        Page::Book => match leaf {
            0 => format!("泉水：{}\n手轮：{}\n\n生命 {}/{} · 药水{}瓶\n装备：{}\n本趟铜片{} · 已存{}\n出征{}次 · 倒下{}次 · 撤回{}次\n\n北水门在镇北；返程阶梯在B1南侧。\n手轮带回后，走近右侧泉台安装。",
                if ex.quest == Quest::Restored { "已经恢复" } else { "仍待修复" },
                match ex.quest { Quest::Seeking => "尚在回水廊东侧", Quest::Carrying => "已经带在身上", Quest::Restored => "已装回泉台" },
                ex.hp, ex.max_hp, ex.tonics, kit_name(ex.kit), ex.loot, ex.banked, ex.attempts, ex.deaths, ex.retreats),
            1 => format!("点地面 / WASD：移动，点邻敌普攻1。\nQ 护卫：全挡敌击，邻格反击1。\n圆盾反击2；喷口2伤无视护卫。\nF {}。\n器物技打断敌人；Q或等候后恢复。\nR 药水：恢复4HP，满血时不消耗。\n\n红格＝下一行动将落下的攻击。\nV 调查、J 手记不走拍；T 投李子。\n滚轮缩放；Home恢复俯视。\n所有常用行动也可点下方按钮。",
                match ex.kit { Kit::None => "未带器物，先去西侧工具箱", Kit::Hook => "钩击：正面两格2伤，拉近", Kit::Buckler => "盾击：正面邻格2伤" }),
            2 => "镇北 · 北水门\n穿过拱门，下到旧泉回水廊。\n\nB1 西侧 · 工具箱与药箱\n挑选圆盾或钩索，带上回程的药。\n\nB1 东侧 · 青铜手轮\n拿到后，从南侧阶梯回镇。\n\n镇东 · 泉台；镇西 · 旅店门廊\n归还手轮；早餐仍在壶下等着。".to_owned(),
            _ => {
                if o.history.is_empty() {
                    return "纸上还没有早餐的记录。\n\n回旅店门廊，坐下来吃一碗。\n原味也是值得记住的早晨。\n\n采李子、投果分享、吃早餐，\n都可以留到冒险回来再慢慢做。".to_owned();
                }
                let mut result = String::new();
                for meal in o.history.iter().rev().skip((leaf - 3) * 2).take(2) {
                    let _ = write!(result, "第{}天 · {}\n早餐用{}颗，留下{}颗。\n小家伙吃了{}颗。\n\n",
                        meal.day, if meal.used == 0 { "原味凝乳" } else { "李子凝乳" }, meal.used, meal.kept, meal.shared);
                }
                result.trim_end().to_owned()
            }
        },
        _ => String::new(),
    }
}
