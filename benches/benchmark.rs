use std::fs;

use aoc2023_day12::{day12_parallel, day12_serial, parse_lines, Row};
use criterion::{black_box, criterion_group, criterion_main, Criterion};

const EXPECTED_ANSWER: (u64, u64) = (8193, 45322533163795);

fn benchmark(c: &mut Criterion) {
    let input = fs::read("input.txt").unwrap();
    c.bench_function("parse", |b| {
        b.iter(|| {
            let mut records_buf = vec![];
            let mut groups_buf = vec![];
            for line in parse_lines(black_box(&input)) {
                black_box(Row::parse(line, &mut records_buf, &mut groups_buf));
            }
        })
    });

    c.bench_function("day12 parallel", |b| {
        b.iter(|| {
            let ans = day12_parallel(black_box(&input));
            debug_assert_eq!(ans, EXPECTED_ANSWER);
        })
    });

    c.bench_function("day12 serial", |b| {
        b.iter(|| {
            let ans = day12_serial(black_box(&input));
            debug_assert_eq!(ans, EXPECTED_ANSWER);
        })
    });
}

criterion_group!(benches, benchmark);
criterion_main!(benches);
