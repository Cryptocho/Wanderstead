use crate::{
    delve::{EnemyKind, Expedition, Kit, Quest},
    map::{self, Area},
};
use bevy::prelude::*;
use serde::{Deserialize, Serialize};
use std::{fs, io, path::PathBuf};

pub const START: IVec2 = IVec2::new(-2, 3);
pub const TABLE: IVec2 = IVec2::new(-4, 3);
pub const KEEPER: IVec2 = IVec2::new(-5, 3);
pub const FIRST_PLUM: IVec2 = IVec2::new(-3, 3);
pub const TREE: IVec2 = IVec2::new(5, -4);
pub const TREE2: IVec2 = IVec2::new(8, -2);
pub const JELLY_START: IVec2 = IVec2::new(4, -2);
const W: usize = map::WIDTH;
const H: usize = map::HEIGHT;
const CELLS: usize = W * H;
use crate::map::DIRS;

pub fn ground(p: IVec2) -> Vec3 {
    Vec3::new(p.x as f32, 0., p.y as f32)
}
pub fn on_map(p: IVec2) -> bool {
    map::inside(p)
}
pub fn distance(a: IVec2, b: IVec2) -> u32 {
    map::distance(a, b)
}
fn cell(p: IVec2) -> usize {
    map::cell_index(p)
}

