use crate::game::*;
use bevy::{
    prelude::*,
    text::{FontSmoothing, LineHeight},
    ui::widget::NodeImageMode,
    window::PrimaryWindow,
};

/// Textured, screen-space travel stationery. All facts come from `Outing`.
pub struct BookPlugin;
impl Plugin for BookPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<Folio>()
            .add_systems(Startup, bind_book)
            .add_systems(Update, (scale_book, click_book).in_set(crate::Frame::Input))
            .add_systems(
                Update,
                (turn_leaf, typeset_book).chain().in_set(crate::Frame::Draw),
            );
    }
}

// Layout units are 1/900 of the window height, independent of desktop DPI.
const MARGIN: f32 = 24.0;
const INK: Color = Color::srgb(0.25, 0.19, 0.15);
const FADED: Color = Color::srgb(0.43, 0.33, 0.23);
const IVORY: Color = Color::srgb(0.96, 0.89, 0.73);

/// Use before emitting a ground Point intent; coordinates are window-logical pixels.
/// The rectangles include decorative paper/wood, not just the clickable text.
pub fn pointer_over_book(p: Vec2, window: Vec2, page: Page, hud: bool) -> bool {
    if !hud && page == Page::Roam {
        return false;
    }
    let scale = (window.y / 900.0).max(0.01);
    let p = p / scale;
    let w = window.x / scale;
    let contains = |x: f32, y: f32, width: f32, height: f32| {
        p.x >= x && p.x <= x + width && p.y >= y && p.y <= y + height
    };
    contains(MARGIN, 22.0, 342.0, 90.0)
        || contains(w - 324.0, 22.0, 300.0, 90.0)
        || contains(MARGIN, 788.0, 660.0, 94.0)
        || (matches!(page, Page::Aim | Page::Look) && contains(MARGIN, 554.0, 470.0, 222.0))
        || (matches!(page, Page::Book | Page::Bowl | Page::Served)
            && contains(w - 644.0, 124.0, 620.0, 610.0))
}

#[derive(Resource, Default)]
struct Folio {
    leaf: usize,
}
#[derive(Component)]
enum Panel {
    Hud,
    Folio,
    Field,
}
#[derive(Component, Clone, Copy)]
enum CopySlot {
    Day,
    Stock,
    Notice,
    Heading,
    Body,
    FieldBody,
    Leaf,
    Context,
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
    Cancel,
    Less,
    More,
    Confirm,
}
#[derive(Component)]
struct Available(bool);
#[derive(Component)]
struct ModalControls;
#[derive(Component)]
struct JournalControls;
#[derive(Component)]
struct BreakfastArt;
#[derive(Component)]
struct BowlChoices;
#[derive(Resource)]
struct Illustrations {
    plain: Handle<Image>,
    fruit: Handle<Image>,
    field: Handle<Image>,
}

