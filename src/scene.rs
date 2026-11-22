//! Viewport navigation over the authored floors and stationary decorative figures.
use bevy::prelude::*;

use crate::map::{self, Area};

pub const JELLY_POSITION: IVec2 = IVec2::new(4, -2);
pub const CISTERN_FIGURES: [IVec2; 5] = [
    IVec2::new(-5, 3),
    IVec2::new(-4, -3),
    IVec2::new(2, -1),
    IVec2::new(5, -3),
    IVec2::new(8, 2),
];

#[derive(Resource)]
pub struct SceneState {
    pub area: Area,
    pub player: IVec2,
    pub facing: IVec2,
    pub movement: u64,
}

impl Default for SceneState {
    fn default() -> Self {
        Self {
            area: Area::Town,
            player: IVec2::new(-2, 3),
            facing: IVec2::NEG_X,
            movement: 0,
        }
    }
}

pub fn ground(cell: IVec2) -> Vec3 {
    Vec3::new(cell.x as f32, 0.0, cell.y as f32)
}

impl SceneState {
    pub fn passable(&self, cell: IVec2) -> bool {
        map::passable(self.area, cell)
            && match self.area {
                Area::Town => cell != JELLY_POSITION,
                Area::Cistern => !CISTERN_FIGURES.contains(&cell),
            }
    }

    pub fn step(&mut self, direction: IVec2) -> bool {
        if direction == IVec2::ZERO || direction != direction.signum() {
            return false;
        }
        let next = self.player + direction;
        if !map::passage(self.player, next, |cell| self.passable(cell)) {
            return false;
        }
        self.player = next;
        self.facing = direction;
        self.movement = self.movement.wrapping_add(1);
        true
    }

    pub fn toggle_area(&mut self) {
        (self.area, self.player) = match self.area {
            Area::Town => (Area::Cistern, map::STAIRS),
            Area::Cistern => (Area::Town, IVec2::new(-2, 3)),
        };
        self.facing = IVec2::NEG_X;
        self.movement = self.movement.wrapping_add(1);
    }

    /// Shortest eight-way route, excluding the origin. Both diagonal shoulders
    /// must be passable, and a prop never substitutes an adjacent destination.
    pub fn path_to(&self, destination: IVec2, path: &mut Vec<IVec2>) -> bool {
        path.clear();
        if !map::inside(self.player) || !self.passable(destination) {
            return false;
        }
        const CELLS: usize = map::WIDTH * map::HEIGHT;
        let origin = map::cell_index(self.player);
        let target = map::cell_index(destination);
        let mut previous = [usize::MAX; CELLS];
        let mut queue = [IVec2::ZERO; CELLS];
        previous[origin] = origin;
        queue[0] = self.player;
        let (mut head, mut end) = (0, 1);
        while head < end && previous[target] == usize::MAX {
            let from = queue[head];
            head += 1;
            for direction in map::DIRS {
                let next = from + direction;
                if !map::passage(from, next, |cell| self.passable(cell)) {
                    continue;
                }
                let index = map::cell_index(next);
                if previous[index] != usize::MAX {
                    continue;
                }
                previous[index] = map::cell_index(from);
                queue[end] = next;
                end += 1;
            }
        }
        if previous[target] == usize::MAX {
            return false;
        }
        let mut index = target;
        while index != origin {
            path.push(IVec2::new(
                (index % map::WIDTH) as i32 + map::MIN.x,
                (index / map::WIDTH) as i32 + map::MIN.y,
            ));
            index = previous[index];
        }
        path.reverse();
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn diagonal_moves_once_and_rejects_large_vectors() {
        let mut scene = SceneState {
            player: IVec2::new(-2, 0),
            ..default()
        };
        let direction = IVec2::new(1, -1);
        assert!(scene.step(direction));
        assert_eq!(scene.player, IVec2::new(-1, -1));
        assert_eq!((scene.facing, scene.movement), (direction, 1));
        assert!(!scene.step(direction * 2));
        assert_eq!(scene.movement, 1);
    }

    #[test]
    fn routes_and_steps_share_strict_wall_and_actor_shoulders() {
        let mut scene = SceneState {
            player: IVec2::new(1, 0),
            ..default()
        };
        let target = IVec2::new(2, 1);
        let mut path = Vec::new();
        assert!(!scene.step(target - scene.player));
        assert!(scene.path_to(target, &mut path));
        assert_eq!(path.len(), 2);
        for next in path.iter().copied() {
            assert!(scene.step(next - scene.player));
        }
        assert_eq!(scene.player, target);

        scene.player = JELLY_POSITION + IVec2::NEG_X;
        let target = JELLY_POSITION + IVec2::NEG_Y;
        assert!(!scene.step(target - scene.player));
        assert!(scene.path_to(target, &mut path));
        assert_eq!(path.len(), 2);
        assert!(!scene.path_to(JELLY_POSITION, &mut path));
        assert!(path.is_empty());
    }

    #[test]
    fn routes_reach_floor_cells_beyond_byte_sized_indices() {
        let scene = SceneState::default();
        let destination = IVec2::new(0, 5);
        assert!(map::cell_index(destination) > u8::MAX as usize);
        let mut path = Vec::new();
        assert!(scene.path_to(destination, &mut path));
        assert_eq!(path.len(), 2);
        let mut from = scene.player;
        for &next in &path {
            assert!(map::passage(from, next, |cell| scene.passable(cell)));
            from = next;
        }
        assert_eq!(from, destination);
    }
}
