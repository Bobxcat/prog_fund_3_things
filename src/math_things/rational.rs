use std::{
    cmp::{self, Ordering},
    ops::{Add, AddAssign, Div, DivAssign, Mul, MulAssign, Neg, Range, Sub},
};

use dashu_float::{FBig, round::Round};
use perf_tracer::trace_op;
use perf_tracer_macros::trace_function;

use crate::{
    derive_binop_by_value, derive_binop_by_value_assymetric,
    math_things::{Sign, bigint::UBig},
};

#[inline(always)]
fn extract_bits(mut x: u64, bits: Range<u32>, keep_position: bool) -> u64 {
    x <<= 64 - bits.end;
    x >>= 64 - bits.end;

    x >>= bits.start;
    if keep_position {
        x <<= bits.start;
    }

    x
}

/// Precision stores the exponent of a power of two
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Precision(pub usize);

impl Precision {
    pub fn digits(n: usize) -> Self {
        Self(64 * n)
    }

    pub fn to_urat(&self) -> URat {
        URat::from_u64_recip(2).powi(self.0 as i32)
    }
}

impl Add for Precision {
    type Output = Self;

    fn add(self, rhs: Self) -> Self::Output {
        Self(self.0 + rhs.0)
    }
}

/// A reduced signed rational number
#[derive(Clone)]
pub struct IRat {
    magnitude: URat,
    sign: Sign,
}

impl std::fmt::Debug for IRat {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self.sign {
            Sign::Pos => write!(f, "{:?}", self.magnitude),
            Sign::Neg => write!(f, "-{:?}", self.magnitude),
        }
    }
}

impl std::fmt::Display for IRat {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self.sign {
            Sign::Pos => write!(f, "{}", self.magnitude),
            Sign::Neg => write!(f, "-{}", self.magnitude),
        }
    }
}

impl IRat {
    pub fn new(magnitude: URat, sign: Sign) -> Self {
        Self { magnitude, sign }
    }

    #[must_use]
    pub fn is_zero(&self) -> bool {
        self.magnitude.is_zero()
    }

    pub fn zero() -> Self {
        Self {
            magnitude: URat::zero(),
            sign: Sign::Pos,
        }
    }

    pub fn one() -> Self {
        Self {
            magnitude: URat::one(),
            sign: Sign::Pos,
        }
    }

    pub fn abs(self) -> IRat {
        IRat::new(self.magnitude, Sign::Pos)
    }

    pub fn abs_unsigned(self) -> URat {
        self.magnitude
    }

    /// A non-normal `x` will result in `0` being returned
    pub fn from_f64(x: f64) -> Self {
        let s = Self {
            magnitude: URat::from_f64(x),
            sign: match x.is_sign_positive() {
                true => Sign::Pos,
                false => Sign::Neg,
            },
        };

        s
    }

    pub fn from_fbig<RoundingMode: Round, const BASE: u64>(x: FBig<RoundingMode, BASE>) -> IRat {
        let (significand, exp) = x.into_repr().into_parts();
        let (sign, sig_words) = significand.as_sign_words();

        let mut x = IRat {
            magnitude: URat {
                num: UBig::from_digits(sig_words.to_vec()),
                den: UBig::one(),
            },
            sign: match sign {
                dashu_base::Sign::Positive => Sign::Pos,
                dashu_base::Sign::Negative => Sign::Neg,
            },
        };

        if exp >= 0 {
            x.magnitude.num *= &UBig::new(BASE).pow(exp.unsigned_abs() as u64);
        } else {
            x.magnitude.den *= &UBig::new(BASE).pow(exp.unsigned_abs() as u64);
        }

        x
    }

    pub fn powi(&self, pow: u32) -> Self {
        let mut s = IRat::one();
        for _ in 0..pow {
            s = s * self;
        }
        s
    }

    /// IMPRECISE
    ///
    /// # Panics
    /// * If `self < 0`
    pub fn sqrt(&self, prec: Precision) -> Self {
        Self {
            magnitude: self.magnitude.sqrt(prec),
            sign: self.sign,
        }
    }

