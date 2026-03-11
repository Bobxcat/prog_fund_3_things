use std::hint::black_box;

use criterion::{Criterion, criterion_group, criterion_main};
use prog_fund_3_things::use_with_all;

pub fn criterion_benchmark(c: &mut Criterion) {
    use_with_all! {
        c.bench_function(IMP_NAME, |b| {
            b.iter(|| black_box(imp::eight_queens_problem()))
        });
    };
}

criterion_group!(benches, criterion_benchmark);
criterion_main!(benches);
