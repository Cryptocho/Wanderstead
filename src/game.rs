use bevy::prelude::*;
use serde::{Deserialize, Serialize};
use std::{fs, io, path::PathBuf};

pub const START: IVec2 = IVec2::new(-2, 3);
pub const TABLE: IVec2 = IVec2::new(-4, 3);
pub const KEEPER: IVec2 = IVec2::new(-5, 3);
pub const FIRST_PLUM: IVec2 = IVec2::new(-3, 3);
pub const TREE: IVec2 = IVec2::new(5, -4);
pub const TREE2: IVec2 = IVec2::new(8, -2);
pub const WALL_START: IVec2 = IVec2::new(2, -3);
pub const JELLY_START: IVec2 = IVec2::new(4, -2);
const W: usize = 17;
const H: usize = 11;
const CELLS: usize = W * H;
const DIRS: [IVec2; 4] = [IVec2::X, IVec2::NEG_Y, IVec2::Y, IVec2::NEG_X];

pub fn ground(p: IVec2) -> Vec3 {
    Vec3::new(p.x as f32, 0., p.y as f32)
}
pub fn on_map(p: IVec2) -> bool {
    (-8..=8).contains(&p.x) && (-5..=5).contains(&p.y)
}
pub fn distance(a: IVec2, b: IVec2) -> u32 {
    a.x.abs_diff(b.x) + a.y.abs_diff(b.y)
}
pub fn walkable(p: IVec2) -> bool {
    on_map(p)
        && !(p.x <= -5 && (-1..=2).contains(&p.y))
        && !(p.x == WALL_START.x && (WALL_START.y..=0).contains(&p.y))
        && ![TABLE, KEEPER, TREE, TREE2].contains(&p)
}
fn cell(p: IVec2) -> usize {
    (p.x + 8) as usize + W * (p.y + 5) as usize
}

