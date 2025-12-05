// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

// Run this bencher via `cargo bench --features fuzzing`.
use aptos_block_executor::combinatorial_tests::bencher::Bencher;
use criterion::{criterion_group, criterion_main, Criterion};
use proptest::prelude::*;

//
// Transaction benchmarks
//

fn random_benches(c: &mut Criterion) {
    c.bench_function("random_benches", |b| {
        let bencher = Bencher::<[u8; 32], [u8; 32]>::new(10000, 100);
        bencher.bench(&any::<[u8; 32]>(), b)
    });
}

criterion_group!(benches, random_benches);

criterion_main!(benches);
