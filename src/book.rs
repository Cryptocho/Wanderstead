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
            .init_resource::<BookTypography>()
            .add_systems(Startup, bind_book)
            .add_systems(Update, (scale_book, click_book).in_set(crate::Frame::Input))
            .add_systems(
                Update,
                (survey, turn_leaf, typeset_book, describe_control)
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

/// Native 1440p: HUD at two physical pixels per 12px font pixel; reading at 30px.
/// Override this resource before Startup to tune the whole stationery family.
#[derive(Resource)]
pub struct BookTypography {
    pub hud: f32,
    pub reading: f32,
    pub heading: f32,
}
impl Default for BookTypography {
    fn default() -> Self {
        Self {
            hud: 15.0,
            reading: 18.75,
            heading: 22.5,
        }
    }
}

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
/// Dimensions are in window-height / 900 units, independent of desktop DPI.
#[derive(Component, Clone, Copy)]
enum HudPart {
    Place,
    Compass,
    Guide,
    Journal,
    Vitals,
    Notice,
    LeftActions,
    TownActions,
    DelveActions,
    Mini,
    Field,
}
const HUD_PARTS: [HudPart; 11] = [
    HudPart::Compass,
    HudPart::Guide,
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
            Self::Place => [12.0, 10.0, 700.0, 24.0],
            Self::Compass => [12.0, 38.0, 180.0, 58.0],
            Self::Journal => [458.0, 850.0, 100.0, 36.0],
            Self::Vitals => [180.0, 812.0, 310.0, 32.0],
            Self::Guide => [502.0, 812.0, width - 516.0, 32.0],
            Self::Notice => [180.0, 758.0, width - 194.0, 46.0],
            Self::LeftActions => [180.0, 850.0, 270.0, 36.0],
            Self::TownActions => [566.0, 850.0, 218.0, 36.0],
            Self::DelveActions => [566.0, 850.0, 400.0, 36.0],
            Self::Mini => [12.0, 746.0, 158.0, 140.0],
            Self::Field => [180.0, 536.0, 500.0, 206.0],
        }
    }
    fn node(self) -> Node {
        let [x, y, width, height] = self.rect(1600.0);
        let mut node = at(x, y, width, height);
        if matches!(self, Self::Notice) {
            // Shrink to real typeset line width and grow upward for the second line.
            node.width = Val::Auto;
            node.height = Val::Auto;
            node.min_width = Val::Px(146.);
            node.max_width = Val::Percent(84.);
            node.top = Val::Auto;
            node.bottom = Val::Px(96.);
        } else if matches!(self, Self::Guide) {
            node.width = Val::Auto;
            node.right = Val::Px(14.0);
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
        // Painted captions are click-through, including their transparent margin.
        if matches!(
            part,
            HudPart::Place | HudPart::Compass | HudPart::Guide | HudPart::Notice
        ) {
            return false;
        }
        let [x, y, w, h] = part.rect(width);
        part.visible(page, hud, area) && p.x >= x && p.x <= x + w && p.y >= y && p.y <= y + h
    })
}

