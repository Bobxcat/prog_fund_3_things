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

#[derive(Debug, Default)]
struct Tracer {
    categories: HashMap<&'static str, Duration>,
    callstack_trace: HashMap<Vec<&'static str>, Duration>,
    callstack: Vec<&'static str>,
}

const DO_TRACE: bool = true;

static TRACER_INSTANCE: LazyLock<Mutex<Tracer>> = LazyLock::new(Default::default);

/// Traces the operation while automatically keeping track of the callstack
#[inline(always)]
pub fn trace_op<T>(label: &'static str, op: impl FnOnce() -> T) -> T {
    trace_callstack_push(label);
    let start = Instant::now();
    let res = op();
    let t = start.elapsed();
    trace_op_time(label, t);
    trace_callstack_pop(t);
    res
}

#[inline(always)]
pub fn trace_callstack_push(label: &'static str) {
    if !DO_TRACE {
        return;
    }
    let mut tracer = TRACER_INSTANCE.lock().unwrap();
    tracer.callstack.push(label);
}

#[inline(always)]
pub fn trace_callstack_pop(time: Duration) {
    if !DO_TRACE {
        return;
    }
    let mut tracer = TRACER_INSTANCE.lock().unwrap();
    let stack = tracer.callstack.clone();
    tracer
        .callstack_trace
        .entry(stack)
        .or_default()
        .add_assign(time);
    tracer.callstack.pop();
}

/// Recommended to use `trace_op` when possible, to avoid manually calling
/// []
#[inline(always)]
pub fn trace_op_time(label: &'static str, time: Duration) {
    if !DO_TRACE {
        return;
    }
    let mut tracer = TRACER_INSTANCE.lock().unwrap();
    tracer.categories.entry(label).or_default().add_assign(time);
}

fn print_cols<const N: usize>(cols: [&[String]; N], pad_after_cols: [usize; N]) {
    let num_rows = cols.iter().map(|col| col.len()).max().unwrap_or(0);
    let cols_max_width = cols
        .iter()
        .map(|col| col.iter().map(|entry| entry.len()).max().unwrap_or(0))
        .collect::<Vec<_>>();

    for row in 0..num_rows {
        for col in 0..N {
            let entry = cols[col].get(row).unwrap_or(const { &String::new() });
            let pad = cols_max_width[col] + pad_after_cols[0];
            print!("{entry:<pad$}");
        }
        println!();
    }
}

pub fn print_trace_time() {
    println!("=============");
    println!("Trace Results");
    let tracer = TRACER_INSTANCE.lock().unwrap();
    let mut categories = tracer
        .categories
        .iter()
        .map(|(a, b)| (*a, *b))
        .collect::<Vec<_>>();
    categories.sort_by_key(|(_, t)| Reverse(*t));

    if categories.is_empty() {
        println!("{{empty}}");
        println!("=============");
        return;
    }

    let mut col0 = vec![];
    let mut col1 = vec![];

    for (lbl, t) in categories {
        col0.push(format!("{lbl}"));
        col1.push(format!("{t:?}"));
    }

    print_cols([&col0, &col1], [4; _]);

    println!("=============");
    println!("Flamegraph");

    //

    println!("=============");
}

pub fn reset_trace() {
    let mut tracer = TRACER_INSTANCE.lock().unwrap();
    tracer.categories.clear();
}
