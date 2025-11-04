// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use aptos_crypto::arkworks::random::{sample_field_element, scalar_from_uniform_be_bytes};
use ark_ff::PrimeField;
use criterion::{criterion_group, criterion_main, Criterion};
use rand::thread_rng;

/// Benchmarks rejection sampling for an arbitrary field F
fn bench_sample_field_element_generic<F: PrimeField>(c: &mut Criterion, curve_name: &str) {
    let mut rng = thread_rng();
    // results are similar for e.g. rand::rngs::StdRng::seed_from_u64(42);

    c.bench_function(&format!("{}: sample_field_element", curve_name), |b| {
        b.iter(|| {
            let _scalar: F = sample_field_element(&mut rng);
        });
    });
}

/// Benchmarks double-sized byte reduction for an arbitrary field F
fn bench_scalar_from_uniform_be_bytes_generic<F: PrimeField>(c: &mut Criterion, curve_name: &str) {
    let mut rng = thread_rng();

    c.bench_function(
        &format!("{}: scalar_from_uniform_be_bytes", curve_name),
        |b| {
            b.iter(|| {
                let _scalar: F = scalar_from_uniform_be_bytes(&mut rng);
            });
        },
    );
}

/// Runs benchmarks for multiple fields
fn criterion_benchmark(c: &mut Criterion) {
    bench_sample_field_element_generic::<ark_bn254::Fr>(c, "BN254");
    bench_scalar_from_uniform_be_bytes_generic::<ark_bn254::Fr>(c, "BN254");

    bench_sample_field_element_generic::<ark_bls12_381::Fr>(c, "BLS12-381");
    bench_scalar_from_uniform_be_bytes_generic::<ark_bls12_381::Fr>(c, "BLS12-381");
}

criterion_group!(benches, criterion_benchmark);
criterion_main!(benches);