// The authored map has 315 cells: byte distances and queue indices overflow.
struct Routes {
    dist: [u16; CELLS],
    first: [u8; CELLS],
    previous: [usize; CELLS],
}
impl Routes {
    fn from(origin: IVec2, area: Area, blocked: impl Fn(IVec2) -> bool) -> Self {
        let mut r = Self {
            dist: [u16::MAX; CELLS],
            first: [DIRS.len() as u8; CELLS],
            previous: [usize::MAX; CELLS],
        };
        if !on_map(origin) {
            return r;
        }
        let mut q = [IVec2::ZERO; CELLS];
        q[0] = origin;
        r.dist[cell(origin)] = 0;
        let (mut head, mut end) = (0, 1);
        while head < end {
            let p = q[head];
            head += 1;
            let i = cell(p);
            for (d, delta) in DIRS.into_iter().enumerate() {
                let n = p + delta;
                if !map::passage(p, n, |p| map::passable(area, p) && !blocked(p)) {
                    continue;
                }
                let j = cell(n);
                if r.dist[j] != u16::MAX {
                    continue;
                }
                r.dist[j] = r.dist[i] + 1;
                r.first[j] = if p == origin { d as u8 } else { r.first[i] };
                r.previous[j] = i;
                q[end] = n;
                end += 1;
            }
        }
        r
    }
    fn step(&self, dest: IVec2) -> Option<IVec2> {
        if !on_map(dest) {
            return None;
        }
        DIRS.get(self.first[cell(dest)] as usize).copied()
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ActionKind {
    Start,
    Step,
    Wait,
    Gather,
    Shake,
    Throw { from: IVec2, to: IVec2 },
    Eat,
    Strike,
    Brace,
    Skill,
    Drink,
    Hurt,
    Descend,
    Return,
    Restore,
}
#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub struct Meal {
    pub day: u32,
    pub used: u8,
    pub shared: u8,
    pub kept: u8,
}
#[derive(Resource)]
pub struct Outing {
    pub day: u32,
    pub turn: u32,
    pub player: IVec2,
    pub facing: IVec2,
    pub jelly: IVec2,
    pub jelly_facing: IVec2,
    pub target: Option<IVec2>,
    pub fruit: Vec<IVec2>,
    pub plums: u8,
    pub eaten: u8,
    pub tree_shaken: bool,
    pub breakfast: Option<u8>,
    pub history: Vec<Meal>,
    pub action: ActionKind,
    pub swallowed: bool,
    pub notice: &'static str,
    pub area: Area,
    pub expedition: Expedition,
    pub save_revision: u32,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum Page {
    #[default]
    Roam,
    Aim,
    Look,
    Bowl,
    Served,
    Book,
    Gear,
    Barter,
    Down,
    Homecoming,
}
#[derive(Resource)]
pub struct Interface {
    pub page: Page,
    pub cursor: IVec2,
    pub filling: u8,
    pub hud: bool,
}
impl Default for Interface {
    fn default() -> Self {
        Self {
            page: Page::Roam,
            cursor: START,
            filling: 0,
            hud: false,
        }
    }
}
#[derive(Event, Clone, Copy)]
pub enum Intent {
    Dir(IVec2),
    Interact,
    Wait,
    Look,
    Toss,
    Book,
    Cancel,
    Less,
    More,
    NewMorning,
    Point(IVec2),
    Brace,
    Skill,
    Drink,
}
#[derive(Clone, Copy, PartialEq, Eq)]
enum Here {
    Fruit(IVec2),
    Table,
    Bough,
    Keeper,
    Herbalist,
    Jelly,
    None,
    Gate,
    Stairs,
    Fountain,
    ToolChest,
    TonicChest,
    Wheel,
    Enemy(IVec2),
}
const HARVEST: [[IVec2; 4]; 3] = [
    [
        IVec2::new(5, -3),
        IVec2::new(5, -2),
        IVec2::new(5, -1),
        IVec2::new(6, -2),
    ],
    [
        IVec2::new(4, -4),
        IVec2::new(3, -3),
        IVec2::new(6, -3),
        IVec2::new(6, -4),
    ],
    [
        IVec2::new(4, -3),
        IVec2::new(4, -1),
        IVec2::new(6, -2),
        IVec2::new(7, -3),
    ],
];
impl Outing {
    pub fn new(history: Vec<Meal>) -> Self {
        let (day, plums) = history
            .last()
            .map_or((1, 0), |m| (m.day.saturating_add(1), m.kept));
        let mut fruit = Vec::with_capacity(32);
        fruit.push(FIRST_PLUM);
        Self {
            day,
            turn: 0,
            player: START,
            facing: IVec2::NEG_X,
            jelly: JELLY_START,
            jelly_facing: IVec2::X,
            target: None,
            fruit,
            plums,
            eaten: 0,
            tree_shaken: false,
            breakfast: None,
            history,
            action: ActionKind::Start,
            swallowed: false,
            notice: "酒壶、脸盆、水桶都搁在空泉池边。北水门通往地下的回水廊。",
            area: Area::Town,
            expedition: Expedition::default(),
            save_revision: 0,
        }
    }
    fn there(&self, p: IVec2) -> Here {
        if self.area == Area::Cistern {
            return if self.expedition.enemy_at(p).is_some() {
                Here::Enemy(p)
            } else if p == map::STAIRS {
                Here::Stairs
            } else if p == map::TOOL_CHEST {
                Here::ToolChest
            } else if p == map::TONIC_CHEST {
                Here::TonicChest
            } else if p == map::RELIC {
                Here::Wheel
            } else {
                Here::None
            };
        }
        if p == map::GATE {
            return Here::Gate;
        }
        if p == map::APOTHECARY {
            return Here::Herbalist;
        }
        if matches!(map::glyph(Area::Town, p), b'f' | b'F') {
            return Here::Fountain;
        }
        if p == self.jelly {
            Here::Jelly
        } else if self.fruit.contains(&p) {
            Here::Fruit(p)
        } else if p == TABLE {
            Here::Table
        } else if p == TREE {
            Here::Bough
        } else if p == KEEPER {
            Here::Keeper
        } else {
            Here::None
        }
    }
    fn can_reach_from(&self, from: IVec2, to: IVec2) -> bool {
        map::reach(from, to, |p| self.walkable(p) && !self.blocked(p))
    }
    fn fountain_reach_from(&self, from: IVec2) -> bool {
        DIRS.iter().any(|d| {
            let p = from + *d;
            matches!(map::glyph(self.area, p), b'f' | b'F') && self.can_reach_from(from, p)
        })
    }
    fn nearby(&self) -> Here {
        std::iter::once(self.player)
            .chain(std::iter::once(self.player + self.facing))
            .chain(DIRS.into_iter().map(|d| self.player + d))
            .filter(|&p| self.can_reach_from(self.player, p))
            .map(|p| self.there(p))
            .find(|h| *h != Here::None)
            .unwrap_or(Here::None)
    }
    pub fn prompt(&self) -> &'static str {
        match self.nearby() {
            Here::Gate => "下北门 · 旧泉回水廊",
            Here::Stairs => "撤回镇上 · 保住本趟铜片与阀轮",
            Here::Fountain if self.expedition.quest == Quest::Carrying => "装回铜阀轮",
            Here::Fountain => "查看镇泉",
            Here::Herbalist => "向药师换一瓶药",
            Here::ToolChest => "开启器物箱 · 二选一",
            Here::TonicChest => "收取药剂",
            Here::Wheel => "取走东台阀轮",
            Here::Enemy(_) => "攻击 · 1伤 / Q格挡反击 / F技艺",
            Here::Fruit(_) => "拾起李子",
            Here::Bough if self.tree_shaken => "看看枝头",
            Here::Bough => "摇一摇树枝",
            Here::Table if self.breakfast.is_some() => "回到餐桌",
            Here::Table => "坐下吃早餐",
            Here::Keeper => "与店主说话",
            Here::Jelly => "看看小魔物",
            Here::None => "看看脚下",
        }
    }
    pub fn describe(&self, p: IVec2) -> &'static str {
        if self.area == Area::Cistern {
            return if let Some(i) = self.expedition.enemy_at(p) {
                match self.expedition.enemies[i].kind {
                    EnemyKind::Sentry => {
                        "重装守卫：邻近后锁定一格蓄势，下一拍3伤。走开会挥空；Q格挡反击，F打断。"
                    }
                    EnemyKind::Moth => {
                        "灯蛾：隔拍飞行，近身绕行，亮起的锁定格下一拍2伤。侧移或Q应对。"
                    }
                }
            } else if p == map::STAIRS {
                "石阶上有日光。主动撤回会保住铜片与阀轮，并在店前恢复体力。"
            } else if p == map::TOOL_CHEST {
                "西翼器物箱：圆盾强化格挡反击；钩索能隔一格拉近、打断敌人。一趟只选一件。"
            } else if p == map::TONIC_CHEST {
                "药剂柜可补两瓶，上限三瓶。R消耗一瓶恢复4体力；满血不会浪费。"
            } else if p == map::RELIC {
                "东台的铜阀轮。必须亲自带上石阶，才算安全带回；镇泉台边E安装。"
            } else if map::glyph(self.area, p) == b'P' {
                "通风口每四拍喷气，下一拍将喷发时会亮起。喷气2伤，Q挡不住，移到旁边石格。"
            } else if crate::delve::safe(p) {
                "日光照得到的石阶安全格。敌人止步于此，可以等候、喝药，或从这里撤退。"
            } else if !self.walkable(p) {
                "回水廊的厚墙与深水。沿石道寻找另一翼，不能穿过去。"
            } else {
                "干燥石格可躲开锁定攻击。狭口适合逐个迎敌；一次行动，敌人才走一拍。"
            };
        }
        if p == map::GATE {
            return "北门石阶通往旧泉回水廊。西翼有器物与药剂，东台保管着镇泉缺失的铜阀轮。";
        }
        if p == map::APOTHECARY {
            return "药师对着天光看了看瓶底。一颗熟李子能换一瓶下井用的药。";
        }
        if matches!(map::glyph(Area::Town, p), b'f' | b'F') {
            return match self.expedition.quest {
                Quest::Restored => {
                    "铜阀轮已经安好，清水重新流过花坛。下次清理回水廊仍可带回铜片，改善行装。"
                }
                Quest::Carrying => "带回的铜阀轮正合轴口。靠近泉台任意一边，按E把它装回去。",
                Quest::Seeking => {
                    "泉台干涸，轴上缺了一只铜阀轮。店主说它留在北门地下回水廊的东台。"
                }
            };
        }
        if p == self.jelly {
            if self.target.is_some() {
                "小家伙把身子探长了，眼睛朝着地上的李子。"
            } else {
                "青团似的小家伙歇在草上，肚皮里透出一粒紫色。"
            }
        } else if self.fruit.contains(&p) {
            "一颗熟透的李子，果皮上还带着薄霜。"
        } else if p == self.player {
            "靴子沾着草屑，行囊还轻得很。"
        } else if p == TREE {
            if self.tree_shaken {
                "枝头只剩青果了。等明早再来。"
            } else {
                "树枝都被李子压弯了。站到树干旁就够得着。"
            }
        } else if p == TREE2 {
            "这棵李树开花晚，果子还是青的。"
        } else if p == TABLE {
            if self.breakfast.is_some() {
                "空碗旁留着你刚写好的那页手记。"
            } else {
                "铜壶招牌下的长桌，白釉碗里盛着凝乳。"
            }
        } else if p == KEEPER {
            "店主拿布揩干碗底，又摞起一只。"
        } else if !self.walkable(p) {
            "石墙和屋脚都结实得很。低墙挡住脚，挡不住抛过去的李子。"
        } else if p.x > 2 {
            "树下的草被压出细软的小径。"
        } else {
            "车辙中间露出干燥的浅色石头。"
        }
    }
    pub fn can_toss(&self, p: IVec2) -> bool {
        self.plums > 0
            && self.area == Area::Town
            && self.walkable(p)
            && self.fruit.len() < 32
            && (1..=3).contains(&distance(self.player, p))
            && p != self.jelly
            && !self.fruit.contains(&p)
            && map::clear_line(self.player, p, |at| {
                // A lob clears the deliberately low orchard wall, not buildings or trees.
                (self.walkable(at) || (at.x == 2 && (-3..=0).contains(&at.y))) && !self.blocked(at)
            })
    }
    fn blocked(&self, p: IVec2) -> bool {
        if self.area == Area::Town {
            p == self.jelly
        } else {
            self.expedition.enemy_at(p).is_some()
        }
    }
    pub fn walkable(&self, p: IVec2) -> bool {
        map::passable(self.area, p)
    }
    pub fn is_interactable(&self, p: IVec2) -> bool {
        self.there(p) != Here::None
    }
    pub fn threatened(&self) -> bool {
        self.area == Area::Cistern
            && (self
                .expedition
                .enemies
                .iter()
                .any(|e| distance(self.player, e.pos) <= 5)
                || self.danger_at(self.player)
                || DIRS.iter().any(|d| self.danger_at(self.player + *d)))
    }
    pub fn danger_at(&self, p: IVec2) -> bool {
        self.area == Area::Cistern && self.expedition.danger_at(p)
    }
    fn destination(&self, dest: IVec2, r: &Routes) -> Option<IVec2> {
        if !on_map(dest) {
            return None;
        }
        if !self.is_interactable(dest) && self.walkable(dest) && !self.blocked(dest) {
            return Some(dest);
        }
        if !self.is_interactable(dest) {
            return None;
        }
        // A click anywhere on the blocked 3×3 fountain finds the entire reachable rim.
        let fountain =
            self.area == Area::Town && matches!(map::glyph(self.area, dest), b'f' | b'F');
        let mut best = None;
        let mut score = u16::MAX;
        for y in map::MIN.y..=map::MAX.y {
            for x in map::MIN.x..=map::MAX.x {
                let p = IVec2::new(x, y);
                let adjacent = if fountain {
                    self.fountain_reach_from(p)
                } else {
                    self.can_reach_from(p, dest)
                };
                if adjacent && self.walkable(p) && !self.blocked(p) && r.dist[cell(p)] < score {
                    score = r.dist[cell(p)];
                    best = Some(p);
                }
            }
        }
        best
    }
    pub fn path_to(&self, dest: IVec2, out: &mut Vec<IVec2>) -> bool {
        out.clear();
        let r = Routes::from(self.player, self.area, |p| self.blocked(p));
        let Some(stand) = self.destination(dest, &r) else {
            return false;
        };
        if r.dist[cell(stand)] == u16::MAX {
            return false;
        }
        let mut i = cell(stand);
        while i != cell(self.player) {
            out.push(IVec2::new(
                (i % W) as i32 + map::MIN.x,
                (i / W) as i32 + map::MIN.y,
            ));
            i = r.previous[i];
        }
        out.reverse();
        true
    }
    fn beat(&mut self, kind: ActionKind) {
        self.action = kind;
        self.turn = self.turn.saturating_add(1);
        self.swallowed = false;
        // A quiet walk changes neither the journal nor the town snapshot.
        if !matches!(kind, ActionKind::Step | ActionKind::Wait) {
            self.save_revision = self.save_revision.wrapping_add(1);
        }
        if self.target.is_none() && !self.fruit.iter().any(|&p| distance(self.jelly, p) <= 5) {
            return;
        }
        if self.fruit.is_empty() {
            self.target = None;
            return;
        }
        let r = Routes::from(self.jelly, Area::Town, |p| p == self.player);
        let heard = match kind {
            ActionKind::Throw { to, .. } if distance(self.jelly, to) <= 5 => Some(to),
            _ => None,
        };
        let old = self
            .target
            .filter(|p| self.fruit.contains(p) && r.dist[cell(*p)] != u16::MAX);
        self.target = heard
            .filter(|p| r.dist[cell(*p)] != u16::MAX)
            .or(old)
            .or_else(|| {
                self.fruit
                    .iter()
                    .copied()
                    .filter(|&p| distance(self.jelly, p) <= 5 && r.dist[cell(p)] != u16::MAX)
                    .min_by_key(|&p| (r.dist[cell(p)], p.x, p.y))
            });
        let Some(t) = self.target else {
            return;
        };
        self.save_revision = self.save_revision.wrapping_add(1);
        if t == self.jelly {
            let i = self
                .fruit
                .iter()
                .position(|&p| p == t)
                .expect("target is a fallen plum");
            self.fruit.swap_remove(i);
            self.eaten = self.eaten.saturating_add(1);
            self.target = None;
            self.swallowed = true;
        } else if let Some(d) = r.step(t) {
            self.jelly_facing = d;
            self.jelly += d;
        }
    }
    fn step(&mut self, d: IVec2) {
        if distance(IVec2::ZERO, d) != 1 {
            return;
        }
        self.facing = d;
        let p = self.player + d;
        if !self.can_reach_from(self.player, p) {
            self.notice = "墙角或身旁的身影挡住去路，得从旁边绕。";
            return;
        }
        if self.area == Area::Cistern {
            if let Some(i) = self.expedition.enemy_at(p) {
                self.expedition.target_hit = None;
                self.expedition.strike(i, 1, false);
                self.notice = "刀锋落下：1伤。锁定格还亮着，就移动、Q格挡或F打断。";
                self.combat_beat(ActionKind::Strike, false);
                return;
            }
            if !self.walkable(p) {
                self.notice = self.describe(p);
                return;
            }
            self.player = p;
            self.expedition.target_hit = None;
            self.combat_beat(ActionKind::Step, false);
            return;
        }
        if !self.walkable(p) || p == self.jelly {
            self.notice = if self.there(p) != Here::None || p == TREE2 {
                self.describe(p)
            } else if !on_map(p) {
                "屋墙和深水拦住去路；北门石阶可以下到旧泉回水廊。"
            } else {
                "脚下没路，得从旁边绕。"
            };
            return;
        }
        self.player = p;
        self.beat(ActionKind::Step);
        if self.swallowed {
            self.notice = "小魔物咕嘟一下，草里少了一颗李子。";
        }
    }
    fn use_here(&mut self, h: Here, ui: &mut Interface) {
        match h {
            Here::Gate => self.descend(ui),
            Here::Herbalist => {
                ui.page = Page::Barter;
            }
            Here::Stairs => self.home(ui, false),
            Here::Fountain => {
                if self.expedition.quest == Quest::Carrying && self.expedition.wheel_home {
                    self.expedition.quest = Quest::Restored;
                    self.action = ActionKind::Restore;
                    self.turn = self.turn.saturating_add(1);
                    self.save_revision = self.save_revision.wrapping_add(1);
                    self.notice = "手轮起初拧不动。一阵闷响之后，第一股水从铜嘴里溅了出来。";
                    ui.page = Page::Homecoming;
                } else {
                    self.notice = self.describe(map::FOUNTAIN);
                }
            }
            Here::ToolChest if !self.expedition.seal_open => {
                ui.filling = if self.expedition.kit == Kit::Hook {
                    1
                } else {
                    0
                };
                ui.page = Page::Gear;
            }
            Here::ToolChest => self.notice = "本趟已选好器物；下次出发可以换一种打法。",
            Here::TonicChest if !self.expedition.tonic_open => {
                if self.expedition.tonics >= 3 {
                    self.notice = "药囊已经装满，余药还在柜里。";
                    return;
                }
                self.expedition.tonic_open = true;
                self.expedition.tonics = self.expedition.tonics.saturating_add(2).min(3);
                self.expedition.target_hit = None;
                self.notice = "把瓶塞按紧，药装进了囊里。受伤时可以喝一瓶。";
                self.combat_beat(ActionKind::Gather, false);
            }
            Here::TonicChest => self.notice = "本趟的药剂柜已经空了。",
            Here::Wheel if self.expedition.quest == Quest::Seeking => {
                self.expedition.quest = Quest::Carrying;
                self.expedition.wheel_home = false;
                self.notice = "黄铜手轮沉甸甸的。南面石阶还能看见天光，从那里带它回镇。";
                self.expedition.target_hit = None;
                self.combat_beat(ActionKind::Gather, false);
            }
            Here::Wheel if self.expedition.quest == Quest::Restored => {
                self.notice = "阀轮已在镇泉上转动。守卫铜片带回每四枚可增加1点体力上限，最高14。";
            }
            Here::Wheel => self.notice = "阀轮就在行囊中。由南端石阶亲自带回镇上。",
            Here::Enemy(p) => {
                self.facing = p - self.player;
                self.step(self.facing);
            }
            Here::Fruit(p) => {
                if self.plums == u8::MAX {
                    self.notice = "行囊装满了。";
                    return;
                }
                let i = self.fruit.iter().position(|&v| v == p).unwrap();
                self.fruit.swap_remove(i);
                self.plums += 1;
                self.notice = "把李子在斗篷上擦了擦，放进行囊。";
                self.beat(ActionKind::Gather);
            }
            Here::Bough if !self.tree_shaken => {
                self.tree_shaken = true;
                let batch = &HARVEST[((self.day - 1) % 3) as usize];
                for &at in batch {
                    // A previously tossed plum keeps its place; the next one rolls to a free neighbour.
                    let pos = std::iter::once(at)
                        .chain(DIRS.into_iter().map(|d| at + d))
                        .find(|p| self.walkable(*p) && !self.fruit.contains(p));
                    if let Some(p) = pos {
                        if self.fruit.len() < 32 {
                            self.fruit.push(p);
                        }
                    }
                }
                self.notice = "树梢轻轻一颤，熟李子接连落进草里。";
                self.beat(ActionKind::Shake);
            }
            Here::Bough => self.notice = "只剩硬硬的青果了。明早再来。",
            Here::Table if self.breakfast.is_some() => ui.page = Page::Served,
            Here::Table => {
                ui.filling = if self.plums > 0 { 1 } else { 0 };
                ui.page = Page::Bowl;
            }
            Here::Keeper => {
                self.expedition.hp = self.expedition.max_hp;
                self.notice = if self.expedition.quest == Quest::Restored {
                    "店主：「你听！我那把壶才放下，就灌满了。进来歇歇？」"
                } else {
                    "店主：「今早的水是我挑回来的。北门底下有只铜手轮，带得动吗？」"
                }
            }
            Here::Jelly => self.notice = self.describe(self.jelly),
            Here::None => self.notice = self.describe(self.player),
        }
    }
    fn serve(&mut self, ui: &mut Interface) {
        if self.breakfast.is_some() {
            ui.page = Page::Served;
            return;
        }
        let n = ui.filling.min(self.plums);
        self.plums -= n;
        self.breakfast = Some(n);
        self.beat(ActionKind::Eat);
        self.expedition.hp = self.expedition.max_hp;
        self.history.push(Meal {
            day: self.day,
            used: n,
            shared: self.eaten,
            kept: self.plums,
        });
        self.notice = if n == 0 {
            "瓷勺碰到碗底。今天的凝乳是原味的。"
        } else {
            "轻轻一搅，白凝乳里留下一道紫红的弧。"
        };
        ui.page = Page::Served;
        self.save_revision = self.save_revision.wrapping_add(1);
    }
    fn descend(&mut self, ui: &mut Interface) {
        self.area = Area::Cistern;
        self.player = map::STAIRS;
        self.facing = IVec2::NEG_Y;
        self.expedition.begin(self.day);
        self.action = ActionKind::Descend;
        self.turn = self.turn.saturating_add(1);
        self.save_revision = self.save_revision.wrapping_add(1);
        self.notice = "阶梯还晒得到太阳。西边是工匠间，东边的铜架上挂着手轮。";
        ui.page = Page::Roam;
    }
    fn home(&mut self, ui: &mut Interface, fallen: bool) {
        if fallen {
            self.expedition.enemies.clear();
            self.expedition.hp = self.expedition.max_hp;
            self.expedition.skill_ready = true;
            self.expedition.target_hit = None;
        } else {
            self.expedition.return_home(false);
        }
        self.area = Area::Town;
        self.player = if fallen { START } else { map::GATE + IVec2::Y };
        self.facing = IVec2::Y;
        self.action = ActionKind::Return;
        self.turn = self.turn.saturating_add(1);
        self.save_revision = self.save_revision.wrapping_add(1);
        self.notice = if fallen {
            "店主把你扶回门廊，体力恢复。本趟铜片与未带回阀轮已失落，已存铜片/已修泉水保留。北门可再试，先拿西箱、看红格再走。"
        } else if self.expedition.quest == Quest::Carrying {
            "铜阀轮安全带回了！铜片已存，体力恢复。到镇泉任意一侧按E安装；阀轮现在会保存在手记中。"
        } else {
            "主动撤回：铜片已存，体力恢复，装备和余药保留。每四枚铜片加固一点体力；北门随时能再出发。"
        };
        ui.page = Page::Roam;
        ui.cursor = self.player;
    }
    fn combat_beat(&mut self, kind: ActionKind, brace: bool) {
        self.action = kind;
        self.turn = self.turn.saturating_add(1);
        self.swallowed = false;
        if matches!(kind, ActionKind::Gather | ActionKind::Drink) {
            self.save_revision = self.save_revision.wrapping_add(1);
        }
        let (hurt, guarded) = self.expedition.advance(self.player, brace);
        if hurt > 0 {
            self.notice =
                if map::glyph(self.area, self.player) == b'P' && self.expedition.beat % 4 == 0 {
                    "石盖下的水柱把你掀了一下。喷气前，盖缝会先亮起来。"
                } else {
                    "这一下打中了。下次见脚下红格，移开或架盾。"
                };
        } else if guarded {
            self.notice = match (self.expedition.target_hit.is_some(), self.expedition.kit) {
                (true, Kit::Buckler) => "盾沿撞回了近处的对手：反击 2。盾击已就绪。",
                (true, Kit::Hook) => "你拨开来势，近身回击 1。钩索已可再用。",
                (true, Kit::None) => "你挡下这一击，趁对手贴身还了一下：反击 1。",
                (false, _) => "你护住脸和胸口，挡下了远处的一击。",
            };
        } else if kind == ActionKind::Step {
            self.notice = if self.danger_at(self.player) {
                "脚下出现预兆：下一次行动会命中这里。移开一格；敌击可Q，喷气必须躲开。"
            } else if crate::delve::safe(self.player) {
                "石阶日光下是安全格。E可带着本趟收获撤回镇上。"
            } else {
                "脚步声在石廊里传得很远。"
            };
        }
        if self.expedition.hp == 0 {
            self.save_revision = self.save_revision.wrapping_add(1);
            self.expedition.deaths = self.expedition.deaths.saturating_add(1);
            self.expedition.loot = 0;
            if !self.expedition.wheel_home && self.expedition.quest == Quest::Carrying {
                self.expedition.quest = Quest::Seeking;
            }
            self.notice =
                "你倒在回水廊里。本趟铜片和未带回的阀轮失落；E回店恢复体力，再从北门挑战。";
        }
    }
    fn brace(&mut self) {
        self.expedition.target_hit = None;
        self.expedition.skill_ready = true;
        self.notice = "架势稳住一拍：挡下本拍敌击并反击邻敌；技艺恢复。喷气仍需走位躲开。";
        self.combat_beat(ActionKind::Brace, true);
    }
    fn skill(&mut self) {
        if self.expedition.kit == Kit::None {
            self.notice = "还没有器物。西翼器物箱提供圆盾或钩索；基础Q仍能格挡反击。";
            return;
        }
        if !self.expedition.skill_ready {
            self.notice = "技艺尚未就绪。Q格挡或空格等候一拍便能恢复。";
            return;
        }
        let reach = if self.expedition.kit == Kit::Hook {
            2
        } else {
            1
        };
        let mut target = None;
        for range in 1..=reach {
            let p = self.player + self.facing * range;
            if !map::passage(p - self.facing, p, |at| {
                self.walkable(at) && (at == p || !self.blocked(at))
            }) {
                break;
            }
            if let Some(i) = self.expedition.enemy_at(p) {
                target = Some((i, range));
                break;
            }
        }
        let Some((i, range)) = target else {
            self.notice = "面向目标再用F：圆盾打正面邻格，钩索可打直线两格，不能穿墙。";
            return;
        };
        self.expedition.target_hit = None;
        self.expedition.strike(i, 2, true);
        if self.expedition.kit == Kit::Hook && range == 2 {
            let near = self.player + self.facing;
            if self.expedition.enemy_at(near).is_none() {
                self.expedition.enemies[i].pos = near;
            }
        }
        self.expedition.skill_ready = false;
        self.notice = if self.expedition.kit == Kit::Hook {
            "钩索命中2伤，把远敌拖到面前并打断一拍。Q或等候后再次就绪。"
        } else {
            "盾击2伤，打断蓄势并压住一拍。Q可强力反击，也会恢复盾击。"
        };
        self.combat_beat(ActionKind::Skill, false);
    }
    fn drink(&mut self) {
        if self.expedition.hp == self.expedition.max_hp {
            self.notice = "体力充足，药剂留在行囊里。";
            return;
        }
        if self.expedition.tonics == 0 {
            self.notice = "药剂用完了。西翼药柜可补两瓶；也可从南端石阶带着收获撤回恢复。";
            return;
        }
        self.expedition.tonics -= 1;
        self.expedition.hp = self
            .expedition
            .hp
            .saturating_add(4)
            .min(self.expedition.max_hp);
        self.expedition.target_hit = None;
        self.notice = "饮下一瓶药，体力恢复4点。喝药也推进一拍，先离开预警格更安全。";
        if self.area == Area::Cistern {
            self.combat_beat(ActionKind::Drink, false);
        } else {
            self.beat(ActionKind::Drink);
        }
    }
    fn morning(&mut self, ui: &mut Interface) {
        let carried = self.plums;
        let day = self.day.saturating_add(1);
        let revision = self.save_revision.wrapping_add(1);
        let expedition = std::mem::take(&mut self.expedition);
        let history = std::mem::take(&mut self.history);
        *self = Self::new(history);
        self.day = day;
        self.plums = carried;
        self.expedition = expedition;
        self.expedition.hp = self.expedition.max_hp;
        self.save_revision = revision;
        self.notice = "又一个晴天。泉水与已带回的收获都还在；李树成熟，回水廊的守卫换了巡路。";
        ui.page = Page::Roam;
        ui.cursor = START;
        ui.filling = 0;
    }
    pub fn act(&mut self, ui: &mut Interface, intent: Intent) {
        if self.area == Area::Cistern && self.expedition.hp == 0 {
            ui.page = Page::Down;
            if matches!(intent, Intent::Interact) {
                self.home(ui, true);
            }
            return;
        }
        if matches!(intent, Intent::Book) {
            ui.page = if ui.page == Page::Book {
                Page::Roam
            } else {
                Page::Book
            };
            return;
        }
        if matches!(intent, Intent::Cancel) {
            ui.page = if ui.page == Page::Roam {
                Page::Book
            } else {
                Page::Roam
            };
            return;
        }
        match (ui.page, intent) {
            (Page::Roam, Intent::Brace) if self.area == Area::Cistern => self.brace(),
            (Page::Roam, Intent::Skill) if self.area == Area::Cistern => self.skill(),
            (Page::Roam, Intent::Drink) => self.drink(),
            (Page::Barter, Intent::Interact) if self.plums != 0 && self.expedition.tonics < 3 => {
                self.plums -= 1;
                self.expedition.tonics += 1;
                self.beat(ActionKind::Gather);
                self.notice = "药师收下李子，把软木塞压紧：「别放在装钥匙的那个兜里。」";
                ui.page = Page::Roam;
            }
            (Page::Gear, Intent::Less) => ui.filling = 0,
            (Page::Gear, Intent::More) => ui.filling = 1,
            (Page::Gear, Intent::Dir(d)) if d.x < 0 => ui.filling = 0,
            (Page::Gear, Intent::Dir(d)) if d.x > 0 => ui.filling = 1,
            (Page::Gear, Intent::Interact) => {
                self.expedition.kit = if ui.filling == 0 {
                    Kit::Buckler
                } else {
                    Kit::Hook
                };
                self.expedition.seal_open = true;
                self.expedition.skill_ready = true;
                self.expedition.target_hit = None;
                ui.page = Page::Roam;
                self.notice = if self.expedition.kit == Kit::Buckler {
                    "圆盾上手：Q格挡并反击2伤；F正面盾击2伤、打断蓄势。Q或等候恢复技艺。"
                } else {
                    "钩索上手：F正面两格2伤，拉近并打断；Q反击1伤。Q或等候恢复技艺。"
                };
                self.combat_beat(ActionKind::Gather, false);
            }
            (Page::Homecoming | Page::Served, Intent::Interact) => ui.page = Page::Roam,
            (Page::Served, Intent::NewMorning) if self.area == Area::Town => self.morning(ui),
            (Page::Roam, Intent::Dir(d)) => self.step(d),
            (Page::Roam, Intent::Interact) => self.use_here(self.nearby(), ui),
            (Page::Roam, Intent::Wait) => {
                if self.area == Area::Cistern {
                    self.expedition.target_hit = None;
                    self.expedition.skill_ready = true;
                    self.notice = "稳住呼吸等候一拍，技艺恢复。看着敌人的预兆选择下一步。";
                    self.combat_beat(ActionKind::Wait, false);
                } else {
                    self.beat(ActionKind::Wait);
                    self.notice = if self.swallowed {
                        "小家伙咽下果子，满足地晃了晃。"
                    } else {
                        "镇上钟声悠悠，北门与泉台就在石路尽头。"
                    };
                }
            }
            (Page::Roam, Intent::Look) => {
                ui.page = Page::Look;
                ui.cursor = self.player;
            }
            (Page::Look, Intent::Look) | (Page::Aim, Intent::Toss) => ui.page = Page::Roam,
            (Page::Roam, Intent::Toss) if self.area == Area::Town && self.plums > 0 => {
                ui.page = Page::Aim;
                ui.cursor = self.player + self.facing;
                if !self.can_toss(ui.cursor) {
                    ui.cursor = DIRS
                        .iter()
                        .map(|&d| self.player + d)
                        .find(|&p| self.can_toss(p))
                        .unwrap_or(self.player);
                }
            }
            (Page::Roam, Intent::Toss) => self.notice = "行囊里没有李子。先拾一颗。",
            (Page::Aim | Page::Look, Intent::Dir(d)) => {
                let p = ui.cursor + d;
                if on_map(p) && (ui.page == Page::Look || distance(self.player, p) <= 3) {
                    ui.cursor = p;
                }
            }
            (Page::Aim | Page::Look, Intent::Point(p)) if on_map(p) => {
                if ui.cursor == p {
                    self.act(ui, Intent::Interact);
                } else if ui.page == Page::Look || distance(self.player, p) <= 3 {
                    ui.cursor = p;
                }
            }
            (Page::Look, Intent::Interact) => {
                self.notice = self.describe(ui.cursor);
                ui.page = Page::Roam;
            }
            (Page::Aim, Intent::Interact) if self.can_toss(ui.cursor) => {
                self.plums -= 1;
                self.fruit.push(ui.cursor);
                self.notice = "李子越过草尖，扑地落下。";
                self.beat(ActionKind::Throw {
                    from: self.player,
                    to: ui.cursor,
                });
                ui.page = Page::Roam;
            }
            (Page::Aim, Intent::Interact) => self.notice = "挑三步内的一块空地。",
            (Page::Roam, Intent::Point(p)) if self.adjacent_object(p) => {
                if distance(self.player, p) == 1 {
                    self.facing = p - self.player;
                }
                self.use_here(self.there(p), ui);
            }
            (Page::Bowl, Intent::Less) => ui.filling = ui.filling.saturating_sub(1),
            (Page::Bowl, Intent::More) => ui.filling = ui.filling.saturating_add(1).min(self.plums),
            (Page::Bowl, Intent::Interact) => self.serve(ui),
            _ => {}
        }
        if self.area == Area::Cistern && self.expedition.hp == 0 {
            ui.page = Page::Down;
        }
    }
    pub fn adjacent_object(&self, p: IVec2) -> bool {
        if self.area == Area::Town && matches!(map::glyph(self.area, p), b'f' | b'F') {
            return self.fountain_reach_from(self.player);
        }
        self.can_reach_from(self.player, p) && self.there(p) != Here::None
    }
}

