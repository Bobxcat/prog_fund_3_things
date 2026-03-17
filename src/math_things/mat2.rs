use std::ops::{Div, Mul};

use crate::{
    derive_binop_by_value_assymetric,
    math_things::{rational::IRat, vec2::Vec2},
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Mat2 {
    /// Row major representation:
    ///
    /// a b
    ///
    /// c d
    vals: [IRat; 4],
}

impl Mat2 {
    /// From row-major values: `[a, b, c, d]`
    pub fn new(vals: [IRat; 4]) -> Self {
        Self { vals }
    }

    pub fn a(&self) -> &IRat {
        &self.vals[0]
    }
    pub fn b(&self) -> &IRat {
        &self.vals[1]
    }
    pub fn c(&self) -> &IRat {
        &self.vals[2]
    }
    pub fn d(&self) -> &IRat {
        &self.vals[3]
    }

    pub fn determinant(&self) -> IRat {
        &(self.a() * self.d()) - &(self.b() * self.c())
    }

    #[must_use]
    pub fn inverse(&self) -> Option<Self> {
        let det = self.determinant();
        if det.is_zero() {
            return None;
        }

        Some(
            &Self::new([
                self.d().clone(),
                -self.b().clone(),
                -self.c().clone(),
                self.a().clone(),
            ]) * &det.recip(),
        )
    }
}

impl Mul<&IRat> for &Mat2 {
    type Output = Mat2;

    fn mul(self, rhs: &IRat) -> Self::Output {
        Mat2::new([
            self.a() * rhs,
            self.b() * rhs,
            self.c() * rhs,
            self.d() * rhs,
        ])
    }
}
derive_binop_by_value_assymetric!(Mat2, IRat, Mul, mul, *);

impl Div<&IRat> for &Mat2 {
    type Output = Mat2;

    fn div(self, rhs: &IRat) -> Self::Output {
        Mat2::new([
            self.a() / rhs,
            self.b() / rhs,
            self.c() / rhs,
            self.d() / rhs,
        ])
    }
}
derive_binop_by_value_assymetric!(Mat2, IRat, Div, div, /);

impl Mul<&Vec2> for &Mat2 {
    type Output = Vec2;

    fn mul(self, rhs: &Vec2) -> Self::Output {
        Vec2 {
            x: self.a() * &rhs.x + self.b() * &rhs.y,
            y: self.c() * &rhs.x + self.d() * &rhs.y,
        }
    }
}
impl Mul<Vec2> for &Mat2 {
    type Output = Vec2;

    fn mul(self, rhs: Vec2) -> Self::Output {
        self * &rhs
    }
}
impl Mul<&Vec2> for Mat2 {
    type Output = Vec2;

    fn mul(self, rhs: &Vec2) -> Self::Output {
        &self * rhs
    }
}
impl Mul<Vec2> for Mat2 {
    type Output = Vec2;

    fn mul(self, rhs: Vec2) -> Self::Output {
        &self * &rhs
    }
}
