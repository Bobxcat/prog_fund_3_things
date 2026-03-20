use std::{
    cmp::Reverse,
    collections::HashMap,
    ops::{AddAssign, Neg},
    sync::{LazyLock, Mutex, RwLock},
    time::{Duration, Instant},
};

pub mod bigint;
pub mod mat2;
pub mod rational;
pub mod raytracer_2d;
pub mod vec2;

/// Assumes a `impl Op<&ty> for &ty { ... }`
#[macro_export]
macro_rules! derive_binop_by_value {
    ($ty:ident, $tr:ident, $func:ident, $op:tt) => {
        impl $tr<$ty> for &$ty {
            type Output = $ty;

            #[inline]
            fn $func(self, rhs: $ty) -> Self::Output {
                self $op &rhs
            }
        }

        impl $tr<&$ty> for $ty {
            type Output = $ty;

            #[inline]
            fn $func(self, rhs: &$ty) -> Self::Output {
                &self $op rhs
            }
        }

        impl $tr<$ty> for $ty {
            type Output = $ty;

            #[inline]
            fn $func(self, rhs: $ty) -> Self::Output {
                &self $op &rhs
            }
        }
    };
}

/// Assumes a `impl Op<&other_ty> for &impl_ty { ... }`
#[macro_export]
macro_rules! derive_binop_by_value_assymetric {
    ($impl_ty:ident, $other_ty:ident, $tr:ident, $func:ident, $op:tt) => {
        // ORIGINAL DIRECTION
        impl $tr<$other_ty> for &$impl_ty {
            type Output = $impl_ty;
            #[inline]
            fn $func(self, rhs: $other_ty) -> Self::Output {
                self $op &rhs
            }
        }
        impl $tr<&$other_ty> for $impl_ty {
            type Output = $impl_ty;
            #[inline]
            fn $func(self, rhs: &$other_ty) -> Self::Output {
                &self $op rhs
            }
        }
        impl $tr<$other_ty> for $impl_ty {
            type Output = $impl_ty;
            #[inline]
            fn $func(self, rhs: $other_ty) -> Self::Output {
                &self $op &rhs
            }
        }

        // FLIPPED
        impl $tr<&$impl_ty> for &$other_ty {
            type Output = $impl_ty;
            #[inline]
            fn $func(self, rhs: &$impl_ty) -> Self::Output {
                rhs $op self
            }
        }
        impl $tr<$impl_ty> for &$other_ty {
            type Output = $impl_ty;
            #[inline]
            fn $func(self, rhs: $impl_ty) -> Self::Output {
                &rhs $op self
            }
        }
        impl $tr<&$impl_ty> for $other_ty {
            type Output = $impl_ty;
            #[inline]
            fn $func(self, rhs: &$impl_ty) -> Self::Output {
                rhs $op &self
            }
        }
        impl $tr<$impl_ty> for $other_ty {
            type Output = $impl_ty;
            #[inline]
            fn $func(self, rhs: $impl_ty) -> Self::Output {
                &rhs $op &self
            }
        }
    };
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Sign {
    Pos,
    Neg,
}

impl Sign {
    pub fn from_is_pos(is_pos: bool) -> Self {
        match is_pos {
            true => Self::Pos,
            false => Self::Neg,
        }
    }

    #[must_use]
    pub fn multiply(self, rhs: Self) -> Self {
        use Sign::*;
        match (self, rhs) {
            (Neg, Neg) | (Pos, Pos) => Pos,
            (Pos, Neg) | (Neg, Pos) => Neg,
        }
    }
}

impl Neg for Sign {
    type Output = Sign;

    fn neg(self) -> Self::Output {
        match self {
            Sign::Pos => Sign::Neg,
            Sign::Neg => Sign::Pos,
        }
    }
}
