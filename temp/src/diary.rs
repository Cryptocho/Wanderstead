use bevy::{prelude::*, text::FontSmoothing};

use crate::{
    party::{Expedition, PartyMovement},
    world::PLAN,
};

pub struct DiaryPlugin;

impl Plugin for DiaryPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<Survey>()
            .add_event::<FoundMark>()
            .add_systems(Startup, bind_diary)
            .add_systems(
                Update,
                (investigate.after(PartyMovement), typeset.after(investigate)),
            );
    }
}

struct Site {
    position: fn() -> Vec2,
    reach: f32,
    folio: &'static str,
    name: &'static str,
    hint: &'static str,
    inscription: &'static str,
}

const SITES: [Site; 3] = [
    Site {
        position: || PLAN.chest.into(),
        reach: 1.82,
        folio: "I    /    THE WEST VAULT",
        name: "西廊铜匣",
        hint: "往西南，到火盆旁的石龛。",
        inscription: "匣中藏着一枚铜针。针尖没有指向北方，它在寻找太阳。",
    },
    Site {
        position: || PLAN.crystal.into(),
        reach: 1.92,
        folio: "II   /    THE TIDAL ALTAR",
        name: "听泉石",
        hint: "沿东侧水岸，寻发光的祭台。",
        inscription: "把手放上水晶，指间响起泉声。那是日轮的第二个刻度。",
    },
    Site {
        position: || PLAN.gate.into(),
        reach: 1.91,
        folio: "III  /    THE DAWN GATE",
        name: "晨光之门",
        hint: "拾级向北，走到双像之间。",
        inscription: "石门没有锁。碑上说：当泉与铜同鸣，光会回到庭心。",
    },
];

#[derive(Event)]
pub struct FoundMark;

#[derive(Resource)]
pub struct Survey {
    marks: u8,
    near: Option<usize>,
    page: bool,
    hud: bool,
    excerpt: Option<usize>,
    dismiss_at: f64,
}

impl Default for Survey {
    fn default() -> Self {
        Self {
            marks: 0,
            near: None,
            page: false,
            hud: true,
            excerpt: None,
            dismiss_at: 0.0,
        }
    }
}

impl Survey {
    pub fn complete(&self) -> bool {
        self.marks == 0b111
    }
    fn recorded(&self, i: usize) -> bool {
        self.marks & (1 << i) != 0
    }
}

fn investigate(
    keys: Res<ButtonInput<KeyCode>>,
    party: Res<Expedition>,
    time: Res<Time>,
    mut survey: ResMut<Survey>,
    mut found: EventWriter<FoundMark>,
) {
    let near = SITES
        .iter()
        .position(|s| party.leader.distance_squared((s.position)()) < s.reach * s.reach);
    if near != survey.near {
        survey.near = near;
    }
    if keys.just_pressed(KeyCode::Tab) {
        survey.page = !survey.page;
        survey.hud = true;
    }
    if keys.just_pressed(KeyCode::Escape) && survey.page {
        survey.page = false;
    }
    if keys.just_pressed(KeyCode::KeyH) {
        survey.hud = !survey.hud;
        survey.page = false;
    }
    if survey.excerpt.is_some() && time.elapsed_secs_f64() >= survey.dismiss_at {
        survey.excerpt = None;
    }
    if keys.just_pressed(KeyCode::KeyE)
        && let Some(i) = near
    {
        if !survey.recorded(i) {
            survey.marks |= 1 << i;
            info!(
                "Field note: {} at {:.2}, {:.2} ({} / 3)",
                SITES[i].name,
                party.leader.x,
                party.leader.y,
                survey.marks.count_ones()
            );
            found.write(FoundMark);
        }
        survey.excerpt = Some(i);
        survey.dismiss_at = time.elapsed_secs_f64() + 6.5;
    }
}

const GOLD: Color = Color::srgb(0.93, 0.76, 0.45);
const IVORY: Color = Color::srgb(0.995, 0.955, 0.84);
const SAGE: Color = Color::srgb(0.73, 0.80, 0.72);
const INK: Color = Color::srgba(0.033, 0.070, 0.074, 0.87);
const RULE: Color = Color::srgba(0.70, 0.56, 0.33, 0.69);

