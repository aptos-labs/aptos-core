// Copyright © Velor Foundation
// SPDX-License-Identifier: Apache-2.0

#[macro_use]
extern crate criterion;

use velor_types::keyless::{
    get_public_inputs_hash,
    test_utils::{get_sample_groth16_sig_and_pk, get_sample_jwk},
    Configuration,
};
use criterion::{measurement::Measurement, BenchmarkGroup, Criterion, Throughput};

/// Runs all the benchmarks.
fn bench_group(c: &mut Criterion) {
    let mut group = c.benchmark_group("keyless");

    public_inputs_hash(&mut group);

    group.finish();
}

fn public_inputs_hash<M: Measurement>(g: &mut BenchmarkGroup<M>) {
    g.throughput(Throughput::Elements(1_u64));

    let ((sig, pk), jwk, config) = (
        get_sample_groth16_sig_and_pk(),
        get_sample_jwk(),
        Configuration::new_for_devnet(),
    );

    g.bench_function("public_inputs_hash", move |b| {
        b.iter(|| get_public_inputs_hash(&sig, &pk, &jwk, &config))
    });
}

criterion_group!(
    name = keyless_benches;
    config = Criterion::default(); //.measurement_time(Duration::from_secs(100));
    targets = bench_group);
criterion_main!(keyless_benches);