#[derive(Resource)]
struct Folio {
    leaf: usize,
    seen: [[bool; CELLS]; 2],
    recent: std::collections::VecDeque<&'static str>,
}
impl Default for Folio {
    fn default() -> Self {
        Self {
            leaf: 0,
            seen: [[false; CELLS]; 2],
            recent: std::collections::VecDeque::with_capacity(2),
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
    MapHeading,
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
#[derive(Component)]
struct ControlGuide;
#[derive(Component)]
struct KitIcon;
#[derive(Resource)]
struct Illustrations {
    plain: Handle<Image>,
    fruit: Handle<Image>,
    buckler_icon: Handle<Image>,
    hook_icon: Handle<Image>,
}

struct Stationery {
    font: Handle<Font>,
    button: Handle<Image>,
    glass: Handle<Image>,
    buckler: Handle<Image>,
    hook: Handle<Image>,
    icons: [Handle<Image>; 8],
    hud: f32,
    reading: f32,
    heading: f32,
}
impl Stationery {
    fn text(&self, value: &str, size: f32, color: Color) -> impl Bundle + use<> {
        (
            Text::new(value),
            TextFont {
                font: self.font.clone(),
                font_size: size,
                font_smoothing: FontSmoothing::None,
                line_height: LineHeight::Px(size * 1.3),
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
        self.choice(parent, caption, action, width, 36.0);
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
                    padding: UiRect::horizontal(Val::Px(6.0)),
                    column_gap: Val::Px(5.0),
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
                    b.spawn((ImageNode::new(icon.clone()), at(16., 16., 76., 76.)));
                    b.spawn((
                        self.text(caption, self.reading, IVORY),
                        at(108., 16., width - 124., 84.),
                    ));
                    return;
                }
                if let Some(index) = match action {
                    Control::Context => Some(0),
                    Control::Look => Some(1),
                    Control::Book => Some(2),
                    Control::Toss => Some(3),
                    Control::Wait => Some(4),
                    Control::Brace => Some(5),
                    Control::Skill => Some(6),
                    Control::Drink => Some(7),
                    _ => None,
                } {
                    let mut icon = b.spawn((
                        ImageNode::new(self.icons[index].clone()),
                        Node {
                            width: Val::Px(24.0),
                            height: Val::Px(24.0),
                            flex_shrink: 0.0,
                            ..default()
                        },
                    ));
                    if matches!(action, Control::Skill) {
                        icon.insert(KitIcon);
                    }
                }
                let size = if height > 36.0 {
                    self.reading
                } else {
                    self.hud
                };
                let mut text = b.spawn(self.text(caption, size, IVORY));
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
fn bind_book(mut commands: Commands, assets: Res<AssetServer>, typeface: Res<BookTypography>) {
    let s = Stationery {
        font: assets.load("book/FusionPixel.ttf"),
        button: assets.load("book/expedition-button.png"),
        glass: assets.load("book/expedition-glass.png"),
        buckler: assets.load("book/buckler-plate.png"),
        hook: assets.load("book/hook-plate.png"),
        icons: [
            "context", "look", "journal", "fruit", "wait", "brace", "skill", "drink",
        ]
        .map(|name| assets.load(format!("book/icon-{name}.png"))),
        hud: typeface.hud,
        reading: typeface.reading,
        heading: typeface.heading,
    };
    commands.insert_resource(Illustrations {
        plain: assets.load("book/plain-breakfast.png"),
        fruit: assets.load("book/breakfast.png"),
        buckler_icon: assets.load("book/icon-brace.png"),
        hook_icon: assets.load("book/icon-hook.png"),
    });
    commands
        .spawn((
            Name::new("Day, place and current goal"),
            HudPart::Place,
            HudPart::Place.node(),
            GlobalZIndex(10),
        ))
        .with_children(|p| {
            p.spawn((
                s.text("", s.hud, IVORY),
                CopySlot::Place,
                hud_shadow(),
                at(0., 0., 700., 24.),
            ));
        });
    commands
        .spawn((
            Name::new("North-locked compass and actual grid position"),
            HudPart::Compass,
            HudPart::Compass.node(),
            GlobalZIndex(10),
        ))
        .with_children(|p| {
            p.spawn((
                ImageNode::new(assets.load("book/compass.png")),
                at(0., 10., 48., 48.),
            ));
            p.spawn((
                s.text("N", s.hud, IVORY),
                hud_shadow(),
                at(20., 0., 20., 16.),
            ));
            p.spawn((
                s.text("", s.hud, IVORY),
                CopySlot::MapHeading,
                hud_shadow(),
                at(54., 28., 120., 20.),
            ));
        });
    commands
        .spawn((
            Name::new("Travel journal clasp"),
            HudPart::Journal,
            HudPart::Journal.node(),
            GlobalZIndex(10),
        ))
        .with_children(|p| s.button(p, "J 手记", Control::Book, 100.));
    commands
        .spawn((
            Name::new("Actual player health and carried resources"),
            HudPart::Vitals,
            sliced(s.glass.clone()),
            HudPart::Vitals.node(),
            GlobalZIndex(10),
        ))
        .with_children(|p| {
            p.spawn((
                ImageNode::new(assets.load("figures/traveler-portrait.png")),
                at(4., 0., 32., 32.),
            ));
            p.spawn((
                s.text("", s.hud, IVORY),
                CopySlot::Vitals,
                at(42., 1., 262., 20.),
            ));
            p.spawn((
                BackgroundColor(Color::srgb(0.16, 0.13, 0.10)),
                at(42., 25., 260., 4.),
            ));
            p.spawn((
                HpFill,
                BackgroundColor(Color::srgb(0.42, 0.73, 0.60)),
                at(42., 25., 260., 4.),
            ));
        });
    commands
        .spawn((
            Name::new("Action explanations and keyboard guide"),
            HudPart::Guide,
            HudPart::Guide.node(),
            GlobalZIndex(10),
        ))
        .with_children(|p| {
            p.spawn((
                s.text("", s.hud, IVORY),
                hud_shadow(),
                ControlGuide,
                Node {
                    width: Val::Percent(100.),
                    ..default()
                },
            ));
        });
    commands
        .spawn((
            Name::new("Resolved action and explicit next-turn danger"),
            HudPart::Notice,
            sliced(s.glass.clone()),
            HudPart::Notice.node(),
            GlobalZIndex(10),
        ))
        .with_children(|p| {
            p.spawn((
                s.text("", s.hud, IVORY),
                CopySlot::Notice,
                Node {
                    margin: UiRect::axes(Val::Px(10.), Val::Px(5.)),
                    ..default()
                },
            ));
        });
    commands
        .spawn((
            Name::new("Context and investigation"),
            HudPart::LeftActions,
            HudPart::LeftActions.node(),
            GlobalZIndex(10),
        ))
        .with_children(|p| {
            s.button(p, "", Control::Context, 160.);
            s.button(p, "V 调查", Control::Look, 102.);
        });
    commands
        .spawn((
            Name::new("Town fruit and wait"),
            HudPart::TownActions,
            HudPart::TownActions.node(),
            GlobalZIndex(10),
        ))
        .with_children(|p| {
            s.button(p, "T 投果", Control::Toss, 102.);
            s.button(p, "空格 等候", Control::Wait, 116.);
        });
    commands
        .spawn((
            Name::new("Guard, equipped skill, tonic and wait"),
            HudPart::DelveActions,
            HudPart::DelveActions.node(),
            GlobalZIndex(10),
        ))
        .with_children(|p| {
            s.button(p, "Q 护卫", Control::Brace, 92.);
            s.button(p, "", Control::Skill, 100.);
            s.button(p, "R 药水", Control::Drink, 92.);
            s.button(p, "空格 等候", Control::Wait, 116.);
        });
    commands
        .spawn((
            Name::new("Explored authored map, town and B1"),
            HudPart::Mini,
            sliced(s.glass.clone()),
            HudPart::Mini.node(),
            GlobalZIndex(10),
        ))
        .with_children(|p| {
            p.spawn((
                s.text("青：你  金：出入口", s.hud, MUTED),
                at(8., 5., 144., 20.),
            ));
            for z in 0..map::HEIGHT {
                for x in 0..map::WIDTH {
                    let cell = map::MIN + IVec2::new(x as i32, z as i32);
                    p.spawn((
                        MapCell(cell),
                        BackgroundColor(Color::NONE),
                        at(16. + x as f32 * 6., 28. + z as f32 * 6., 5., 5.),
                    ));
                }
            }
            p.spawn((
                s.text("走过的地方留在图上", s.hud, MUTED),
                at(8., 121., 144., 17.),
            ));
        });
    commands
        .spawn((
            Name::new("Investigation and throw confirmation"),
            HudPart::Field,
            sliced(assets.load("book/field-tag.png")),
            HudPart::Field.node(),
            GlobalZIndex(20),
        ))
        .with_children(|p| {
            p.spawn((
                s.text("", s.reading, INK),
                CopySlot::FieldHeading,
                at(24., 14., 452., 28.),
            ));
            p.spawn((
                s.text("", s.reading, INK),
                CopySlot::FieldBody,
                at(24., 51., 452., 96.),
            ));
            p.spawn(Node {
                column_gap: Val::Px(12.),
                ..at(24., 156., 452., 36.)
            })
            .with_children(|row| {
                s.button(row, "确认", Control::Confirm, 248.);
                s.button(row, "Esc 取消", Control::Cancel, 160.);
            });
        });
    commands
        .spawn((
            Name::new("Bound modal reading pages"),
            Panel::Folio,
            ImageNode::new(assets.load("book/folio.png")),
            Node {
                left: Val::Percent(50.),
                margin: UiRect {
                    left: Val::Px(-380.),
                    ..default()
                },
                display: Display::None,
                ..at(0., 96., 760., 700.)
            },
            GlobalZIndex(30),
        ))
        .with_children(|p| {
            p.spawn((
                s.text("", s.heading, INK),
                CopySlot::Heading,
                at(64., 32., 520., 36.),
            ));
            p.spawn(Node {
                ..at(610., 28., 104., 36.)
            })
            .with_children(|b| s.button(b, "Esc 合上", Control::Cancel, 104.));
            p.spawn((
                s.text("", s.reading, FADED),
                CopySlot::Summary,
                at(64., 92., 632., 84.),
            ));
            p.spawn((
                Panel::Illustration,
                BreakfastArt,
                ImageNode::new(assets.load("book/breakfast.png")),
                at(64., 90., 632., 100.),
            ));
            for (page, path) in [
                (Page::Barter, "book/tonic-plate.png"),
                (Page::Homecoming, "book/handwheel-plate.png"),
            ] {
                p.spawn((
                    Panel::Keepsake(page),
                    ImageNode::new(assets.load(path)),
                    at(606., 94., 76., 76.),
                ));
            }
            p.spawn((
                s.text("", s.reading, INK),
                CopySlot::Body,
                at(64., 206., 632., 404.),
            ));
            p.spawn((
                Panel::GearChoices,
                Node {
                    flex_direction: FlexDirection::Column,
                    row_gap: Val::Px(16.),
                    ..at(64., 216., 632., 240.)
                },
            ))
            .with_children(|row| {
                s.choice(
                    row,
                    "圆盾 · 守住窄路\nQ 反击 2；F 盾击 2、打断\n让守卫先出手，你能还它一下",
                    Control::Buckler,
                    632.,
                    108.,
                );
                s.choice(
                    row,
                    "钩索 · 隔格先手\nF 两格钩击 2、拉近、打断\n磷蛾隔着一格也够得着",
                    Control::Hook,
                    632.,
                    108.,
                );
            });
            p.spawn((
                Panel::ModalControls,
                Node {
                    column_gap: Val::Px(12.),
                    justify_content: JustifyContent::FlexEnd,
                    ..at(64., 630., 632., 44.)
                },
            ))
            .with_children(|row| {
                row.spawn((
                    Panel::BowlChoices,
                    Node {
                        column_gap: Val::Px(8.),
                        ..default()
                    },
                ))
                .with_children(|b| {
                    s.choice(b, "← 少一颗", Control::Less, 132., 44.);
                    s.choice(b, "→ 多一颗", Control::More, 132., 44.);
                });
                row.spawn((Panel::ServedChoices, Node::default()))
                    .with_children(|b| {
                        s.choice(b, "住到明天", Control::NewMorning, 212., 44.);
                    });
                s.choice(row, "确认", Control::Confirm, 248., 44.);
            });
            p.spawn((
                Panel::JournalControls,
                Node {
                    justify_content: JustifyContent::SpaceBetween,
                    align_items: AlignItems::Center,
                    ..at(64., 630., 632., 44.)
                },
            ))
            .with_children(|row| {
                s.choice(row, "← 上一页", Control::Less, 144., 44.);
                row.spawn((s.text("", s.reading, FADED), CopySlot::Leaf));
                s.choice(row, "→ 下一页", Control::More, 144., 44.);
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
fn visible(area: Area, origin: IVec2, destination: IVec2) -> bool {
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
        if map::glyph(area, p) == b'#' {
            return false;
        }
    }
    true
}
fn survey(outing: Res<Outing>, mut folio: ResMut<Folio>) {
    if !outing.is_changed() {
        return;
    }
    if !outing.notice.is_empty() && folio.recent.front().copied() != Some(outing.notice) {
        if folio.recent.len() == 2 {
            folio.recent.pop_back();
        }
        folio.recent.push_front(outing.notice);
    }
    let layer = area_index(outing.area);
    for z in map::MIN.y..=map::MAX.y {
        for x in map::MIN.x..=map::MAX.x {
            let p = IVec2::new(x, z);
            if !folio.seen[layer][map::cell_index(p)] && visible(outing.area, outing.player, p) {
                folio.seen[layer][map::cell_index(p)] = true;
            }
        }
    }
}
fn area_index(area: Area) -> usize {
    match area {
        Area::Town => 0,
        Area::Cistern => 1,
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
            if enemy.hp == 0 || !folio.seen[1][map::cell_index(enemy.pos)] {
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
                "危险 · 下一拍 {}  离开红格 / Q护卫；喷口2伤无视护卫。\n{}",
                short(&result, 40),
                short(o.notice, 80)
            );
        }
        if o.danger_at(o.player) {
            return "危险 · 脚下喷口下一拍涌出！离开红格，喷口2伤无视护卫。".to_owned();
        }
    }
    if folio.recent.is_empty() {
        o.prompt().to_owned()
    } else {
        let mut lines = folio.recent.iter();
        let latest = short(lines.next().copied().unwrap_or(""), 84);
        match lines.next() {
            Some(previous) => format!("{}\n{}", latest, short(previous, 84)),
            None => latest,
        }
    }
}
fn map_color(o: &Outing, folio: &Folio, p: IVec2) -> Color {
    if p == o.player {
        return Color::srgb(0.48, 1.0, 0.86);
    }
    if !folio.seen[area_index(o.area)][map::cell_index(p)] {
        return Color::srgb(0.07, 0.11, 0.11);
    }
    if o.area == Area::Cistern && o.danger_at(p) {
        return Color::srgb(0.95, 0.36, 0.24);
    }
    match map::glyph(o.area, p) {
        b'#' => Color::srgb(0.24, 0.23, 0.20),
        b'~' => Color::srgb(0.22, 0.49, 0.57),
        b'<' | b'>' => Color::srgb(0.96, 0.78, 0.37),
        b'g' => Color::srgb(0.40, 0.51, 0.32),
        b'@' | b'P' => Color::srgb(0.86, 0.67, 0.41),
        b'C' if !o.expedition.seal_open => Color::srgb(0.72, 0.48, 0.25),
        b'!' if !o.expedition.tonic_open => Color::srgb(0.74, 0.73, 0.42),
        b'R' if o.expedition.quest == Quest::Seeking => Color::srgb(0.90, 0.59, 0.26),
        _ => Color::srgb(0.50, 0.57, 0.53),
    }
}
fn hud_shadow() -> TextShadow {
    TextShadow {
        offset: Vec2::new(1.0, 1.0),
        color: Color::srgba(0.08, 0.06, 0.04, 0.95),
    }
}
fn typeset_book(
    outing: Res<Outing>,
    ui: Res<Interface>,
    folio: Res<Folio>,
    mut texts: Query<(&CopySlot, &mut Text, &mut TextColor)>,
    mut panels: Query<
        (&mut Node, Option<&Panel>, Option<&HudPart>),
        (Or<(With<Panel>, With<HudPart>)>, Without<HpFill>),
    >,
    mut hp: Query<(&mut Node, &mut BackgroundColor), (With<HpFill>, Without<Panel>)>,
    mut cells: Query<(&MapCell, &mut BackgroundColor), Without<HpFill>>,
    illustrations: Res<Illustrations>,
    mut illustration: Query<&mut ImageNode, (With<BreakfastArt>, Without<Button>)>,
    mut kit_icons: Query<&mut ImageNode, (With<KitIcon>, Without<BreakfastArt>, Without<Button>)>,
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
        node.width = Val::Px(260.0 * ex.hp as f32 / ex.max_hp.max(1) as f32);
        color.0 = if ex.hp <= 3 {
            Color::srgb(0.86, 0.35, 0.25)
        } else {
            Color::srgb(0.42, 0.73, 0.60)
        };
    }
    for (MapCell(p), mut color) in &mut cells {
        color.0 = map_color(o, &folio, *p);
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
    for mut icon in &mut kit_icons {
        icon.image = if ex.kit == Kit::Hook {
            illustrations.hook_icon.clone()
        } else {
            illustrations.buckler_icon.clone()
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
    for (slot, mut text, mut color) in &mut texts {
        let value = match slot {
            CopySlot::Place => format!(
                "第{}天  {}  ·  {}",
                o.day,
                if o.area == Area::Town {
                    "铜壶镇"
                } else {
                    "旧泉回水廊 B1"
                },
                goal(o)
            ),
            CopySlot::MapHeading => format!(
                "{} ({},{})",
                if o.area == Area::Town { "镇" } else { "B1" },
                o.player.x,
                o.player.y
            ),
            CopySlot::Vitals => format!(
                "HP {}/{}  药{}  果{}  铜{}",
                ex.hp, ex.max_hp, ex.tonics, o.plums, ex.loot
            ),
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
                        "手里{}颗，八向三格内，可越矮墙。\n树、房屋与紧角挡路。\n点地面选落点，再点投出。\nEsc 收回；无效落点的铜扣变暗。",
                        o.plums
                    )
                } else {
                    investigation(o, ui.cursor)
                }
            }
            CopySlot::Leaf => format!("{} / {}", folio.leaf + 1, leaf_count(o)),
            CopySlot::Close => if matches!(ui.page, Page::Aim | Page::Look | Page::Gear) {
                "Esc 取消"
            } else {
                "Esc 合上"
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
        if matches!(slot, CopySlot::Notice) {
            color.0 = if value.starts_with("危险") {
                Color::srgb(1.0, 0.66, 0.43)
            } else {
                IVORY
            };
        }
        if text.0 != value {
            text.0 = value;
        }
    }
}

/// Hover copy lives inside Guide's existing bounds; it cannot cover a target tile.
/// Disabled controls explain their requirements as well as active controls.
fn describe_control(
    controls: Query<(&Interaction, &Control, &Available), With<Button>>,
    mut guides: Query<&mut Text, With<ControlGuide>>,
    outing: Res<Outing>,
) {
    let hovered = controls
        .iter()
        .find(|(interaction, _, _)| **interaction == Interaction::Hovered);
    let message = match hovered {
        Some((_, Control::Context, _)) => outing.prompt(),
        Some((_, Control::Look, _)) => "V 调查格子与敌人；不走拍。Esc 取消。",
        Some((_, Control::Book, _)) => "J 旅行手记：任务、器物、完整操作与早餐记录；← → 翻页。",
        Some((_, Control::Toss, Available(false))) => "没有李子。旅店旁拾果，或去东边摇树。",
        Some((_, Control::Toss, _)) => "T 八向三格内投一颗果子；可越果园矮墙，树、房屋与紧角挡路。",
        Some((_, Control::Wait, _)) => "空格 / 数字键盘5 等一拍；敌人也会行动，器物重新蓄势。",
        Some((_, Control::Brace, _)) => {
            "Q 护卫一拍：挡敌击，邻格反击；圆盾强化反击。喷口无视护卫。"
        }
        Some((_, Control::Skill, Available(false))) if outing.expedition.kit == Kit::None => {
            "尚未带器物：去B1西侧工具箱，选圆盾或钩索。"
        }
        Some((_, Control::Skill, Available(false))) => "器物未蓄势：Q 护卫或等候一拍后恢复。",
        Some((_, Control::Skill, _)) if outing.expedition.kit == Kit::Hook => {
            "F 正面两格钩击2伤、拉近、打断；朝向随最后一步改变。"
        }
        Some((_, Control::Skill, _)) => "F 正面邻格盾击2伤并打断；朝向随最后一步改变。",
        Some((_, Control::Drink, Available(false))) if outing.expedition.tonics == 0 => {
            "药囊为空：B1药箱或镇上药师可补给；一颗李子换一瓶。"
        }
        Some((_, Control::Drink, Available(false))) => "生命已满；药水保留在药囊中。",
        Some((_, Control::Drink, _)) => "R 喝一瓶药，恢复4HP并走一拍；满血不消耗。",
        _ if outing.turn < 4 => "斜走：同时按两个方向键，或按小键盘。J 翻手记。",
        _ => "",
    };
    for mut text in &mut guides {
        if text.0 != message {
            text.0.clear();
            text.0.push_str(message);
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
            1 => format!("WASD / 方向键组合；小键盘1-9八向。\n斜走一拍；两侧都空才可穿过墙角。\n点地面寻路；点邻敌普攻1伤。\nQ 护卫：挡敌击，邻格反击1（盾2）。\n喷口2伤无视护卫，快离开红格。\nF {}。\n器物打断敌人；Q或等候后重新蓄势。\nR 药水恢复4HP；满血不消耗。\n空格 / 小键盘5：等一拍。\nV 调查、J 手记不走拍；Esc 取消。\nT 投李子：八向三格，可越果园矮墙。\n滚轮缩放；Home恢复俯视。\n悬停按钮看说明；H 隐藏 / 显示HUD。",
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