    /// Converts `self` into an f64, with possible loss
    pub fn to_f64(&self) -> f64 {
        // FIXME: Will fail with large fractions that should be representable
        self.to_fbig().to_f64().value()
    }

    /// Converts `self` into a big float, with possible loss
    pub fn to_fbig(&self) -> FBig {
        self.magnitude.to_fbig()
            * match self.sign {
                Sign::Pos => FBig::ONE,
                Sign::Neg => FBig::NEG_ONE,
            }
    }

    /// Panics if `self` is zero
    #[must_use]
    #[track_caller]
    pub fn recip(&self) -> Self {
        Self {
            magnitude: self.magnitude.recip(),
            sign: self.sign,
        }
    }
}

impl From<URat> for IRat {
    fn from(value: URat) -> Self {
        Self {
            magnitude: value,
            sign: Sign::Pos,
        }
    }
}

impl From<f64> for IRat {
    fn from(value: f64) -> Self {
        Self::from_f64(value)
    }
}

impl From<i32> for IRat {
    fn from(value: i32) -> Self {
        Self::from(value as i64)
    }
}

impl From<i64> for IRat {
    fn from(value: i64) -> Self {
        Self {
            magnitude: URat::from(value.unsigned_abs()),
            sign: Sign::from_is_pos(value >= 0),
        }
    }
}

impl From<u32> for IRat {
    fn from(value: u32) -> Self {
        Self::from(value as u64)
    }
}

impl From<u64> for IRat {
    fn from(value: u64) -> Self {
        Self {
            magnitude: URat::from(value),
            sign: Sign::Pos,
        }
    }
}

impl Add<&IRat> for &IRat {
    type Output = IRat;

    fn add(self, rhs: &IRat) -> Self::Output {
        use Sign::*;
        match (self.sign, rhs.sign) {
            (Pos, Pos) | (Neg, Neg) => IRat {
                magnitude: &self.magnitude + &rhs.magnitude,
                sign: self.sign,
            },
            (Pos, Neg) | (Neg, Pos) => {
                if self.magnitude > rhs.magnitude {
                    IRat {
                        magnitude: &self.magnitude - &rhs.magnitude,
                        sign: self.sign,
                    }
                } else {
                    IRat {
                        magnitude: &rhs.magnitude - &self.magnitude,
                        sign: rhs.sign,
                    }
                }
            }
        }
    }
}
derive_binop_by_value!(IRat, Add, add, +);

impl Sub<&IRat> for &IRat {
    type Output = IRat;

    fn sub(self, rhs: &IRat) -> Self::Output {
        self + &(-rhs.clone())
    }
}
derive_binop_by_value!(IRat, Sub, sub, -);

impl Neg for IRat {
    type Output = IRat;

    fn neg(mut self) -> Self::Output {
        self.sign = -self.sign;
        self
    }
}

impl Mul<&IRat> for &IRat {
    type Output = IRat;

    fn mul(self, rhs: &IRat) -> Self::Output {
        IRat {
            magnitude: &self.magnitude * &rhs.magnitude,
            sign: self.sign.multiply(rhs.sign),
        }
    }
}
derive_binop_by_value!(IRat, Mul, mul, *);

impl Mul<&UBig> for &IRat {
    type Output = IRat;

    fn mul(self, rhs: &UBig) -> Self::Output {
        IRat {
            magnitude: &self.magnitude * rhs,
            sign: self.sign,
        }
    }
}
derive_binop_by_value_assymetric!(IRat, UBig, Mul, mul, *);

impl Div<&IRat> for &IRat {
    type Output = IRat;

    #[track_caller]
    fn div(self, rhs: &IRat) -> Self::Output {
        IRat {
            magnitude: &self.magnitude / &rhs.magnitude,
            sign: self.sign.multiply(rhs.sign),
        }
    }
}
derive_binop_by_value!(IRat, Div, div, /);