struct Faces {
    title: Handle<Font>,
    keys: Handle<Font>,
}
impl Faces {
    fn typeface(&self, size: f32, color: Color, pixel: bool) -> (TextFont, TextColor, TextShadow) {
        (
            TextFont {
                font: if pixel {
                    self.keys.clone()
                } else {
                    self.title.clone()
                },
                font_size: size,
                font_smoothing: if pixel {
                    FontSmoothing::None
                } else {
                    FontSmoothing::AntiAliased
                },
                ..default()
            },
            TextColor(color),
            TextShadow {
                offset: Vec2::new(0.0, 1.0),
                color: Color::srgba(0.01, 0.018, 0.019, 0.75),
            },
        )
    }
}

#[derive(Component)]
enum Panel {
    Plate,
    Guide,
    Book,
    Quotation,
}

#[derive(Component)]
enum CopySlot {
    Guide,
    Count,
    Entry(usize),
    QuoteTitle,
    Quote,
}

fn rule() -> impl Bundle {
    (
        Node {
            height: Val::Px(1.0),
            width: Val::Percent(100.0),
            margin: UiRect::vertical(Val::Px(5.0)),
            ..default()
        },
        BackgroundColor(RULE),
    )
}

fn bind_diary(mut commands: Commands, assets: Res<AssetServer>) {
    let faces = Faces {
        title: assets.load("fonts/Inscription.ttf"),
        keys: assets.load("fonts/FieldPixel.ttf"),
    };
    let seal = assets.load("relic/seal.png");
    let px = Val::Px;
    commands
        .spawn((
            Panel::Plate,
            Name::new("Location cartouche"),
            Node {
                position_type: PositionType::Absolute,
                left: px(18.0),
                top: px(16.0),
                align_items: AlignItems::Center,
                column_gap: px(10.0),
                padding: UiRect {
                    left: px(8.0),
                    right: px(15.0),
                    top: px(5.0),
                    bottom: px(6.0),
                },
                border: UiRect::left(px(2.0)),
                ..default()
            },
            BackgroundColor(INK),
            BorderColor(RULE),
        ))
        .with_children(|plate| {
            plate.spawn((
                ImageNode::new(seal),
                Node {
                    width: px(38.0),
                    height: px(38.0),
                    ..default()
                },
            ));
            plate
                .spawn(Node {
                    flex_direction: FlexDirection::Column,
                    ..default()
                })
                .with_children(|label| {
                    label.spawn((
                        Text::new("A T L A S     /     0 1"),
                        faces.typeface(12.0, GOLD, true),
                    ));
                    label.spawn((Text::new("日照圣殿"), faces.typeface(25.0, IVORY, false)));
                });
        });
    commands
        .spawn((
            Panel::Guide,
            Name::new("Trail controls"),
            Node {
                position_type: PositionType::Absolute,
                left: px(18.0),
                bottom: px(16.0),
                max_width: Val::Percent(94.0),
                padding: UiRect::axes(px(11.0), px(7.0)),
                border: UiRect::left(px(2.0)),
                ..default()
            },
            BackgroundColor(INK),
            BorderColor(RULE),
        ))
        .with_child((
            Text::default(),
            CopySlot::Guide,
            faces.typeface(12.0, IVORY, true),
        ));
    commands
        .spawn((
            Panel::Quotation,
            Name::new("Read inscription"),
            Node {
                position_type: PositionType::Absolute,
                left: px(18.0),
                bottom: px(58.0),
                flex_direction: FlexDirection::Column,
                width: px(370.0),
                max_width: Val::Percent(85.0),
                border: UiRect::left(px(2.0)),
                row_gap: px(5.0),
                padding: UiRect::axes(px(15.0), px(11.0)),
                display: Display::None,
                ..default()
            },
            BackgroundColor(INK),
            BorderColor(RULE),
        ))
        .with_children(|note| {
            note.spawn((
                Text::default(),
                CopySlot::QuoteTitle,
                faces.typeface(12.0, GOLD, true),
            ));
            note.spawn((
                Text::default(),
                CopySlot::Quote,
                faces.typeface(17.0, IVORY, false),
            ));
        });
    commands.spawn((Panel::Book, Name::new("Explorer's folio"), GlobalZIndex(2), Node {
        position_type: PositionType::Absolute, right: px(18.0), top: px(16.0),
        width: px(342.0), max_width: Val::Percent(90.0), max_height: Val::Percent(94.0), flex_direction: FlexDirection::Column,
        padding: UiRect::axes(px(20.0), px(16.0)), border: UiRect::all(px(1.0)), row_gap: px(3.0),
        display: Display::None, ..default()
    }, BackgroundColor(Color::srgba(0.034, 0.068, 0.073, 0.97)), BorderColor(RULE))).with_children(|book| {
        book.spawn((Text::new("F I E L D   N O T E S      /      SOLARIUM"), faces.typeface(12.0, GOLD, true)));
        book.spawn((Text::new("失落的日轮"), faces.typeface(31.0, IVORY, false)));
        book.spawn((Text::default(), CopySlot::Count, faces.typeface(14.0, SAGE, false)));
        for (i, site) in SITES.iter().enumerate() {
            book.spawn(rule());
            book.spawn((Text::new(site.folio), faces.typeface(12.0, GOLD, true)));
            book.spawn((Text::new(site.name), faces.typeface(20.0, IVORY, false)));
            book.spawn((Text::default(), CopySlot::Entry(i), faces.typeface(16.0, SAGE, false)));
        }
        book.spawn(rule());
        book.spawn((Text::new("WASD / 方向键  移动       SHIFT  疾行\nE  调查       C  平视 / 俯视\n滚轮 / +/-  远近       HOME / R  复位\nL  改变日照      F  切换对焦\nH  隐藏界面     M  静音     F12  留影\nTAB / ESC  合上手记"), faces.typeface(12.0, IVORY, true)));
    });
}

