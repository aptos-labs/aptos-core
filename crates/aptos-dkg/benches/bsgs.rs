// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use aptos_dkg::dlog::{bsgs, table};
use ark_ec::{pairing::Pairing, CurveGroup, PrimeGroup};
use criterion::{criterion_group, criterion_main, BenchmarkId, Criterion};
use rand::{rngs::StdRng, Rng, SeedableRng};
use std::collections::HashMap;

/// Generic benchmark for the discrete log computation using Baby-step Giant-step
#[allow(non_snake_case)]
fn bench_dlog_for_engine<E: Pairing>(c: &mut Criterion, engine_name: &str)
where
    E::G1: CurveGroup<ScalarField = E::ScalarField>,
{
    let mut group = c.benchmark_group(format!("dlog_bsgs_{}", engine_name));

    // Parameters
    let range_limit = 1 << 16;
    let table_sizes = [1 << 8, 1 << 12]; // 256 and 4096 entries
    let num_samples = 100usize;

    // Deterministic RNG for reproducibility
    let mut rng = StdRng::seed_from_u64(42);

    let G = E::G1::generator();

    for &table_size in &table_sizes {
        // Precompute baby-step table
        let baby_table: HashMap<Vec<u8>, u32> = table::build::<E::G1>(G, table_size);

        // Pre-generate random scalars and corresponding points H = G * x
        let xs: Vec<u32> = (0..num_samples)
            .map(|_| rng.gen_range(0, range_limit))
            .collect();

        let Hs: Vec<E::G1> = xs.iter().map(|&x| G * E::ScalarField::from(x)).collect();

        group.bench_with_input(
            BenchmarkId::from_parameter(table_size),
            &table_size,
            |b, &_ts| {
                b.iter(|| {
                    for (i, H) in Hs.iter().enumerate() {
                        let recovered = bsgs::dlog::<E::G1>(G, *H, &baby_table, range_limit)
                            .expect("Discrete log not found");
                        assert_eq!(recovered, xs[i]);
                    } // could also use dlog_vec here instead
                });
            },
        );
    }

    group.finish();
}

fn criterion_benchmark(c: &mut Criterion) {
    use ark_bls12_381::Bls12_381;
    use ark_bn254::Bn254;

    bench_dlog_for_engine::<Bn254>(c, "bn254");
    bench_dlog_for_engine::<Bls12_381>(c, "bls12_381");
}

criterion_group!(benches, criterion_benchmark);
criterion_main!(benches);
