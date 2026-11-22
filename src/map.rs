//! The authored floors are shared by scene navigation, picking and Blender.
use bevy::prelude::*;
use serde::{Deserialize, Serialize};

pub const WIDTH: usize = 21;
pub const HEIGHT: usize = 15;
pub const MIN: IVec2 = IVec2::new(-10, -7);
pub const MAX: IVec2 = IVec2::new(10, 7);
pub const STAIRS: IVec2 = IVec2::new(0, 5);
pub const FOUNTAIN: IVec2 = IVec2::new(4, 2);
const BYTES: usize = (WIDTH + 1) * HEIGHT;
const TOWN: &[u8; BYTES] = include_bytes!("../assets/maps/town.grid");
const CISTERN: &[u8; BYTES] = include_bytes!("../assets/maps/cistern.grid");
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Area {
    #[default]
    Town,
    Cistern,
}
pub fn inside(p: IVec2) -> bool {
    p.cmpge(MIN).all() && p.cmple(MAX).all()
}
pub fn cell_index(p: IVec2) -> usize {
    (p.y - MIN.y) as usize * WIDTH + (p.x - MIN.x) as usize
}
pub fn glyph(area: Area, p: IVec2) -> u8 {
    if !inside(p) {
        return b'#';
    }
    let bytes = match area {
        Area::Town => TOWN,
        Area::Cistern => CISTERN,
    };
    bytes[(p.y - MIN.y) as usize * (WIDTH + 1) + (p.x - MIN.x) as usize]
}
pub fn passable(area: Area, p: IVec2) -> bool {
    matches!(glyph(area, p), b'.' | b'g' | b'@' | b'<' | b'>' | b'P')
}

/// Equal-cost compass steps, with cardinal tie-breaking for deterministic routes.
pub const DIRS: [IVec2; 8] = [
    IVec2::X,
    IVec2::NEG_Y,
    IVec2::Y,
    IVec2::NEG_X,
    IVec2::new(1, -1),
    IVec2::new(1, 1),
    IVec2::new(-1, -1),
    IVec2::new(-1, 1),
];

pub fn distance(a: IVec2, b: IVec2) -> u32 {
    a.x.abs_diff(b.x).max(a.y.abs_diff(b.y))
}

/// Contact may end on a solid object or actor; diagonal shoulders must admit passage.
pub fn reach(from: IVec2, to: IVec2, admits: impl Fn(IVec2) -> bool) -> bool {
    let d = to - from;
    distance(from, to) <= 1
        && (d.x == 0
            || d.y == 0
            || (admits(from + IVec2::new(d.x, 0)) && admits(from + IVec2::new(0, d.y))))
}

pub fn passage(from: IVec2, to: IVec2, admits: impl Fn(IVec2) -> bool) -> bool {
    distance(from, to) == 1 && admits(to) && reach(from, to, admits)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn each_diagonal_requires_both_shoulders_for_contact_and_passage() {
        for d in DIRS.into_iter().filter(|d| d.x != 0 && d.y != 0) {
            let shoulders = [IVec2::new(d.x, 0), IVec2::new(0, d.y)];
            assert!(passage(IVec2::ZERO, d, |_| true));
            for blocked in shoulders {
                assert!(!reach(IVec2::ZERO, d, |p| p != blocked));
                assert!(!passage(IVec2::ZERO, d, |p| p != blocked));
            }
            assert!(reach(IVec2::ZERO, d, |p| p != d));
            assert!(!passage(IVec2::ZERO, d, |p| p != d));
        }
    }
}
