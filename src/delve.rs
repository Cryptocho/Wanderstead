//! Turn-based cistern encounters. Only committed player actions advance these clocks.
use crate::map::{self, Area};
use bevy::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum Kit {
    #[default]
    None,
    Buckler,
    Hook,
}
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum Quest {
    #[default]
    Seeking,
    Carrying,
    Restored,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EnemyKind {
    Sentry,
    Moth,
}
#[derive(Clone, Debug)]
pub struct Enemy {
    pub id: u8,
    pub kind: EnemyKind,
    pub pos: IVec2,
    pub hp: u8,
    pub warn: Option<IVec2>,
    rest: u8,
}
#[derive(Clone, Debug)]
pub struct Expedition {
    pub hp: u8,
    pub max_hp: u8,
    pub tonics: u8,
    pub kit: Kit,
    pub quest: Quest,
    pub enemies: Vec<Enemy>,
    pub seal_open: bool,
    pub tonic_open: bool,
    pub attempts: u32,
    pub kills: u8,
    pub target_hit: Option<IVec2>,
    pub loot: u8,
    pub banked: u32,
    pub deaths: u32,
    pub retreats: u32,
    pub skill_ready: bool,
    pub beat: u32,
    /// A carried wheel is durable only after physically returning upstairs.
    pub wheel_home: bool,
}
impl Default for Expedition {
    fn default() -> Self {
        Self {
            hp: 8,
            max_hp: 8,
            tonics: 1,
            kit: Kit::None,
            quest: Quest::Seeking,
            enemies: Vec::with_capacity(8),
            seal_open: false,
            tonic_open: false,
            attempts: 0,
            kills: 0,
            target_hit: None,
            loot: 0,
            banked: 0,
            deaths: 0,
            retreats: 0,
            skill_ready: true,
            beat: 0,
            wheel_home: false,
        }
    }
}
pub fn safe(p: IVec2) -> bool {
    p.y >= 4 && (-2..=2).contains(&p.x)
}
use crate::map::{DIRS, distance};
impl Expedition {
    pub fn begin(&mut self, day: u32) {
        self.attempts = self.attempts.saturating_add(1);
        self.enemies.clear();
        self.loot = 0;
        self.kills = 0;
        self.beat = 0;
        self.seal_open = false;
        self.tonic_open = false;
        self.target_hit = None;
        self.skill_ready = true;
        let alternate = (self.attempts.wrapping_add(day) & 1) != 0;
        let positions = if alternate {
            [
                IVec2::new(-4, 3),
                IVec2::new(-6, -2),
                IVec2::new(2, -3),
                IVec2::new(6, -1),
            ]
        } else {
            [
                IVec2::new(-5, 3),
                IVec2::new(-4, -3),
                IVec2::new(2, -1),
                IVec2::new(5, -3),
            ]
        };
        for (id, pos) in positions.into_iter().enumerate() {
            let kind = if id % 2 == 0 {
                EnemyKind::Sentry
            } else {
                EnemyKind::Moth
            };
            self.enemies.push(Enemy {
                id: id as u8,
                kind,
                pos,
                hp: if kind == EnemyKind::Sentry { 3 } else { 2 },
                warn: None,
                rest: 0,
            });
        }
        if self.quest == Quest::Restored {
            self.enemies.push(Enemy {
                id: 4,
                kind: EnemyKind::Moth,
                pos: IVec2::new(8, 2),
                hp: 2,
                warn: None,
                rest: 0,
            });
        }
    }
    pub fn enemy_at(&self, p: IVec2) -> Option<usize> {
        self.enemies.iter().position(|e| e.hp > 0 && e.pos == p)
    }
    fn admits(&self, p: IVec2) -> bool {
        map::passable(Area::Cistern, p) && self.enemy_at(p).is_none()
    }
    pub(crate) fn attack_reaches(&self, i: usize, target: IVec2) -> bool {
        let pos = self.enemies[i].pos;
        let delta = target - pos;
        let range = distance(pos, target);
        let limit = if self.enemies[i].kind == EnemyKind::Sentry {
            1
        } else {
            2
        };
        range > 0
            && range <= limit
            && (delta.x == 0 || delta.y == 0 || delta.x.abs() == delta.y.abs())
            && map::clear_line(pos, target, |p| self.admits(p))
    }
    pub fn danger_at(&self, p: IVec2) -> bool {
        self.enemies
            .iter()
            .enumerate()
            .any(|(i, e)| e.hp > 0 && e.warn == Some(p) && self.attack_reaches(i, p))
            || (map::glyph(Area::Cistern, p) == b'P' && self.beat.wrapping_add(1) % 4 == 0)
    }
    pub fn strike(&mut self, i: usize, damage: u8, stun: bool) {
        let e = &mut self.enemies[i];
        self.target_hit = Some(e.pos);
        e.hp = e.hp.saturating_sub(damage);
        if stun {
            e.warn = None;
            e.rest = 1;
        }
        if e.hp == 0 {
            self.kills = self.kills.saturating_add(1);
            self.loot = self.loot.saturating_add(1);
        }
    }
    fn next_step(&self, i: usize, player: IVec2) -> Option<IVec2> {
        // 315 cells exceed a byte: both queue indices and predecessors are usize.
        const N: usize = map::WIDTH * map::HEIGHT;
        let origin = self.enemies[i].pos;
        let mut first = [IVec2::ZERO; N];
        let mut seen = [false; N];
        let mut queue = [IVec2::ZERO; N];
        let (mut head, mut end) = (0, 1);
        queue[0] = origin;
        seen[map::cell_index(origin)] = true;
        while head < end {
            let p = queue[head];
            head += 1;
            for d in DIRS {
                let n = p + d;
                if !map::passage(p, n, |p| {
                    map::passable(Area::Cistern, p)
                        && !safe(p)
                        && !self
                            .enemies
                            .iter()
                            .enumerate()
                            .any(|(j, e)| j != i && e.hp > 0 && e.pos == p)
                        && (p == n || p != player)
                }) {
                    continue;
                }
                let k = map::cell_index(n);
                if seen[k] {
                    continue;
                }
                seen[k] = true;
                first[k] = if p == origin {
                    d
                } else {
                    first[map::cell_index(p)]
                };
                if n == player {
                    return (p != origin).then_some(first[k]);
                }
                queue[end] = n;
                end += 1;
            }
        }
        None
    }
    /// Returns (damage received, successful guard). Warnings resolve before new ones form.
    pub fn advance(&mut self, player: IVec2, brace: bool) -> (u8, bool) {
        self.beat = self.beat.wrapping_add(1);
        let mut damage = 0u8;
        let mut guarded = false;
        for i in 0..self.enemies.len() {
            if self.enemies[i].hp == 0 {
                continue;
            }
            if let Some(target) = self.enemies[i].warn.take() {
                if target == player && !safe(player) && self.attack_reaches(i, player) {
                    if brace {
                        guarded = true;
                        if distance(self.enemies[i].pos, player) == 1
                            && map::reach(self.enemies[i].pos, player, |p| self.admits(p))
                        {
                            self.strike(i, if self.kit == Kit::Buckler { 2 } else { 1 }, false);
                        }
                    } else {
                        damage =
                            damage.saturating_add(if self.enemies[i].kind == EnemyKind::Sentry {
                                3
                            } else {
                                2
                            });
                    }
                }
                self.enemies[i].rest = 1;
                continue;
            }
            if self.enemies[i].rest > 0 {
                self.enemies[i].rest -= 1;
                continue;
            }
            if safe(player) {
                continue;
            }
            let pos = self.enemies[i].pos;
            let range = distance(pos, player);
            match self.enemies[i].kind {
                _ if self.attack_reaches(i, player) => self.enemies[i].warn = Some(player),
                EnemyKind::Moth if range <= 2 => {
                    // A moth orbits instead of following the guard's straight pursuit.
                    if let Some(d) = DIRS
                        .into_iter()
                        .cycle()
                        .skip((self.beat as usize + i) % DIRS.len())
                        .take(DIRS.len())
                        .find(|d| {
                            let n = pos + *d;
                            map::passage(pos, n, |p| self.admits(p) && !safe(p) && p != player)
                                && distance(n, player) == 2
                        })
                    {
                        self.enemies[i].pos += d;
                    } else if let Some(d) = self.next_step(i, player) {
                        self.enemies[i].pos += d;
                    }
                }
                _ if range <= 7
                    && (self.enemies[i].kind == EnemyKind::Sentry || self.beat % 2 == 0) =>
                {
                    if let Some(d) = self.next_step(i, player) {
                        self.enemies[i].pos += d;
                    }
                }
                _ => {}
            }
        }
        if map::glyph(Area::Cistern, player) == b'P' && self.beat % 4 == 0 {
            damage = damage.saturating_add(2);
        }
        self.enemies.retain(|e| e.hp > 0);
        self.hp = self.hp.saturating_sub(damage);
        (damage, guarded)
    }
    pub fn return_home(&mut self, fallen: bool) {
        if fallen {
            self.deaths = self.deaths.saturating_add(1);
            if !self.wheel_home && self.quest == Quest::Carrying {
                self.quest = Quest::Seeking;
            }
        } else {
            self.retreats = self.retreats.saturating_add(1);
            self.banked = self.banked.saturating_add(self.loot as u32);
            if self.quest == Quest::Carrying {
                self.wheel_home = true;
            }
        }
        self.loot = 0;
        self.enemies.clear();
        self.max_hp = (8 + (self.banked / 4).min(6)) as u8;
        self.hp = self.max_hp;
        self.skill_ready = true;
        self.target_hit = None;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn encounter(kind: EnemyKind, pos: IVec2) -> Expedition {
        let mut e = Expedition::default();
        e.enemies.push(Enemy {
            id: 0,
            kind,
            pos,
            hp: 3,
            warn: None,
            rest: 0,
        });
        e
    }

    #[test]
    fn diagonal_warning_guard_and_damage_share_the_same_corner_rule() {
        let player = IVec2::new(3, -2);
        let mut e = encounter(EnemyKind::Sentry, IVec2::new(2, -3));
        assert!(!e.attack_reaches(0, player));
        e.enemies[0].warn = Some(player);
        assert!(!e.danger_at(player));
        assert_eq!(e.advance(player, true), (0, false));
        assert_eq!(e.enemies[0].hp, 3);
        e.enemies[0].warn = Some(player);
        assert_eq!(e.advance(player, false), (0, false));

        let player = IVec2::new(1, -1);
        let mut e = encounter(EnemyKind::Sentry, IVec2::new(0, -2));
        assert_eq!(e.advance(player, false), (0, false));
        assert_eq!(e.enemies[0].warn, Some(player));
        assert!(e.danger_at(player));
        e.kit = Kit::Buckler;
        assert_eq!(e.advance(player, true), (0, true));
        assert_eq!(e.enemies[0].hp, 1);
    }

    #[test]
    fn moth_has_two_cell_lines_but_never_warns_through_walls_or_actors() {
        let mut e = encounter(EnemyKind::Moth, IVec2::new(0, -2));
        let player = IVec2::new(2, 0);
        assert!(e.attack_reaches(0, player));
        e.advance(player, false);
        assert_eq!(e.enemies[0].warn, Some(player));
        assert_eq!(e.advance(player, false), (2, false));
        e.enemies[0].kind = EnemyKind::Sentry;
        assert!(!e.attack_reaches(0, player));
        e.enemies[0].kind = EnemyKind::Moth;
        e.enemies.push(Enemy {
            id: 1,
            kind: EnemyKind::Sentry,
            pos: IVec2::new(1, -2),
            hp: 3,
            warn: None,
            rest: 0,
        });
        assert!(!e.attack_reaches(0, player));
        e.enemies.truncate(1);
        e.enemies[0].pos = IVec2::new(2, -3);
        assert!(!e.attack_reaches(0, IVec2::new(4, -1)));
    }

    #[test]
    fn enemies_move_diagonally_but_never_cut_corners_or_enter_the_player() {
        let mut e = encounter(EnemyKind::Sentry, IVec2::new(0, -3));
        let player = IVec2::new(2, -1);
        assert_eq!(e.next_step(0, player), Some(IVec2::new(1, 1)));
        e.advance(player, false);
        assert_eq!(e.enemies[0].pos, IVec2::new(1, -2));
        assert_eq!(e.next_step(0, player), None);
        e.enemies[0].pos = IVec2::new(2, -3);
        let player = IVec2::new(3, -2);
        let d = e.next_step(0, player).unwrap();
        assert_ne!(d, IVec2::new(1, 1));
        assert!(map::passage(e.enemies[0].pos, e.enemies[0].pos + d, |p| e.admits(p)));
    }
    #[test]
    fn locked_strike_can_be_dodged_and_guard_counter_really_kills() {
        let mut e = Expedition::default();
        let p = IVec2::new(0, -1);
        e.enemies.push(Enemy {
            id: 0,
            kind: EnemyKind::Sentry,
            pos: IVec2::new(-1, -1),
            hp: 2,
            warn: Some(p),
            rest: 0,
        });
        e.advance(p + IVec2::Y, false);
        assert_eq!(e.hp, 8);
        e.enemies[0].warn = Some(p);
        e.kit = Kit::Buckler;
        assert_eq!(e.advance(p, true), (0, true));
        assert!(e.enemies.is_empty());
        assert_eq!(e.loot, 1);
    }
    #[test]
    fn death_loses_unbanked_wheel_but_not_an_installed_spring() {
        let mut e = Expedition::default();
        e.quest = Quest::Carrying;
        e.loot = 3;
        e.banked = 5;
        e.return_home(true);
        assert_eq!((e.quest, e.loot, e.banked), (Quest::Seeking, 0, 5));
        e.quest = Quest::Restored;
        e.return_home(true);
        assert_eq!(e.quest, Quest::Restored);
    }
}
