use std::ops::{Add, Div, Mul, Sub};

use crate::{
    derive_binop_by_value, derive_binop_by_value_assymetric,
    math_things::rational::{IRat, URat},
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Vec2 {
    pub x: IRat,
    pub y: IRat,
}

impl Vec2 {
    pub fn new(x: impl Into<IRat>, y: impl Into<IRat>) -> Self {
        Self {
            x: x.into(),
            y: y.into(),
        }
    }

    pub fn to_f32s(&self) -> (f32, f32) {
        (self.x.to_f64() as f32, self.y.to_f64() as f32)
    }

    /// Returns `(self.x, self.y)` as their float representations
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

    pub fn negated(mut self) -> Self {
        Self {
            x: -self.x,
            y: -self.y,
        }
    }

    /// Performs `self ^ other`, otherwise known as the 2d wedge product or the perp dot product:
    ///
    /// https://mathworld.wolfram.com/PerpDotProduct.html
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
