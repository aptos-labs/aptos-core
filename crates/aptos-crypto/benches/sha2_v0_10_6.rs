// Copyright © Aptos Foundation

// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

#[macro_use]
extern crate criterion;

use aptos_crypto::test_utils::random_bytes;
use criterion::{BenchmarkId, Criterion};
use rand::thread_rng;
use sha2_0_10_6::Digest;
fn bench_group(c: &mut Criterion) {
    let mut group = c.benchmark_group("sha2_v0_10_6");
    for msg_len in [0, 1, 4, 16, 64, 256, 1024, 4096, 16384, 65536] {
        group.bench_function(BenchmarkId::new("sha256", msg_len), move |b| {
            b.iter_with_setup(
                || random_bytes(&mut thread_rng(), msg_len),
                |msg| {
                    let mut hasher = sha2_0_10_6::Sha256::default();
                    hasher.update(msg);
                    let _digest = hasher.finalize_reset().to_vec();
                },
            )
        });
    }
    group.finish();
}

criterion_group!(
    name = sha2_0_10_6_benches;
    config = Criterion::default(); //.measurement_time(Duration::from_secs(100));
    targets = bench_group);
criterion_main!(sha2_0_10_6_benches);