impl PartialEq for IRat {
    fn eq(&self, other: &Self) -> bool {
        self.cmp(other).is_eq()
    }
}
impl Eq for IRat {}

impl PartialOrd for IRat {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for IRat {
    fn cmp(&self, other: &Self) -> Ordering {
        // Negative and positive zero should be equal
        if self.is_zero() && other.is_zero() {
            return Ordering::Equal;
        }

        match (self.sign, other.sign) {
            (Sign::Pos, Sign::Pos) => self.magnitude.cmp(&other.magnitude),
            (Sign::Pos, Sign::Neg) => Ordering::Greater,
            (Sign::Neg, Sign::Pos) => Ordering::Less,
            (Sign::Neg, Sign::Neg) => self.magnitude.cmp(&other.magnitude).reverse(),
        }
    }
}

/// A reduced unsigned rational number
#[derive(Clone)]
pub struct URat {
    num: UBig,
    /// The denominator, always non-zero
    den: UBig,
}

impl std::fmt::Debug for URat {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}/{}", self.num, self.den)
    }
}

impl std::fmt::Display for URat {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}",
            self.to_fbig()
                .with_precision(f.precision().unwrap_or(32))
                .value()
                .to_decimal()
                .value()
        )
    }
}

impl URat {
    #[track_caller]
    pub fn new(num: impl Into<UBig>, den: impl Into<UBig>) -> Self {
        let (num, den) = (num.into(), den.into());
        if den.is_zero() {
            panic!("Called URat::new with a denominator of zero");
        }
        Self { num, den }.reduced()
    }

    pub fn from_u64(num: u64) -> Self {
        Self {
            num: num.into(),
            den: UBig::one(),
        }
    }

    pub fn from_u64_recip(den: u64) -> Self {
        if den == 0 {
            panic!("Called URat::from_u64_recip with a denominator of zero");
        }
        Self {
            num: UBig::one(),
            den: den.into(),
        }
    }

    pub fn is_zero(&self) -> bool {
        self.num.is_zero()
    }

    pub fn zero() -> Self {
        Self {
            num: UBig::zero(),
            den: UBig::one(),
        }
    }

    pub fn one() -> Self {
        Self {
            num: UBig::one(),
            den: UBig::one(),
        }
    }

    pub fn abs_difference(&self, other: &Self) -> Self {
        if self > other {
            self - other
        } else {
            other - self
        }
    }

    /// Panics if the denominator is zero
    #[must_use]
    #[track_caller]
    #[trace_function("URat::$f")]
    pub fn reduced(&self) -> Self {
        assert!(!self.den.is_zero());
        let divisor = UBig::gcd(self.num.clone(), self.den.clone());
        Self {
            num: &self.num / &divisor,
            den: &self.den / &divisor,
        }
    }

    pub fn round(&mut self, prec: Precision) {
        // We leave at least 2 digits in the numerator and denominator
        let min_digits = 2;
        let prec_digits = prec.0.div_ceil(64);
        let digits_to_keep = min_digits.max(prec_digits);

        let digits_to_remove = cmp::min(
            self.num.digits().len().saturating_sub(digits_to_keep),
            self.den.digits().len().saturating_sub(digits_to_keep),
        );

        for _ in 0..digits_to_remove {
            self.num.digits_mut().remove(0);
            self.den.digits_mut().remove(0);
        }
    }

    /// Panics if `self` is zero
    #[must_use]
    #[track_caller]
    pub fn recip(&self) -> Self {
        if self.is_zero() {
            panic!("Tried to take the reciprocal of zero");
        }
        Self {
            num: self.den.clone(),
            den: self.num.clone(),
        }
    }

    #[must_use]
    pub fn floor(&self) -> UBig {
        &self.num / &self.den
    }

    #[must_use]
    pub fn powi(&self, exp: i32) -> Self {
        let mut n = Self::one();

        match exp >= 0 {
            true => {
                for _ in 0..exp.abs() {
                    n *= &self;
                }
            }
            false => {
                for _ in 0..exp.abs() {
                    n /= &self;
                }
            }
        }

        n
    }

