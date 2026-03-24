use prog_fund_3_things::*;

fn main() {
    // prefix_ops::start();
    // eight_queens::start();
    math_things::raytracer_2d::start();
    // math_things::raytracer_3d::start(true);

    perf_tracer::print_trace_time(&perf_tracer::PrintOpts::default());
}