// Completed town state is durable. Leaving the program underground abandons the sortie:
// consumed supplies/attempt count remain spent; loose coins and an unbanked wheel do not.
#[derive(Resource)]
pub struct JournalFile(Option<PathBuf>);

#[derive(Serialize, Deserialize)]
struct SavedTown<H = Vec<Meal>, F = Vec<[i32; 2]>> {
    version: u8,
    day: u32,
    plums: u8,
    eaten: u8,
    tree_shaken: bool,
    breakfast: Option<u8>,
    history: H,
    fruit: F,
    jelly: [i32; 2],
    banked: u32,
    kit: Kit,
    quest: Quest,
    tonics: u8,
    attempts: u32,
    deaths: u32,
    retreats: u32,
    abandoned: bool,
}
#[derive(Deserialize)]
#[serde(untagged)]
enum SavedJournal {
    Town(SavedTown),
    Legacy(Vec<Meal>),
}
struct GroundFruit<'a>(&'a [IVec2]);
impl Serialize for GroundFruit<'_> {
    fn serialize<S: serde::Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        s.collect_seq(self.0.iter().map(|p| p.to_array()))
    }
}
impl SavedTown {
    fn from_game(g: &Outing) -> SavedTown<&[Meal], GroundFruit<'_>> {
        SavedTown {
            version: 2,
            day: g.day,
            plums: g.plums,
            eaten: g.eaten,
            tree_shaken: g.tree_shaken,
            breakfast: g.breakfast,
            history: &g.history,
            fruit: GroundFruit(&g.fruit),
            jelly: g.jelly.to_array(),
            banked: g.expedition.banked,
            kit: g.expedition.kit,
            quest: if g.expedition.quest == Quest::Carrying && !g.expedition.wheel_home {
                Quest::Seeking
            } else {
                g.expedition.quest
            },
            tonics: g.expedition.tonics,
            attempts: g.expedition.attempts,
            deaths: g.expedition.deaths,
            retreats: g.expedition.retreats,
            abandoned: g.area == Area::Cistern,
        }
    }
    fn into_game(self) -> Outing {
        let mut g = Outing::new(self.history);
        g.day = self.day.max(1);
        g.plums = self.plums;
        g.eaten = self.eaten;
        g.tree_shaken = self.tree_shaken;
        g.breakfast = self.breakfast;
        g.fruit.clear();
        for [x, y] in self.fruit.into_iter().take(32) {
            let p = IVec2::new(x, y);
            if map::passable(Area::Town, p) && !g.fruit.contains(&p) {
                g.fruit.push(p);
            }
        }
        let jelly = IVec2::new(self.jelly[0], self.jelly[1]);
        if map::passable(Area::Town, jelly) && jelly != START {
            g.jelly = jelly;
        }
        g.expedition.banked = self.banked;
        g.expedition.max_hp = (8 + (self.banked / 4).min(6)) as u8;
        g.expedition.hp = g.expedition.max_hp;
        g.expedition.kit = self.kit;
        g.expedition.quest = self.quest;
        g.expedition.wheel_home = self.quest != Quest::Seeking;
        g.expedition.tonics = self.tonics.min(3);
        g.expedition.attempts = self.attempts;
        g.expedition.deaths = self.deaths;
        g.expedition.retreats = self.retreats;
        g.notice = if self.abandoned {
            "你在铜壶店醒来。上趟没带上台阶的东西还在井下；存好的铜片、器物和药囊都在身边。"
        } else if self.quest == Quest::Restored {
            "泉水仍在流淌。修复与早餐已记下；北门回水廊还有铜片可带回，加固行装。"
        } else if self.quest == Quest::Carrying {
            "安全带回的铜阀轮还在行囊里。去镇泉任意一侧按E安装。"
        } else {
            "镇泉仍缺一只铜阀轮。北门下回水廊：西翼整备，东台取轮，再由南阶亲自带回。"
        };
        g
    }
}
impl JournalFile {
    pub fn open(ephemeral: bool) -> Self {
        if ephemeral {
            return Self(None);
        }
        let dir = std::env::var_os("XDG_DATA_HOME")
            .filter(|p| !p.is_empty())
            .map(PathBuf::from)
            .or_else(|| std::env::var_os("HOME").map(|p| PathBuf::from(p).join(".local/share")));
        Self(dir.map(|p| p.join("wanderstead/plum-mornings.json")))
    }
    pub fn resume(&self) -> Outing {
        let Some(path) = &self.0 else {
            return Outing::new(Vec::new());
        };
        match fs::read(path) {
            Ok(bytes) => match serde_json::from_slice::<SavedJournal>(&bytes) {
                Ok(SavedJournal::Town(saved)) => saved.into_game(),
                Ok(SavedJournal::Legacy(history)) => Outing::new(history),
                Err(e) => {
                    eprintln!("手记读不出：{e}");
                    Outing::new(Vec::new())
                }
            },
            Err(e) if e.kind() == io::ErrorKind::NotFound => Outing::new(Vec::new()),
            Err(e) => {
                eprintln!("手记无法打开：{e}");
                Outing::new(Vec::new())
            }
        }
    }
    pub fn save(&self, game: &Outing) -> io::Result<()> {
        let Some(p) = &self.0 else {
            return Ok(());
        };
        fs::create_dir_all(p.parent().unwrap())?;
        let tmp = p.with_extension("new");
        fs::write(&tmp, serde_json::to_vec(&SavedTown::from_game(game))?)?;
        fs::rename(tmp, p)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn cistern_with_enemy(player: IVec2, enemy: IVec2) -> Outing {
        let mut g = Outing::new(vec![]);
        g.area = Area::Cistern;
        g.player = player;
        g.expedition.begin(1);
        g.expedition.enemies.truncate(1);
        g.expedition.enemies[0].pos = enemy;
        g
    }

    #[test]
    fn diagonal_is_one_step_and_one_turn_and_rejects_large_vectors() {
        let mut g = Outing::new(vec![]);
        let mut ui = Interface::default();
        let from = IVec2::new(-2, 0);
        g.player = from;
        let d = IVec2::new(1, -1);
        g.act(&mut ui, Intent::Dir(d));
        assert_eq!((g.player, g.facing, g.turn), (from + d, d, 1));
        g.act(&mut ui, Intent::Dir(d * 2));
        assert_eq!((g.player, g.turn), (from + d, 1));
    }

    #[test]
    fn preview_and_click_stop_at_the_nearest_legal_interaction_stand() {
        let mut g = Outing::new(vec![]);
        g.player = IVec2::new(-2, 0);
        g.fruit.clear();
        let target = IVec2::new(0, -2);
        g.fruit.push(target);
        let mut path = Vec::new();
        assert!(g.path_to(target, &mut path));
        assert_eq!(path, [IVec2::new(-1, -1)]);
        let mut ui = Interface::default();
        for p in path {
            g.act(&mut ui, Intent::Dir(p - g.player));
        }
        assert!(g.adjacent_object(target));
        g.act(&mut ui, Intent::Point(target));
        assert_eq!((g.player, g.turn, g.plums), (IVec2::new(-1, -1), 2, 1));
    }

    #[test]
    fn routes_detour_around_wall_corners_and_actor_shoulders() {
        let mut g = Outing::new(vec![]);
        g.player = IVec2::new(1, 0);
        let target = IVec2::new(2, 1);
        let mut path = Vec::new();
        assert!(g.path_to(target, &mut path));
        assert_eq!(path.len(), 2);
        assert_ne!(path[0], target);
        let mut from = g.player;
        for &to in &path {
            assert!(map::passage(from, to, |p| g.walkable(p) && !g.blocked(p)));
            from = to;
        }
        g.player = IVec2::new(-2, 0);
        g.jelly = IVec2::new(-1, 0);
        g.act(&mut Interface::default(), Intent::Dir(IVec2::new(1, -1)));
        assert_eq!((g.player, g.turn), (IVec2::new(-2, 0), 0));
        assert!(g.path_to(IVec2::new(-1, -1), &mut path));
        assert_eq!(path.len(), 2);
    }

    #[test]
    fn diagonal_melee_and_hook_cannot_cross_a_wall_corner() {
        // (3,-3) is the blocked shoulder, although both actors stand on dry floor.
        let mut g = cistern_with_enemy(IVec2::new(2, -3), IVec2::new(3, -2));
        let mut ui = Interface::default();
        let hp = g.expedition.enemies[0].hp;
        g.act(&mut ui, Intent::Dir(IVec2::new(1, 1)));
        g.act(&mut ui, Intent::Point(IVec2::new(3, -2)));
        assert_eq!((g.turn, g.expedition.enemies[0].hp), (0, hp));
        assert!(!g.adjacent_object(IVec2::new(3, -2)));
        g.expedition.kit = Kit::Hook;
        g.act(&mut ui, Intent::Skill);
        assert_eq!((g.turn, g.expedition.enemies[0].hp), (0, hp));
    }

    #[test]
    fn diagonal_melee_and_two_cell_hook_each_commit_one_combat_beat() {
        let mut g = cistern_with_enemy(IVec2::new(0, -2), IVec2::new(1, -1));
        let mut ui = Interface::default();
        g.act(&mut ui, Intent::Point(IVec2::new(1, -1)));
        assert_eq!(
            (g.turn, g.expedition.beat, g.expedition.enemies[0].hp),
            (1, 1, 2)
        );
        assert_eq!(g.player, IVec2::new(0, -2));
        g.expedition.enemies[0].pos = IVec2::new(2, 0);
        g.expedition.enemies[0].hp = 3;
        g.expedition.kit = Kit::Hook;
        g.facing = IVec2::new(1, 1);
        g.act(&mut ui, Intent::Skill);
        assert_eq!((g.turn, g.expedition.beat), (2, 2));
        assert_eq!(
            (g.expedition.enemies[0].pos, g.expedition.enemies[0].hp),
            (IVec2::new(1, -1), 1)
        );
    }

    #[test]
    fn wall_corner_blocks_object_reach_and_high_obstacles_block_lobs() {
        let mut g = Outing::new(vec![]);
        g.player = IVec2::new(1, 0);
        let fruit = IVec2::new(2, 1);
        g.fruit.push(fruit);
        assert!(!g.adjacent_object(fruit));
        let mut ui = Interface::default();
        g.act(&mut ui, Intent::Point(fruit));
        assert_eq!((g.turn, g.plums), (0, 0));
        g.player = IVec2::new(2, 2);
        g.plums = 1;
        assert!(!g.can_toss(IVec2::new(6, 2)));
        assert!(!g.can_toss(IVec2::new(5, 0)));
        g.player = IVec2::new(0, 0);
        assert!(g.can_toss(IVec2::new(-3, -3)));
    }

    #[test]
    fn actor_shoulders_block_melee_hook_and_fountain_corner_reach() {
        let mut g = cistern_with_enemy(IVec2::new(0, -2), IVec2::new(2, 0));
        g.expedition.begin(1);
        g.expedition.enemies.truncate(2);
        g.expedition.enemies[0].pos = IVec2::new(2, 0);
        g.expedition.enemies[1].pos = IVec2::new(1, -2);
        g.facing = IVec2::new(1, 1);
        g.expedition.kit = Kit::Hook;
        let hp = g.expedition.enemies[0].hp;
        let mut ui = Interface::default();
        g.act(&mut ui, Intent::Skill);
        assert_eq!((g.turn, g.expedition.enemies[0].hp), (0, hp));
        g.expedition.enemies[0].pos = IVec2::new(1, -1);
        g.act(&mut ui, Intent::Point(IVec2::new(1, -1)));
        assert_eq!((g.turn, g.expedition.enemies[0].hp), (0, hp));

        let mut g = Outing::new(vec![]);
        g.player = IVec2::new(6, 0);
        assert!(g.adjacent_object(map::FOUNTAIN));
        let mut route = Vec::new();
        assert!(g.path_to(map::FOUNTAIN, &mut route));
        assert!(route.is_empty());
        g.jelly = IVec2::new(5, 0);
        assert!(!g.adjacent_object(map::FOUNTAIN));
        assert!(g.path_to(map::FOUNTAIN, &mut route));
        assert!(!route.is_empty());
    }
    #[test]
    fn a_plum_only_changes_hands_for_a_bottle_that_fits() {
        let mut g = Outing::new(vec![]);
        g.player = map::APOTHECARY + IVec2::X;
        g.plums = 1;
        g.expedition.tonics = 2;
        let mut ui = Interface::default();
        g.act(&mut ui, Intent::Point(map::APOTHECARY));
        g.act(&mut ui, Intent::Cancel);
        assert_eq!((g.plums, g.expedition.tonics, g.turn), (1, 2, 0));
        g.act(&mut ui, Intent::Point(map::APOTHECARY));
        g.act(&mut ui, Intent::Interact);
        assert_eq!((g.plums, g.expedition.tonics, g.turn), (0, 3, 1));
        g.plums = 1;
        g.act(&mut ui, Intent::Point(map::APOTHECARY));
        g.act(&mut ui, Intent::Interact);
        assert_eq!((g.plums, g.expedition.tonics, g.turn), (1, 3, 1));
    }
    #[test]
    fn over_the_wall_changes_who_gets_the_same_fruit() {
        let mut g = Outing::new(vec![]);
        g.player = IVec2::new(3, 0);
        g.plums = 1;
        g.fruit.clear();
        g.fruit.extend(HARVEST[0]);
        g.target = Some(IVec2::new(5, -2));
        let mut ui = Interface {
            page: Page::Aim,
            cursor: IVec2::new(1, 0),
            ..default()
        };
        g.act(&mut ui, Intent::Interact);
        assert_eq!(g.plums, 0);
        assert_eq!(g.target, Some(IVec2::new(1, 0)));
        let mut trail = vec![g.jelly];
        while g.eaten == 0 && g.turn < 20 {
            g.act(&mut ui, Intent::Wait);
            trail.push(g.jelly);
        }
        assert_eq!(g.jelly, IVec2::new(1, 0));
        assert_eq!(g.eaten, 1);
        assert!(trail.iter().all(|p| map::passable(Area::Town, *p)));
        assert_eq!(g.fruit.len(), 4);
    }
    #[test]
    fn menus_and_failed_throws_dont_spend_a_plum_or_turn() {
        let mut g = Outing::new(vec![]);
        let mut ui = Interface::default();
        g.act(&mut ui, Intent::Interact);
        g.act(&mut ui, Intent::Toss);
        ui.cursor = g.player;
        g.act(&mut ui, Intent::Interact);
        assert_eq!((g.plums, g.turn, ui.page), (1, 1, Page::Aim));
        g.act(&mut ui, Intent::Cancel);
        g.act(&mut ui, Intent::Look);
        g.act(&mut ui, Intent::Dir(IVec2::X));
        g.act(&mut ui, Intent::Interact);
        assert_eq!(g.turn, 1);
    }
    #[test]
    fn fallen_plum_at_your_boots_is_reachable_while_facing_the_bare_tree() {
        let mut g = Outing::new(vec![]);
        let mut ui = Interface::default();
        g.player = TREE + IVec2::Y;
        g.facing = IVec2::NEG_Y;
        g.act(&mut ui, Intent::Interact);
        assert!(g.tree_shaken);
        g.act(&mut ui, Intent::Interact);
        assert_eq!(g.plums, 1);
        assert!(!g.fruit.contains(&g.player));
    }
    #[test]
    fn serving_preserves_unspent_fruit_and_a_finished_memory() {
        let mut g = Outing::new(vec![]);
        g.player = TABLE + IVec2::Y;
        g.facing = IVec2::NEG_Y;
        g.plums = 3;
        let mut ui = Interface::default();
        g.act(&mut ui, Intent::Interact);
        g.act(&mut ui, Intent::More);
        g.act(&mut ui, Intent::Interact);
        assert_eq!(g.breakfast, Some(2));
        assert_eq!(g.history[0].kept, 1);
        g.act(&mut ui, Intent::NewMorning);
        assert_eq!((g.day, g.plums, g.turn, g.history.len()), (2, 1, 0, 1));
        assert!(!g.tree_shaken);
    }
    #[test]
    fn fountain_center_and_every_footprint_cell_find_a_real_rim() {
        let g = Outing::new(vec![]);
        let mut path = Vec::with_capacity(CELLS);
        for y in 1..=3 {
            for x in 3..=5 {
                assert!(g.path_to(IVec2::new(x, y), &mut path));
                assert!(path.iter().all(|p| g.walkable(*p)));
                let end = path.last().copied().unwrap_or(g.player);
                assert!(
                    DIRS.iter()
                        .any(|d| matches!(map::glyph(Area::Town, end + *d), b'f' | b'F'))
                );
            }
        }
        assert!(cell(IVec2::new(8, 5)) > u8::MAX as usize);
    }
    #[test]
    fn wheel_must_return_before_install_and_survives_morning_and_reload() {
        let mut g = Outing::new(vec![]);
        let mut ui = Interface::default();
        g.descend(&mut ui);
        g.expedition.enemies.clear();
        g.player = map::RELIC + IVec2::Y;
        g.facing = IVec2::NEG_Y;
        g.act(&mut ui, Intent::Interact);
        assert_eq!(g.expedition.quest, Quest::Carrying);
        assert_eq!(SavedTown::from_game(&g).quest, Quest::Seeking);
        g.player = map::STAIRS;
        g.act(&mut ui, Intent::Interact);
        assert_eq!(g.area, Area::Town);
        assert_ne!(g.player, map::GATE);
        assert_eq!(SavedTown::from_game(&g).quest, Quest::Carrying);
        g.player = IVec2::new(2, 2);
        g.act(&mut ui, Intent::Point(map::FOUNTAIN));
        assert_eq!(g.expedition.quest, Quest::Restored);
        g.act(&mut ui, Intent::Interact);
        g.morning(&mut ui);
        let saved = serde_json::to_vec(&SavedTown::from_game(&g)).unwrap();
        let loaded = serde_json::from_slice::<SavedTown>(&saved)
            .unwrap()
            .into_game();
        assert_eq!(loaded.expedition.quest, Quest::Restored);
    }
    #[test]
    fn death_spends_harvest_and_retries_without_blocking_the_stairs() {
        let mut g = Outing::new(vec![]);
        let mut ui = Interface::default();
        g.descend(&mut ui);
        g.expedition.quest = Quest::Carrying;
        g.expedition.loot = 3;
        g.expedition.hp = 1;
        g.expedition.enemies.clear();
        g.player = IVec2::new(0, 0);
        g.expedition.beat = 3;
        g.act(&mut ui, Intent::Wait);
        assert_eq!(ui.page, Page::Down);
        assert_eq!((g.expedition.loot, g.expedition.quest), (0, Quest::Seeking));
        g.act(&mut ui, Intent::Cancel);
        assert_eq!(ui.page, Page::Down);
        g.act(&mut ui, Intent::Interact);
        assert_eq!((g.area, g.expedition.hp), (Area::Town, g.expedition.max_hp));
        g.descend(&mut ui);
        assert_eq!(g.player, map::STAIRS);
        assert!(!g.expedition.enemies.is_empty());
    }
    #[test]
    fn potion_is_consumed_only_when_useful_and_legacy_meals_are_preserved() {
        let mut g = Outing::new(vec![]);
        let mut ui = Interface::default();
        g.act(&mut ui, Intent::Drink);
        assert_eq!((g.expedition.tonics, g.turn), (1, 0));
        g.expedition.hp = 1;
        g.act(&mut ui, Intent::Drink);
        assert_eq!((g.expedition.tonics, g.expedition.hp), (0, 5));
        let legacy = br#"[{"day":4,"used":2,"shared":1,"kept":3}]"#;
        let SavedJournal::Legacy(meals) = serde_json::from_slice(legacy).unwrap() else {
            panic!("legacy format lost")
        };
        let old = Outing::new(meals);
        assert_eq!((old.day, old.plums, old.history[0].used), (5, 3, 2));
    }
}
