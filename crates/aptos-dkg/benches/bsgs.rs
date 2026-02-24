// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use aptos_dkg::dlog::{bsgs, table};
use ark_ec::{pairing::Pairing, CurveGroup, PrimeGroup};
use criterion::{criterion_group, criterion_main, BenchmarkId, Criterion};
use rand::{rngs::StdRng, Rng, SeedableRng};
use std::collections::HashMap;

/// Generic benchmark for the discrete log computation using Baby-step Giant-step
#[allow(non_snake_case)]
fn bench_dlog<E: Pairing>(c: &mut Criterion, curve_name: &str)
where
    E::G1: CurveGroup<ScalarField = E::ScalarField>,
{
    let mut group = c.benchmark_group(format!("dlog_bsgs_{}", curve_name));

    // Parameters
    let range_limit = 1u64 << 16;
    let table_sizes = [1 << 8, 1 << 12]; // 256 and 4096 entries
    let num_samples = 16usize; // For the vector benchmark

    // Deterministic RNG for reproducibility
    let mut rng = StdRng::seed_from_u64(42);

    let G = E::G1::generator();

    for &table_size in &table_sizes {
        // Precompute baby-step table
        let baby_table: HashMap<Vec<u8>, u64> = table::build::<E::G1>(G, table_size);

        // --- Single benchmark
        group.bench_with_input(
            BenchmarkId::new("single_dlog", format!("table_size_{}", table_size)),
            &table_size,
            |b, &_ts| {
                b.iter_with_setup(
                    // setup: generate fresh scalar and point for this iteration
                    || {
                        let x: u64 = rng.gen_range(0, range_limit);
                        let H = G * E::ScalarField::from(x);
                        (x, H)
                    },
                    // actual benchmark: compute discrete log
                    |(x, H)| {
                        let recovered = bsgs::dlog(G, H, &baby_table, range_limit)
                            .expect("Discrete log not found");
                        assert_eq!(recovered, x);
                    },
                );
            },
        );

        // --- Vector benchmark ---
        group.bench_with_input(
            BenchmarkId::new("vector_dlog", format!("table_size_{}", table_size)),
            &table_size,
            |b, &_ts| {
                b.iter_with_setup(
                    // setup: generate fresh batch of scalars and points
                    || {
                        let xs: Vec<u64> = (0..num_samples)
                            .map(|_| rng.gen_range(0, range_limit))
                            .collect();
                        let Hs: Vec<E::G1> =
                            xs.iter().map(|&x| G * E::ScalarField::from(x)).collect();
                        (xs, Hs)
                    },
                    // benchmark: compute discrete logs for the batch
                    |(xs, Hs)| {
                        let recovered = bsgs::dlog_vec(G, &Hs, &baby_table, range_limit)
                            .expect("Discrete log not found");
                        assert_eq!(recovered, xs);
                    },
                );
            },
        );
    }

    group.finish();
}

#[allow(non_snake_case)]
fn bench_table_build<E: Pairing>(c: &mut Criterion, curve_name: &str)
where
    E::G1: CurveGroup<ScalarField = E::ScalarField>,
{
    let mut group = c.benchmark_group(format!("dlog_table_build_{}", curve_name));

    // Limit Criterion to exactly 10 measurement iterations, because tables can be big (24 bits takes 1-2 min)
    group.sample_size(10); // It can't do less than 10

    // Time seems almost linear in the size of the table, so doesn't make sense to benchmark many values
    let table_sizes: &[u64] = &[1u64 << 16];

    let G = E::G1::generator();

    for &table_size in table_sizes {
        group.bench_with_input(
            BenchmarkId::from_parameter(table_size),
            &table_size,
            |b, &_ts| {
                b.iter(|| {
                    // Measure table build time only
                    let table: HashMap<Vec<u8>, u64> = table::build::<E::G1>(G, table_size);
                    let table_len: u64 = table.len().try_into().unwrap();
                    assert_eq!(table_len, table_size, "Unexpected table length");
                });
            },
        );
    }

    group.finish();
}

fn criterion_benchmark(c: &mut Criterion) {
    use ark_bls12_381::Bls12_381;
    use ark_bn254::Bn254;

    bench_dlog::<Bn254>(c, "bn254");
    bench_table_build::<Bn254>(c, "bn254");

    bench_dlog::<Bls12_381>(c, "bls12_381");
    bench_table_build::<Bls12_381>(c, "bls12_381");
}

criterion_group!(benches, criterion_benchmark);
criterion_main!(benches);
