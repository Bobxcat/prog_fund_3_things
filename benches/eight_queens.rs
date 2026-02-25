use std::hint::black_box;

use criterion::{Criterion, criterion_group, criterion_main};
use prog_fund_3_things::eight_queens::*;

pub fn criterion_benchmark(c: &mut Criterion) {
    c.bench_function("with_benchset", |b| {
        b.iter(|| black_box(with_benchset::eight_queens_problem()))
    });
    c.bench_function("with_benchset_unsafe_opts", |b| {
        b.iter(|| black_box(with_benchset_unsafe_opts::eight_queens_problem()))
    });
    c.bench_function("with_benchset_tinyvec", |b| {
        b.iter(|| black_box(with_benchset_tinyvec::eight_queens_problem()))
    });
    c.bench_function("with_tinyset", |b| {
        b.iter(|| black_box(with_tinyset::eight_queens_problem()))
    });
    c.bench_function("with_hashset", |b| {
        b.iter(|| black_box(with_hashset::eight_queens_problem()))
    });
}

criterion_group!(benches, criterion_benchmark);
criterion_main!(benches);