    /// IMPRECISE
    #[trace_function("URat::$f")]
    pub fn sqrt(&self, prec: Precision) -> Self {
        sqrt_algorithms::sqrt_herons_method(self, prec)
    }

    /// Discards the sign
    ///
    /// A non-normal `x` will result in `0` being returned
    #[trace_function("URat::$f")]
    pub fn from_f64(x: f64) -> URat {
        match x.classify() {
            // FIXME: Should this panic instead?
            std::num::FpCategory::Nan | std::num::FpCategory::Infinite => return URat::zero(),
            // FIXME: Handle Subnormals
            std::num::FpCategory::Subnormal => todo!(),
            std::num::FpCategory::Zero => return URat::zero(),
            std::num::FpCategory::Normal => (),
        }

        let bits = x.to_bits();
        let exp_bits = extract_bits(bits, 52..63, false);
        let frac_bits = extract_bits(bits, 0..52, false);

        // `x = 1.frac_bits * 2^exp`
        //`0.f[51]f[50]...f[0]`

        // FIXME: the fractional parsing can be replaced with a UBig::from_u64(..)
        // and a division by 2^52 (which is a constant)
        // ^ Or something like that

        // First, add all the bits of the fraction
        let mut num = Self::one();
        let mut place = Self::one();

        for i in (0..52).rev() {
            place /= &Self::from_u64(2);
            let frac_bit = frac_bits >> i & 1;
            if frac_bit == 1 {
                num = &num + &place;
            }
        }

        let exp = exp_bits as i32 - 1023;
        num *= &URat::from_u64(2).powi(exp);

        num.reduced()
    }

    /// Discards the sign
    #[trace_function("URat::$f")]
    pub fn from_fbig<RoundingMode: Round, const BASE: u64>(x: FBig<RoundingMode, BASE>) -> URat {
        IRat::from_fbig(x).abs_unsigned()
    }

    pub fn to_f64(&self) -> f64 {
        self.to_fbig().to_f64().value()
    }

    /// Converts `self` into a big float, with possible loss
    pub fn to_fbig(&self) -> FBig {
        let res = self.num.to_fbig() / self.den.to_fbig();
        println!(
            "\nTO_FBIG: {} / {} = {}\n",
            self.num
                .to_fbig()
                .with_precision(64)
                .value()
                .to_decimal()
                .value(),
            self.den
                .to_fbig()
                .with_precision(64)
                .value()
                .to_decimal()
                .value(),
            res.clone().with_precision(64).value().to_decimal().value(),
        );
        res
    }
}

impl From<u32> for URat {
    fn from(value: u32) -> Self {
        URat::from(value as u64)
    }
}

impl From<u64> for URat {
    fn from(value: u64) -> Self {
        URat::from_u64(value)
    }
}

impl Add<&URat> for &URat {
    type Output = URat;

    #[trace_function("URat::$f")]
    fn add(self, rhs: &URat) -> Self::Output {
        // a/b + c/d = (ad + bc) / bd
        let x = trace_op("addition_step", || URat {
            num: &self.num * &rhs.den + &rhs.num * &self.den,
            den: &self.den * &rhs.den,
        });
        trace_op("reduction_step", move || x.reduced())
    }
}
derive_binop_by_value!(URat, Add, add, +);

impl Sub<&URat> for &URat {
    type Output = URat;

    #[trace_function("URat::$f")]
    fn sub(self, rhs: &URat) -> Self::Output {
        // a/b - c/d = (ad - bc) / bd
        URat {
            num: &self.num * &rhs.den - &rhs.num * &self.den,
            den: &self.den * &rhs.den,
        }
        .reduced()
    }
}
derive_binop_by_value!(URat, Sub, sub, -);

impl Mul<&URat> for &URat {
    type Output = URat;

