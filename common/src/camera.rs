use std::ops::{
    Range,
    RangeBounds,
};

use glam::{
    Affine3A,
    Vec2,
    Vec3,
};
use serde::{
    Deserialize,
    Serialize,
};

use crate::angle::Radians;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
/// A Camera that orbits around a target.
/// 
/// Distance and position of the orbit can be controlled.
pub struct OrbitCamera {
    /// fov of the camera
    pub fov: Radians,
    /// radius of orbit
    radius: f32,
    /// radius bounds of the orbit
    bounds: Range<f32>,
    /// target to orbit around
    target: Vec3,
    /// angle on the xz axis
    phi: f32,
    /// angle on the y axis
    theta: f32,
}

impl OrbitCamera {
    /// Create a new [`OrbitCamera`].
    pub fn new(
        fov: impl Into<Radians>,
        radius: f32,
        bounds: impl RangeBounds<f32>,
        target: Vec3,
    ) -> Self {
        Self {
            fov: fov.into(),
            radius,
            bounds: range_from_range_bounds(bounds, 0.0, 1000.0),
            target,
            phi: std::f32::consts::FRAC_PI_2,
            theta: 0.0,
        }
    }

    /// The view matrix of the [`OrbitCamera`].
    pub fn view(&self) -> Affine3A {
        let eye = self.eye();

        Affine3A::look_at_lh(eye, self.target, Vec3::Y)
    }

    /// Update the orbit position with `delta`.
    pub fn orbit(&mut self, delta: Vec2) {
        self.theta += delta.x;
        self.phi += delta.y;
        self.phi = self.phi.clamp(0.1, std::f32::consts::PI - 0.1);
    }

    /// Zoom into or away from the target.
    pub fn zoom(&mut self, delta: f32) {
        let zoomed = self.radius + delta;
        if self.bounds.contains(&zoomed) {
            self.radius = zoomed;
        }
    }

    /// Get the position of the `eye` or `origin`.
    pub fn eye(&self) -> Vec3 {
        // get origin point in 3d space
        let (ts, tc) = f32::sin_cos(self.theta);
        let (ps, pc) = f32::sin_cos(self.phi);

        // spherical to cartesian
        let x = self.radius * ps * tc;
        let y = self.radius * pc;
        let z = self.radius * ps * ts;

        Vec3::new(x, y, z)
    }

    /// Change the target of the [`OrbitCamera`].
    pub fn set_target(&mut self, target: Vec3) {
        self.target = target;
    }

    /// Manually set phi, the "inclination" component.
    pub fn set_phi(&mut self, phi: f32) {
        self.phi = phi;
    }

    /// Manually set theta, the "horizontal" component.
    pub fn set_theta(&mut self, theta: f32) {
        self.theta = theta;
    }
}

fn range_from_range_bounds<T: RangeBounds<f32>>(range: T, min: f32, max: f32) -> Range<f32> {
    use std::ops::Bound;

    let start = match range.start_bound().cloned() {
        Bound::Included(start) => start,
        Bound::Excluded(start) => next_up(start),
        Bound::Unbounded => min,
    };

    let end = match range.end_bound().cloned() {
        Bound::Included(end) => next_up(end),
        Bound::Excluded(end) => end,
        Bound::Unbounded => max,
    };
    start..end
}

// taken from f32.rs
// it is nightly but only because of const fn, which I have removed
fn next_up(x: f32) -> f32 {
    // We must use strictly integer arithmetic to prevent denormals from
    // flushing to zero after an arithmetic operation on some platforms.
    const TINY_BITS: u32 = 0x1; // Smallest positive f32.
    const CLEAR_SIGN_MASK: u32 = 0x7fff_ffff;

    let bits = x.to_bits();
    if x.is_nan() || bits == f32::INFINITY.to_bits() {
        return x;
    }

    let abs = bits & CLEAR_SIGN_MASK;
    let next_bits = if abs == 0 {
        TINY_BITS
    } else if bits == abs {
        bits + 1
    } else {
        bits - 1
    };
    f32::from_bits(next_bits)
}
