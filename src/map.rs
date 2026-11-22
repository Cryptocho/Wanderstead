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