    #[trace_function("URat::$f")]
    fn mul(self, rhs: &URat) -> Self::Output {
        URat {
            num: &self.num * &rhs.num,
            den: &self.den * &rhs.den,
        }
        .reduced()
    }
}
derive_binop_by_value!(URat, Mul, mul, *);

impl MulAssign<&URat> for URat {
    fn mul_assign(&mut self, rhs: &URat) {
        *self = &*self * rhs;
    }
}

impl Mul<&UBig> for &URat {
    type Output = URat;

    fn mul(self, rhs: &UBig) -> Self::Output {
        URat {
            num: &self.num * rhs,
            den: self.den.clone(),
        }
        .reduced()
    }
}
derive_binop_by_value_assymetric!(URat, UBig, Mul, mul, *);

impl Div<&URat> for &URat {
    type Output = URat;

    #[track_caller]
    fn div(self, rhs: &URat) -> Self::Output {
        if rhs.is_zero() {
            panic!("Tried to divide by zero");
        }

        // (a/b) / (c/d) = ad / bc
        URat {
            num: &self.num * &rhs.den,
            den: &self.den * &rhs.num,
        }
        .reduced()
    }
}
derive_binop_by_value!(URat, Div, div, /);

impl Div<u64> for &URat {
    type Output = URat;

    #[track_caller]
    fn div(self, rhs: u64) -> Self::Output {
        self / URat::from_u64(rhs)
    }
}
impl Div<u64> for URat {
    type Output = URat;

    fn div(self, rhs: u64) -> Self::Output {
        &self / rhs
    }
}

impl DivAssign<&URat> for URat {
    fn div_assign(&mut self, rhs: &URat) {
        *self = &*self / rhs;
    }
}

impl PartialEq for URat {
    fn eq(&self, other: &Self) -> bool {
        self.cmp(other).is_eq()
    }
}

impl Eq for URat {}

impl PartialOrd for URat {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for URat {
    fn cmp(&self, other: &Self) -> Ordering {
        // FIXME: Is this a necessary/performant check?
        // We're guaranteed that self and other are reduced, so this is a sufficient check
        if self.num == other.num && self.den == other.den {
            return Ordering::Equal;
        }

        // a/b > c/d == ad > bc

        let lhs = &self.num * &other.den;
        let rhs = &self.den * &other.num;
        lhs.cmp(&rhs)
    }
}

mod sqrt_algorithms {
    use dashu_base::SquareRoot;
    use perf_tracer::trace_op;
    use perf_tracer_macros::trace_function;

    use crate::math_things::rational::{Precision, URat};

    #[trace_function]
    pub fn sqrt_herons_method(s: &URat, prec: Precision) -> URat {
        if s.is_zero() {
            return URat::zero();
        }

        let prec_with_guard = prec + Precision::digits(2);
        let prec_num = prec.to_urat();

        let mut guess = s.clone() / 2;

        let mut i = 0;
        while i < 512 {
            let mut next_guess = trace_op("next_guess_calc", || (&guess + s / &guess) / 2);

            next_guess.round(prec_with_guard);

            // The change between guesses is at most the
            // absolute error of the current guess
            if trace_op("difference_comparison", || {
                next_guess.abs_difference(&guess) < prec_num
            }) {
                break;
            }
            guess = next_guess;
            i += 1;
        }
        guess.round(prec);

        guess
    }

    pub fn sqrt_through_fbig(s: &URat, prec: Precision) -> URat {
        let f = s.to_fbig().with_precision(prec.0 + 20).value();
        URat::from_fbig(f.sqrt())
    }
}

#[cfg(test)]
mod tests {
    use crate::math_things::rational::{IRat, Precision};

    #[test]
    fn test_sqrt() {
        [1.01, 10., 16., 0.01].into_iter().for_each(|x| {
            let num = IRat::from_f64(x).sqrt(Precision(128));
            let should = x.sqrt();

            println!(
                "sqrt({x}); should={should}; got={num:?}={}",
                num.to_fbig()
                    .with_precision(128)
                    .value()
                    .to_decimal()
                    .value()
            );
        });
    }
}
