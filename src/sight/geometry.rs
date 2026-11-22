//! The visible traveler is a billboard. Its safety margin, projected back to the
//! optical centre, makes a finite pyramid, not a ray of infinite length.
use bevy::prelude::*;

#[derive(Clone, Copy, Debug)]
pub(super) struct Bounds {
    pub min: Vec3,
    pub max: Vec3,
}

impl Bounds {
    pub fn union(self, other: Self) -> Self {
        Self {
            min: self.min.min(other.min),
            max: self.max.max(other.max),
        }
    }

    pub fn ray(self, ray: Ray3d, limit: f32) -> Option<f32> {
        let mut enter = 0.0_f32;
        let mut exit = limit;
        for i in 0..3 {
            let d = ray.direction[i];
            let o = ray.origin[i];
            if d.abs() < 1e-7 {
                if o < self.min[i] || o > self.max[i] {
                    return None;
                }
            } else {
                let a = (self.min[i] - o) / d;
                let b = (self.max[i] - o) / d;
                enter = enter.max(a.min(b));
                exit = exit.min(a.max(b));
                if enter > exit {
                    return None;
                }
            }
        }
        Some(enter)
    }
}

#[derive(Clone, Copy, PartialEq)]
pub(super) struct View {
    pub eye: Vec3,
    pub foot: Vec3,
    pub right: Vec3,
    pub up: Vec3,
    pub forward: Vec3,
}

// Includes the stepping feet, the widest held tool, and headroom. Distances
// are in the character's plane; zooming does not enlarge a screen-space hole.
const HALF_WIDTH: f32 = 0.63;
const SOLE: f32 = -0.055;
const CROWN: f32 = 1.55;
pub(super) const RELEASE_MARGIN: f32 = 0.08;

impl View {
    pub fn billboard_in_front(self, base: Vec3, half: f32, height: f32, margin: f32) -> bool {
        let target = self.foot - self.eye;
        let offset = base - self.eye;
        let far = target.dot(self.forward);
        let depth = offset.dot(self.forward);
        if depth <= 0. || depth >= far {
            return false;
        }
        let t = depth / far;
        let delta = offset - target * t;
        let y = delta.dot(self.up);
        delta.dot(self.right).abs() <= half + (HALF_WIDTH + margin) * t
            && y <= (CROWN + margin) * t
            && y + height >= (SOLE - margin) * t
    }
}

#[derive(Clone, Copy, Default)]
struct Span {
    axis: Vec3,
    low: f32,
    high: f32,
}
pub(super) struct Corridor {
    bounds: Bounds,
    // 5 pyramid faces; six distinct edge directions crossed with each box axis.
    axes: [Span; 23],
}

impl Corridor {
    pub fn new(v: View, margin: f32) -> Self {
        let w = v.right * (HALF_WIDTH + margin);
        let bottom = v.foot + v.up * (SOLE - margin);
        let top = v.foot + v.up * (CROWN + margin);
        let corners = [bottom - w, bottom + w, top + w, top - w];
        let mut bounds = Bounds {
            min: v.eye,
            max: v.eye,
        };
        for &p in &corners {
            bounds.min = bounds.min.min(p);
            bounds.max = bounds.max.max(p);
        }
        let span = |axis: Vec3| {
            let mut low = axis.dot(v.eye);
            let mut high = low;
            for &p in &corners {
                let d = axis.dot(p);
                low = low.min(d);
                high = high.max(d);
            }
            Span { axis, low, high }
        };
        let edges = corners.map(|p| p - v.eye);
        let mut axes = [Span::default(); 23];
        axes[0] = span(v.forward);
        for i in 0..4 {
            axes[1 + i] = span(edges[i].cross(edges[(i + 1) % 4]));
        }
        for (i, edge) in [edges[0], edges[1], edges[2], edges[3], v.right, v.up]
            .into_iter()
            .enumerate()
        {
            for (j, axis) in [Vec3::X, Vec3::Y, Vec3::Z].into_iter().enumerate() {
                axes[5 + 3 * i + j] = span(edge.cross(axis));
            }
        }
        Self { bounds, axes }
    }

    pub fn hits(&self, b: Bounds) -> bool {
        if b.min.cmpgt(self.bounds.max).any() || b.max.cmplt(self.bounds.min).any() {
            return false;
        }
        let mid = (b.max + b.min) * 0.5;
        let half = (b.max - b.min) * 0.5;
        self.axes.iter().all(|p| {
            let c = p.axis.dot(mid);
            let r = p.axis.abs().dot(half);
            c + r >= p.low && c - r <= p.high
        })
    }
}
