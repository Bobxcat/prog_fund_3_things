use std::{
    fmt::Display,
    ops::{Add, Div, Mul, Sub},
};

use perf_tracer::trace_op;
use perf_tracer_macros::trace_function;

use crate::{
    derive_binop_by_value, derive_binop_by_value_assymetric,
    math_things::rational::{IRat, Precision, URat},
};

#[derive(Clone, PartialEq, Eq)]
pub struct Vec2 {
    pub x: IRat,
    pub y: IRat,
}

impl std::fmt::Debug for Vec2 {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{self}")
    }
}

impl Display for Vec2 {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "({:?}, {:?})", self.x, self.y)
    }
}

impl Vec2 {
    pub fn new(x: impl Into<IRat>, y: impl Into<IRat>) -> Self {
        Self {
            x: x.into(),
            y: y.into(),
        }
    }

    pub fn with_x(mut self, x: impl Into<IRat>) -> Self {
        self.x = x.into();
        self
    }

    pub fn with_y(mut self, y: impl Into<IRat>) -> Self {
        self.y = y.into();
        self
    }

    /// `to_f64s`, but casted to i32s
    pub fn to_i32s(&self) -> (i32, i32) {
        (self.x.to_f64() as i32, self.y.to_f64() as i32)
    }

    pub fn to_f32s(&self) -> (f32, f32) {
        (self.x.to_f64() as f32, self.y.to_f64() as f32)
    }

    /// Returns `(self.x, self.y)` as their float representations
    #[trace_function("Vec2::to_f64s")]
    pub fn to_f64s(&self) -> (f64, f64) {
        (self.x.to_f64(), self.y.to_f64())
    }

    pub fn dot(&self, other: &Self) -> IRat {
        &self.x * &other.x + &self.y * &other.y
    }

    /// Returns the rejection of `self` from `from`
    pub fn rejection(&self, from: &Self) -> Self {
        self - self.projection(from)
    }

    /// Returns the rejection of `self` on `onto`
    pub fn projection(&self, onto: &Self) -> Self {
        (self.dot(onto) / onto.sqr_magnitude()) * self
    }

    pub fn sqr_magnitude(&self) -> IRat {
        self.dot(self)
    }

    /// IMPRECISE
    #[trace_function("Vec2::magnitude")]
    pub fn magnitude(&self, prec: Precision) -> IRat {
        self.sqr_magnitude().sqrt(prec)
    }

    pub fn sqr_dist(&self, other: &Self) -> IRat {
        (self - other).sqr_magnitude()
    }

    pub fn negated(self) -> Self {
        Self {
            x: -self.x,
            y: -self.y,
        }
    }

    /// IMPRECISE
    ///
    /// The returned value will not be normalized.
    /// If you want an exactly normalized vector without maintaining exact direction, use
    /// `nearby_normalized`
    #[must_use]
    #[trace_function("Vec2::normalized")]
    pub fn normalized(&self, prec: Precision) -> Self {
        self / self.magnitude(prec)
    }

    /// Returns this vector reflected across `normal`
    /// * `self` and `normal` should be unit vectors
    #[must_use]
    #[trace_function("Vec2::reflected")]
    pub fn reflected(&self, normal: &Self) -> Vec2 {
        // https://en.wikipedia.org/wiki/Specular_reflection#Vector_formulation
        self - IRat::from(2u64) * normal * normal.dot(self)
    }

    /// Performs `self ^ other`, otherwise known as the 2d wedge product or the perp dot product:
    ///
    /// https://mathworld.wolfram.com/PerpDotProduct.html
    #[trace_function("Vec2::cross")]
    pub fn cross(&self, other: &Self) -> IRat {
        &self.x * &other.y - &self.y * &other.x
    }

    /// Finds a point nearby `self` on the unit sphere
    #[trace_function("Vec2::nearby_normalized")]
    pub fn nearby_normalized(&self, prec: Precision) -> Vec2 {
        Self::nearby_normalized_inner(self, prec)
    }

    /// Requires `pt` to be in the first quadrant
    fn nearby_normalized_inner(pt: &Vec2, prec: Precision) -> Vec2 {
        // https://en.wikipedia.org/wiki/Pythagorean_triple#Rational_points_on_a_unit_circle
        // Using the parametric equation, an appropriately accurate value of t can be found
        // for a point P in quadrant 1 by a binary search:
        // Starting with a=0,b=1 perform a->avg(a,b) if b is closer than a, else perform b->avg(a,b)

        let mut a = IRat::zero();
        let mut a_pt = Vec2::new(1, 0);
        let mut a_dist = a_pt.sqr_dist(pt);

        let mut b = IRat::one();
        let mut b_pt = Vec2::new(0, 1);
        let mut b_dist = b_pt.sqr_dist(pt);

        // * Each step doubles the precision, so for `n` steps,
        // the precision is a quarter of a circle / 2^n
        // * Since the points are further apart at t=0, the worst case
        // of the innacuracy is `dist(P(1/2^n), P(0))` where P is the point at a given t value
        // * Since `P(0)=(1,0)`, `dist(P(1/2^n), P(0))` is approximately `P(1/2^n).y`
        // `P(t).y = 2t / (1+t^2)` and near t=0, `P(t).x ~ 2t` so `P(1/2^n).y ~ 1/2^(n-1)`
        // * To conclude, the worst case precision for `n` steps is approximately `1/2 ^ (n-1)`
        // *

        for _ in 0..16 {
            match b_dist > a_dist {
                true => todo!(),
                false => todo!(),
            }
        }

        match b_dist > a_dist {
            true => a_pt,
            false => b_pt,
        }
    }
}

impl Mul<&IRat> for &Vec2 {
    type Output = Vec2;

    fn mul(self, rhs: &IRat) -> Self::Output {
        Vec2 {
            x: &self.x * rhs,
            y: &self.y * rhs,
        }
    }
}
derive_binop_by_value_assymetric!(Vec2, IRat, Mul, mul, *);

impl Div<&IRat> for &Vec2 {
    type Output = Vec2;

    fn div(self, rhs: &IRat) -> Self::Output {
        Vec2 {
            x: &self.x / rhs,
            y: &self.y / rhs,
        }
    }
}
derive_binop_by_value_assymetric!(Vec2, IRat, Div, div, /);

impl Add<&Vec2> for &Vec2 {
    type Output = Vec2;

    fn add(self, rhs: &Vec2) -> Self::Output {
        Vec2 {
            x: &self.x + &rhs.x,
            y: &self.y + &rhs.y,
        }
    }
}
derive_binop_by_value!(Vec2, Add, add, +);

impl Sub<&Vec2> for &Vec2 {
    type Output = Vec2;

    fn sub(self, rhs: &Vec2) -> Self::Output {
        Vec2 {
            x: &self.x - &rhs.x,
            y: &self.y - &rhs.y,
        }
    }
}
derive_binop_by_value!(Vec2, Sub, sub, -);
