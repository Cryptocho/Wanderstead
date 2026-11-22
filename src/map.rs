//! The authored map is consumed verbatim by the simulation, picking, and Blender bakers.
use bevy::prelude::*;
use serde::{Deserialize, Serialize};

pub const WIDTH: usize = 21;
pub const HEIGHT: usize = 15;
pub const MIN: IVec2 = IVec2::new(-10, -7);
pub const MAX: IVec2 = IVec2::new(10, 7);
pub const GATE: IVec2 = IVec2::new(0, -5);
pub const STAIRS: IVec2 = IVec2::new(0, 5);
pub const FOUNTAIN: IVec2 = IVec2::new(4, 2);
pub const APOTHECARY: IVec2 = IVec2::new(6, 2);
pub const TOOL_CHEST: IVec2 = IVec2::new(-6, 0);
pub const TONIC_CHEST: IVec2 = IVec2::new(-5, -4);
pub const RELIC: IVec2 = IVec2::new(6, -4);
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

/// Integer supercover: every crossed cell and both shoulders at exact corners.
pub fn clear_line(from: IVec2, to: IVec2, admits: impl Fn(IVec2) -> bool) -> bool {
    let delta = to - from;
    let sign = delta.signum();
    let (nx, ny) = (delta.x.abs(), delta.y.abs());
    let (mut ix, mut iy) = (0, 0);
    let mut p = from;
    while ix < nx || iy < ny {
        let decision = (1 + 2 * ix) * ny - (1 + 2 * iy) * nx;
        let step = if decision == 0 {
            ix += 1;
            iy += 1;
            sign
        } else if decision < 0 {
            ix += 1;
            IVec2::new(sign.x, 0)
        } else {
            iy += 1;
            IVec2::new(0, sign.y)
        };
        if !passage(p, p + step, &admits) {
            return false;
        }
        p += step;
    }
    true
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
                assert!(!clear_line(IVec2::ZERO, d * 2, |p| p != blocked));
            }
            assert!(reach(IVec2::ZERO, d, |p| p != d));
            assert!(!passage(IVec2::ZERO, d, |p| p != d));
        }
    }

    #[test]
    fn supercover_is_symmetric_and_checks_intermediate_cells() {
        for target in [IVec2::new(3, 1), IVec2::new(-2, 3), IVec2::new(0, 3)] {
            for d in DIRS {
                let forward = clear_line(IVec2::ZERO, target, |p| p != d);
                let backward = clear_line(target, IVec2::ZERO, |p| p == IVec2::ZERO || p != d);
                assert_eq!(forward, backward);
            }
        }
        assert!(!clear_line(IVec2::ZERO, IVec2::X * 3, |p| p != IVec2::X));
    }
}