struct Stationery {
    font: Handle<Font>,
    button: Handle<Image>,
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
        parent
            .spawn((
                Button,
                action,
                Available(true),
                sliced(self.button.clone()),
                Node {
                    width: Val::Px(width),
                    height: Val::Px(46.0),
                    justify_content: JustifyContent::Center,
                    align_items: AlignItems::Center,
                    flex_shrink: 0.0,
                    ..default()
                },
            ))
            .with_children(|b| {
                let mut text = b.spawn(self.text(caption, 24.0, IVORY));
                match action {
                    Control::Context => {
                        text.insert(CopySlot::Context);
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
        button: assets.load("book/button.png"),
    };
    commands.insert_resource(Illustrations {
        plain: assets.load("book/plain-breakfast.png"),
        fruit: assets.load("book/breakfast.png"),
        field: assets.load("book/field-notes.png"),
    });
    commands
        .spawn((
            Name::new("Copper kettle inn sign"),
            Panel::Hud,
            ImageNode::new(assets.load("book/inn-sign.png")),
            at(24.0, 22.0, 342.0, 90.0),
            GlobalZIndex(10),
        ))
        .with_children(|p| {
            p.spawn((
                ImageNode::new(assets.load("book/kettle.png")),
                at(16.0, 12.0, 64.0, 64.0),
            ));
            p.spawn((
                s.text("林边 · 铜壶旅店", 24.0, IVORY),
                at(91.0, 12.0, 240.0, 32.0),
            ));
            p.spawn((
                s.text("", 24.0, Color::srgb(0.82, 0.71, 0.53)),
                CopySlot::Day,
                at(91.0, 46.0, 240.0, 30.0),
            ));
        });
    commands
        .spawn((
            Name::new("Plums and journal clasp"),
            Panel::Hud,
            Button,
            Control::Book,
            Available(true),
            ImageNode::new(assets.load("book/inventory.png")),
            Node {
                right: Val::Px(24.0),
                left: Val::Auto,
                ..at(0.0, 22.0, 300.0, 90.0)
            },
            GlobalZIndex(10),
        ))
        .with_children(|p| {
            p.spawn((
                ImageNode::new(assets.load("book/plum.png")),
                at(10.0, 12.0, 64.0, 64.0),
            ));
            p.spawn((
                s.text("", 24.0, INK),
                CopySlot::Stock,
                at(80.0, 13.0, 210.0, 32.0),
            ));
            p.spawn((
                s.text("J  翻开旅行手记", 24.0, FADED),
                at(80.0, 47.0, 210.0, 32.0),
            ));
        });
    commands
        .spawn((
            Name::new("Parchment roadside strip"),
            Panel::Hud,
            ImageNode::new(assets.load("book/road-strip.png")),
            Node {
                top: Val::Auto,
                bottom: Val::Px(18.0),
                ..at(24.0, 0.0, 660.0, 94.0)
            },
            GlobalZIndex(10),
        ))
        .with_children(|p| {
            p.spawn((
                s.text("", 24.0, INK),
                CopySlot::Notice,
                at(20.0, 6.0, 620.0, 28.0),
            ));
            p.spawn(Node {
                column_gap: Val::Px(6.0),
                ..at(20.0, 40.0, 620.0, 46.0)
            })
            .with_children(|row| {
                s.button(row, "E  拾起", Control::Context, 188.0);
                s.button(row, "T 投果", Control::Toss, 94.0);
                s.button(row, "V 观察", Control::Look, 94.0);
                s.button(row, "J 手记", Control::Book, 94.0);
                s.button(row, "等候", Control::Wait, 126.0);
            });
        });
    commands.spawn((
        Name::new("Field observation paper tag"),
        Panel::Field,
        ImageNode::new(assets.load("book/field-tag.png")),
        at(24.0, 554.0, 470.0, 222.0),
        GlobalZIndex(20),
    )).with_children(|p| {
        p.spawn((s.text("", 24.0, INK), CopySlot::Heading, at(24.0, 12.0, 422.0, 32.0)));
        p.spawn((s.text("", 24.0, INK), CopySlot::FieldBody, at(24.0, 52.0, 422.0, 96.0)));
        p.spawn(Node {
            column_gap: Val::Px(12.0),
            ..at(24.0, 158.0, 422.0, 46.0)
        }).with_children(|row| {
            s.button(row, "确认", Control::Confirm, 226.0);
            s.button(row, "取消", Control::Cancel, 112.0);
        });
    });
    commands
        .spawn((
            Name::new("Bound travel folio"),
            Panel::Folio,
            ImageNode::new(assets.load("book/folio.png")),
            Node {
                right: Val::Px(24.0),
                left: Val::Auto,
                display: Display::None,
                ..at(0.0, 124.0, 620.0, 610.0)
            },
            GlobalZIndex(20),
        ))
        .with_children(|p| {
            p.spawn((
                s.text("", 24.0, INK),
                CopySlot::Heading,
                at(104.0, 34.0, 370.0, 34.0),
            ));
            p.spawn(Node {
                ..at(484.0, 26.0, 88.0, 46.0)
            })
            .with_children(|b| s.button(b, "合上", Control::Cancel, 88.0));
            p.spawn((
                ImageNode::new(assets.load("book/breakfast.png")),
                BreakfastArt,
                at(104.0, 102.0, 474.0, 142.0),
            ));
            p.spawn((
                s.text("", 24.0, INK),
                CopySlot::Body,
                at(104.0, 268.0, 470.0, 260.0),
            ));
            p.spawn((
                ModalControls,
                Node {
                    column_gap: Val::Px(12.0),
                    justify_content: JustifyContent::FlexEnd,
                    ..at(104.0, 539.0, 474.0, 46.0)
                },
            ))
            .with_children(|row| {
                row.spawn((
                    BowlChoices,
                    Node {
                        column_gap: Val::Px(12.0),
                        ..default()
                    },
                ))
                .with_children(|b| {
                    s.button(b, "少一颗", Control::Less, 112.0);
                    s.button(b, "多一颗", Control::More, 112.0);
                });
                s.button(row, "确认", Control::Confirm, 226.0);
            });
            p.spawn((
                JournalControls,
                Node {
                    justify_content: JustifyContent::SpaceBetween,
                    ..at(104.0, 539.0, 474.0, 46.0)
                },
            ))
            .with_children(|row| {
                s.button(row, "上一页", Control::Less, 126.0);
                row.spawn((
                    s.text("", 24.0, FADED),
                    CopySlot::Leaf,
                    Node {
                        align_self: AlignSelf::Center,
                        ..default()
                    },
                ));
                s.button(row, "下一页", Control::More, 126.0);
            });
        });
}

fn scale_book(window: Query<&Window, With<PrimaryWindow>>, mut scale: ResMut<UiScale>) {
    let Ok(window) = window.single() else {
        return;
    };
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
        image.color = if !available.0 {
            Color::srgb(0.52, 0.50, 0.45)
        } else {
            match interaction {
                Interaction::Hovered => Color::srgb(1.15, 1.12, 1.02),
                Interaction::Pressed => Color::srgb(0.79, 0.75, 0.65),
                Interaction::None => Color::WHITE,
            }
        };
        if *interaction != Interaction::Pressed || !available.0 {
            continue;
        }
        intents.write(match action {
            Control::Confirm if ui.page == Page::Served => Intent::NewMorning,
            Control::Context | Control::Confirm => Intent::Interact,
            Control::Toss => Intent::Toss,
            Control::Look => Intent::Look,
            Control::Book => Intent::Book,
            Control::Wait => Intent::Wait,
            Control::Cancel => Intent::Cancel,
            Control::Less => Intent::Less,
            Control::More => Intent::More,
        });
    }
}
fn leaf_count(outing: &Outing) -> usize {
    2 + outing.history.len().div_ceil(2).max(1)
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
            Intent::Less => {
                folio.leaf = folio.leaf.saturating_sub(1);
            }
            Intent::More => {
                folio.leaf = (folio.leaf + 1).min(leaf_count(&outing) - 1);
            }
            _ => {}
        }
    }
}
fn typeset_book(
    outing: Res<Outing>,
    ui: Res<Interface>,
    folio: Res<Folio>,
    mut texts: Query<(&CopySlot, &mut Text)>,
    mut panels: Query<
        (&Panel, &mut Node),
        (
            Without<ModalControls>,
            Without<JournalControls>,
            Without<BowlChoices>,
        ),
    >,
    mut modal: Query<
        &mut Node,
        (
            With<ModalControls>,
            Without<Panel>,
            Without<JournalControls>,
            Without<BowlChoices>,
        ),
    >,
    mut journal: Query<
        &mut Node,
        (
            With<JournalControls>,
            Without<Panel>,
            Without<ModalControls>,
            Without<BowlChoices>,
        ),
    >,
    mut choices: Query<
        &mut Node,
        (
            With<BowlChoices>,
            Without<Panel>,
            Without<ModalControls>,
            Without<JournalControls>,
        ),
    >,
    illustrations: Res<Illustrations>,
    mut illustration: Query<&mut ImageNode, (With<BreakfastArt>, Without<Button>)>,
    mut controls: Query<(&Control, &mut Available, &mut ImageNode), With<Button>>,
) {
    if !outing.is_changed() && !ui.is_changed() && !folio.is_changed() {
        return;
    }
    let shown = ui.hud || ui.page != Page::Roam;
    for (panel, mut node) in &mut panels {
        let visible = match panel {
            Panel::Hud => shown,
            Panel::Folio => matches!(ui.page, Page::Book | Page::Bowl | Page::Served),
            Panel::Field => matches!(ui.page, Page::Aim | Page::Look),
        };
        node.display = if visible {
            Display::Flex
        } else {
            Display::None
        };
    }
    for mut n in &mut modal {
        n.display = if ui.page != Page::Book {
            Display::Flex
        } else {
            Display::None
        };
    }
    for mut n in &mut journal {
        n.display = if ui.page == Page::Book {
            Display::Flex
        } else {
            Display::None
        };
    }
    for mut n in &mut choices {
        n.display = if ui.page == Page::Bowl {
            Display::Flex
        } else {
            Display::None
        };
    }
    let art = if matches!(ui.page, Page::Aim | Page::Look)
        || (ui.page == Page::Book && folio.leaf == 1)
    {
        &illustrations.field
    } else if (ui.page == Page::Bowl && ui.filling > 0)
        || (ui.page != Page::Bowl && outing.breakfast.unwrap_or(0) > 0)
    {
        &illustrations.fruit
    } else {
        &illustrations.plain
    };
    for mut image in &mut illustration {
        if image.image != *art {
            image.image = art.clone();
        }
    }
    for (control, mut available, mut image) in &mut controls {
        let yes = match control {
            Control::Toss => ui.page == Page::Roam && outing.plums > 0,
            Control::Look | Control::Wait => ui.page == Page::Roam,
            Control::Context => ui.page == Page::Roam || ui.page == Page::Served,
            Control::Book | Control::Cancel => true,
            Control::Less => match ui.page {
                Page::Book => folio.leaf > 0,
                Page::Bowl => ui.filling > 0,
                _ => false,
            },
            Control::More => match ui.page {
                Page::Book => folio.leaf + 1 < leaf_count(&outing),
                Page::Bowl => ui.filling < outing.plums,
                _ => false,
            },
            Control::Confirm => match ui.page {
                Page::Aim => outing.can_toss(ui.cursor),
                Page::Look | Page::Bowl | Page::Served => true,
                _ => false,
            },
        };
        if available.0 != yes {
            available.0 = yes;
            image.color = if yes {
                Color::WHITE
            } else {
                Color::srgb(0.52, 0.50, 0.45)
            };
        }
    }
    for (slot, mut text) in &mut texts {
        let value = match slot {
            CopySlot::Day => format!("第 {} 天   晴", outing.day),
            CopySlot::Stock => format!("李子   × {}", outing.plums),
            CopySlot::Notice => {
                let notice = if outing.notice.is_empty() { outing.prompt() } else { outing.notice };
                let first = notice.split_inclusive(['。', '！', '？']).next().unwrap_or(notice);
                let mut chars = first.chars();
                let mut short: String = chars.by_ref().take(24).collect();
                if chars.next().is_some() { short.push('…'); }
                short
            }
            CopySlot::Context => {
                if ui.page == Page::Served {
                    "E  住到明天".to_owned()
                } else {
                    format!("E  {}", outing.prompt().trim_start_matches("E "))
                }
            }
            CopySlot::Heading => match ui.page {
                Page::Aim => "往墙那边，投一颗".to_owned(),
                Page::Look => "停下来，仔细看看".to_owned(),
                Page::Bowl => "今天的早餐".to_owned(),
                Page::Served => "早餐后".to_owned(),
                Page::Book => match folio.leaf {
                    0 => "旅行手记 · 今日",
                    1 => "出门小记",
                    _ => "留在纸上的早晨",
                }
                .to_owned(),
                Page::Roam => String::new(),
            },
            CopySlot::Body => body(&outing, &ui, folio.leaf),
            CopySlot::FieldBody => match ui.page {
                Page::Aim => format!("手里 {} 颗，三步内可越墙。\n方向键 / 点地面选落点。\n落点：东 {}，南 {}", outing.plums, ui.cursor.x, ui.cursor.y),
                Page::Look => outing.describe(ui.cursor).to_owned(),
                _ => String::new(),
            },
            CopySlot::Leaf => format!("{} / {}", folio.leaf + 1, leaf_count(&outing)),
            CopySlot::Close => if ui.page == Page::Served {
                "留下"
            } else if matches!(ui.page, Page::Aim | Page::Look) {
                "取消"
            } else {
                "合上"
            }
            .to_owned(),
            CopySlot::Confirm => match ui.page {
                Page::Aim => "E  投出一颗",
                Page::Look => "E  看好了",
                Page::Bowl => "E  端上早餐",
                Page::Served => "住到明天",
                _ => "确认",
            }
            .to_owned(),
        };
        if text.0 != value {
            text.0 = value;
        }
    }
}
fn body(o: &Outing, ui: &Interface, leaf: usize) -> String {
    match ui.page {
        Page::Aim => format!("手里还有 {} 颗李子。\n\n方向键或点地面选落点。\n三步以内，可以越过低墙。\n\n落点：东 {}，南 {}\n确认投出；合上便放回手里。",o.plums,ui.cursor.x,ui.cursor.y),
        Page::Look => format!("{}\n\n方向键或点地面换一格看。\n观察和翻手记，都不费行动。\n\n小家伙到果子旁会停一拍，\n再把它吞进肚子。",o.describe(ui.cursor)),
        Page::Bowl => {
            let meal = if ui.filling == 0 { "原味白凝乳".to_owned() } else { format!("白凝乳，加 {} 颗李子",ui.filling) };
            format!("{}\n\n手里有 {} 颗；这碗用 {} 颗。\n吃完还留 {} 颗在包里。\n\n用左右键，或点下面的铜扣。\n空手也有一碗原味的。",meal,o.plums,ui.filling,o.plums.saturating_sub(ui.filling))
        },
        Page::Served => {
            let used = o.breakfast.unwrap_or(0);
            format!("{}\n\n这碗用了 {} 颗李子。\n包里留着 {} 颗。\n小家伙今天吃了 {} 颗。\n\n可以出去再走走，也可以住到明天。\n手记写好了。",if used==0 {"白凝乳，入口微酸。"} else {"碗沿还挂着紫红的果汁。"},used,o.plums,o.eaten)
        },
        Page::Book => match leaf {
            0 => format!("第 {} 天 · 晴\n\n手里有 {} 颗李子。\n地上还散着 {} 颗。\n小家伙已吃了 {} 颗。\n\n{}\n\n左右翻页，J / Esc 合上。",o.day,o.plums,o.fruit.len(),o.eaten,
                if o.breakfast.is_some() {"今天已经吃过早餐了。"} else {"早餐还在门廊下等着。"}),
            1 => "方向键 / WASD 走路，空格等候。\n滚轮推近时，会低到旅人的眼前。\nHome 拉回俯视。\n\nT 抛李子；V 察看，不费回合。\n\n回到餐桌边，按 E 吃早餐。\n点邻格可互动，点远处能走过去。".to_owned(),
            _ => {
                let start = (leaf-2)*2;
                if o.history.is_empty() { return "纸上还没有早餐的记录。\n\n先去门廊，坐下来吃一碗。\n原味也是值得记住的早晨。".to_owned(); }
                let mut result=String::new();
                for meal in o.history.iter().rev().skip(start).take(2) {
                    use std::fmt::Write;
                    let _=write!(result,"第 {} 天 · {}\n早餐用 {} 颗，留了 {} 颗。\n小家伙吃了 {} 颗。\n\n",meal.day,if meal.used==0 {"原味"} else {"李子凝乳"},meal.used,meal.kept,meal.shared);
                }
                result.trim_end().to_owned()
            },
        },
        Page::Roam => String::new(),
    }
}
