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
        self - IRat::from(2u64) * normal * (normal.dot(self))
    }

    /// Performs `self ^ other`, otherwise known as the 2d wedge product or the perp dot product:
    ///
    /// https://mathworld.wolfram.com/PerpDotProduct.html
    #[trace_function("Vec2::cross")]
    pub fn cross(&self, other: &Self) -> IRat {
        &self.x * &other.y - &self.y * &other.x
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
