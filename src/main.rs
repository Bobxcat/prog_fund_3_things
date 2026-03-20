use prog_fund_3_things::*;

/// fooo
///
/// docs
#[perf_tracer_macros::trace_function]
fn foo(a: i32, b: i32) -> i32 {
    let x = a + 1;
    b + x
}

fn main() {
    // prefix_ops::start();
    // eight_queens::start();
    math_things::raytracer_2d::start();
}
