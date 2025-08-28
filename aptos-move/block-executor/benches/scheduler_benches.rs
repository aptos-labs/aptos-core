// Copyright © Aptos Foundation
// Parts of the project are originally copyright © Meta Platforms, Inc.
// SPDX-License-Identifier: Apache-2.0

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