fn typeset(
    survey: Res<Survey>,
    mut panels: Query<(&Panel, &mut Node)>,
    mut copy: Query<(&CopySlot, &mut Text)>,
) {
    if !survey.is_changed() {
        return;
    }
    for (panel, mut node) in &mut panels {
        let on = survey.hud
            && match panel {
                Panel::Plate => true,
                Panel::Guide => !survey.page,
                Panel::Book => survey.page,
                Panel::Quotation => !survey.page && survey.excerpt.is_some(),
            };
        node.display = if on { Display::Flex } else { Display::None };
    }
    for (field, mut text) in &mut copy {
        let content = match *field {
            CopySlot::Guide => match survey.near {
                Some(i) => {
                    **text = format!(
                        "E  {} · {}       TAB  手记",
                        if survey.recorded(i) {
                            "重读"
                        } else {
                            "调查"
                        },
                        SITES[i].name
                    );
                    continue;
                }
                None => "WASD  移动    C  近景    L  日照    TAB  手记",
            },
            CopySlot::Count => [
                "遗迹图录   0 / 3",
                "遗迹图录   1 / 3",
                "遗迹图录   2 / 3",
                "图录完整   3 / 3   ·   日轮已在庭心升起",
            ][survey.marks.count_ones() as usize],
            CopySlot::Entry(i) => {
                if survey.recorded(i) {
                    SITES[i].inscription
                } else {
                    SITES[i].hint
                }
            }
            CopySlot::QuoteTitle => {
                if survey.complete() {
                    "SOLARIUM   /   日轮共鸣"
                } else {
                    "FIELD NOTE   /   遗迹铭文"
                }
            }
            CopySlot::Quote => {
                if survey.complete() {
                    "三处刻度对齐了。回到庭心，看日轮在石纹之上升起。"
                } else if let Some(i) = survey.excerpt {
                    SITES[i].inscription
                } else {
                    ""
                }
            }
        };
        if text.as_str() != content {
            content.clone_into(&mut text.0);
        }
    }
}