// One flood per decision. Distances and first steps fit in a byte on this map.
struct Routes {
    dist: [u8; CELLS],
    first: [u8; CELLS],
}
impl Routes {
    fn from(origin: IVec2, occupied: IVec2) -> Self {
        let mut r = Self {
            dist: [u8::MAX; CELLS],
            first: [4; CELLS],
        };
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
                if n == occupied || !walkable(n) {
                    continue;
                }
                let j = cell(n);
                if r.dist[j] != u8::MAX {
                    continue;
                }
                r.dist[j] = r.dist[i] + 1;
                r.first[j] = if p == origin { d as u8 } else { r.first[i] };
                q[end] = n;
                end += 1;
            }
        }
        r
    }
    fn step(&self, dest: IVec2) -> Option<IVec2> {
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
            hud: true,
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
}
#[derive(Clone, Copy, PartialEq, Eq)]
enum Here {
    Fruit(IVec2),
    Table,
    Bough,
    Keeper,
    Jelly,
    None,
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
        let mut fruit = Vec::with_capacity(8);
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
            notice: "壶下的桌上摆着早餐。门旁滚了一颗李子。",
        }
    }
    fn there(&self, p: IVec2) -> Here {
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
    fn nearby(&self) -> Here {
        [
            self.player,
            self.player + self.facing,
            self.player + IVec2::NEG_Y,
            self.player + IVec2::X,
            self.player + IVec2::Y,
            self.player + IVec2::NEG_X,
        ]
        .into_iter()
        .map(|p| self.there(p))
        .find(|h| *h != Here::None)
        .unwrap_or(Here::None)
    }
    pub fn prompt(&self) -> &'static str {
        match self.nearby() {
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
        } else if !walkable(p) {
            "石墙和屋脚都结实得很。低墙挡住脚，挡不住抛过去的李子。"
        } else if p.x > 2 {
            "树下的草被压出细软的小径。"
        } else {
            "车辙中间露出干燥的浅色石头。"
        }
    }
    pub fn can_toss(&self, p: IVec2) -> bool {
        self.plums > 0
            && walkable(p)
            && (1..=3).contains(&distance(self.player, p))
            && p != self.jelly
            && !self.fruit.contains(&p)
    }
    pub fn route(&self, dest: IVec2) -> Option<IVec2> {
        if !on_map(dest) || self.player == dest {
            return None;
        }
        let r = Routes::from(self.player, self.jelly);
        if walkable(dest) && dest != self.jelly {
            return r.step(dest);
        }
        let stand = DIRS
            .iter()
            .map(|d| dest + *d)
            .filter(|&p| walkable(p) && p != self.jelly)
            .min_by_key(|&p| r.dist[cell(p)])?;
        r.step(stand)
    }
    fn beat(&mut self, kind: ActionKind) {
        self.action = kind;
        self.turn = self.turn.saturating_add(1);
        self.swallowed = false;
        if self.target.is_none() && !self.fruit.iter().any(|&p| distance(self.jelly, p) <= 5) {
            return;
        }
        if self.fruit.is_empty() {
            self.target = None;
            return;
        }
        let r = Routes::from(self.jelly, self.player);
        let heard = match kind {
            ActionKind::Throw { to, .. } if distance(self.jelly, to) <= 5 => Some(to),
            _ => None,
        };
        let old = self
            .target
            .filter(|p| self.fruit.contains(p) && r.dist[cell(*p)] != u8::MAX);
        self.target = heard
            .filter(|p| r.dist[cell(*p)] != u8::MAX)
            .or(old)
            .or_else(|| {
                self.fruit
                    .iter()
                    .copied()
                    .filter(|&p| distance(self.jelly, p) <= 5 && r.dist[cell(p)] != u8::MAX)
                    .min_by_key(|&p| (r.dist[cell(p)], p.x, p.y))
            });
        let Some(t) = self.target else {
            return;
        };
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
        self.facing = d;
        let p = self.player + d;
        if !walkable(p) || p == self.jelly {
            self.notice = if self.there(p) != Here::None || p == TREE2 {
                self.describe(p)
            } else if !on_map(p) {
                "早餐还在旅店。这次就走到这里吧。"
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
                        .find(|p| walkable(*p) && !self.fruit.contains(p));
                    if let Some(p) = pos {
                        self.fruit.push(p);
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
                self.notice = if self.breakfast.is_some() {
                    "店主：碗搁这儿就好。明天见。"
                } else {
                    "店主：白的也好吃。要是嫌酸，树下有熟李子。"
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
    }
    pub fn act(&mut self, ui: &mut Interface, intent: Intent) {
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
            (Page::Roam, Intent::Dir(d)) => self.step(d),
            (Page::Roam, Intent::Interact) => self.use_here(self.nearby(), ui),
            (Page::Roam, Intent::Wait) => {
                self.beat(ActionKind::Wait);
                self.notice = if self.swallowed {
                    "小家伙咽下果子，满足地晃了晃。"
                } else {
                    "你停下脚，看了它一会儿。"
                };
            }
            (Page::Roam, Intent::Look) => {
                ui.page = Page::Look;
                ui.cursor = self.player;
            }
            (Page::Look, Intent::Look) | (Page::Aim, Intent::Toss) => ui.page = Page::Roam,
            (Page::Roam, Intent::Toss) if self.plums > 0 => {
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
            (Page::Roam, Intent::Point(p))
                if distance(self.player, p) <= 1 && self.there(p) != Here::None =>
            {
                if p != self.player {
                    self.facing = p - self.player;
                }
                self.use_here(self.there(p), ui);
            }
            (Page::Bowl, Intent::Less) => ui.filling = ui.filling.saturating_sub(1),
            (Page::Bowl, Intent::More) => ui.filling = ui.filling.saturating_add(1).min(self.plums),
            (Page::Bowl, Intent::Interact) => self.serve(ui),
            (Page::Served, Intent::Interact | Intent::NewMorning) => {
                let carried = self.plums;
                let history = std::mem::take(&mut self.history);
                *self = Self::new(history);
                self.plums = carried;
                self.notice = "又一个晴天。门廊下已经摆好了干净的碗。";
                ui.page = Page::Roam;
                ui.cursor = START;
                ui.filling = 0;
            }
            _ => {}
        }
    }
    pub fn adjacent_object(&self, p: IVec2) -> bool {
        distance(self.player, p) <= 1 && self.there(p) != Here::None
    }
}

// Only a finished breakfast is permanent. Reopening starts the following morning.
#[derive(Resource)]
pub struct JournalFile(Option<PathBuf>);
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
        let history = self
            .0
            .as_ref()
            .and_then(|p| match fs::read(p) {
                Ok(bytes) => match serde_json::from_slice::<Vec<Meal>>(&bytes) {
                    Ok(v) => Some(v),
                    Err(e) => {
                        eprintln!("手记读不出：{e}");
                        None
                    }
                },
                Err(e) if e.kind() == io::ErrorKind::NotFound => None,
                Err(e) => {
                    eprintln!("手记无法打开：{e}");
                    None
                }
            })
            .unwrap_or_default();
        Outing::new(history)
    }
    pub fn save(&self, meals: &[Meal]) -> io::Result<()> {
        let Some(p) = &self.0 else {
            return Ok(());
        };
        fs::create_dir_all(p.parent().unwrap())?;
        let tmp = p.with_extension("new");
        fs::write(&tmp, serde_json::to_vec(meals)?)?;
        fs::rename(tmp, p)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
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
        assert!(trail.iter().any(|p| p.y == 1));
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
}
