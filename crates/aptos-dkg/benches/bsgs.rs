// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use aptos_dkg::dlog::{bsgs, table};
use ark_ec::{pairing::Pairing, PrimeGroup};
use criterion::{criterion_group, criterion_main, BenchmarkId, Criterion};
use rand::{rngs::StdRng, Rng, SeedableRng};

/// Benchmark for small range_limit / table_size: dlog per element, batched, and batched_rolling
/// for various batch sizes. Two configs: len=8 (range_limit=32, table_size=20) and len=7 (range_limit=37, table_size=25).
#[allow(non_snake_case)]
fn bench_dlog_vec_small_range<E: Pairing>(c: &mut Criterion, curve_name: &str) {
    let batch_sizes: &[usize] = &[1, 8, 32, 128, 512, 1024, 2048];
    let configs: &[(usize, u64, u64)] = &[
        (8, 1 << 32, 1 << 25), // H_vec len 8, range_limit 32, table_size 20
        (7, 1 << 37, 1 << 25), // H_vec len 7, range_limit 37, table_size 25
    ];

    println!(
        "Starting benchmark for curve {} with small range",
        curve_name
    );
    let mut rng = StdRng::seed_from_u64(42);
    let G = E::G1::generator();

    for &(vec_len, range_limit, table_size) in configs {
        let group_name = format!(
            "dlog_bsgs_{}_small_range_len{}_range{}_table{}",
            curve_name,
            vec_len,
            range_limit.ilog2(),
            table_size.ilog2()
        );
        let mut group = c.benchmark_group(&group_name);

        println!(
            "Building baby table for curve {} with table size {}",
            curve_name, table_size
        );
        let baby_table = table::build::<E::G1>(G, table_size);
        println!(
            "Baby table built for curve {} with table size {}",
            curve_name, table_size
        );
        let xs: Vec<u64> = (0..vec_len)
            .map(|_| rng.gen_range(0, range_limit))
            .collect();
        let Hs: Vec<E::G1> = xs.iter().map(|&x| G * E::ScalarField::from(x)).collect();

        // Single-target dlog per element (no batching across targets)
        group.bench_with_input(BenchmarkId::new("dlog_single_elt", ""), &(), |b, _| {
            b.iter(|| {
                let recovered: Vec<u64> = Hs
                    .iter()
                    .map(|H| {
                        bsgs::dlog(G, *H, &baby_table, range_limit).expect("Discrete log not found")
                    })
                    .collect();
                assert_eq!(recovered, xs);
            });
        });

        for &batch_size in batch_sizes {
            group.bench_with_input(
                BenchmarkId::new("dlog_with_batch_size_per_elt", batch_size),
                &batch_size,
                |b, &batch_size| {
                    b.iter(|| {
                        let recovered: Vec<u64> = Hs
                            .iter()
                            .map(|H| {
                                bsgs::dlog_with_batch_size(
                                    G,
                                    *H,
                                    &baby_table,
                                    range_limit,
                                    batch_size,
                                )
                                .expect("Discrete log not found")
                            })
                            .collect();
                        assert_eq!(recovered, xs);
                    });
                },
            );

            group.bench_with_input(
                BenchmarkId::new("dlog_vec_batched_with_batch_size", batch_size),
                &batch_size,
                |b, &batch_size| {
                    b.iter(|| {
                        let recovered = bsgs::dlog_vec_batched_with_batch_size(
                            G,
                            &Hs,
                            &baby_table,
                            range_limit,
                            batch_size,
                        )
                        .expect("Discrete log not found");
                        assert_eq!(recovered, xs);
                    });
                },
            );

            group.bench_with_input(
                BenchmarkId::new("dlog_vec_batched_rolling_with_batch_size", batch_size),
                &batch_size,
                |b, &batch_size| {
                    b.iter(|| {
                        let recovered = bsgs::dlog_vec_batched_rolling_with_batch_size(
                            G,
                            &Hs,
                            &baby_table,
                            range_limit,
                            batch_size,
                        )
                        .expect("Discrete log not found");
                        assert_eq!(recovered, xs);
                    });
                },
            );

            group.bench_with_input(
                BenchmarkId::new("dlog_vec_batched_stepped_with_batch_size", batch_size),
                &batch_size,
                |b, &batch_size| {
                    b.iter(|| {
                        let recovered = bsgs::dlog_vec_batched_stepped_with_batch_size(
                            G,
                            &Hs,
                            &baby_table,
                            range_limit,
                            batch_size,
                        )
                        .expect("Discrete log not found");
                        assert_eq!(recovered, xs);
                    });
                },
            );
        }

        group.finish();
    }
}

#[allow(non_snake_case)]
fn bench_table_build<E: Pairing>(c: &mut Criterion, curve_name: &str) {
    let mut group = c.benchmark_group(format!("dlog_table_build_{}", curve_name));

    // Limit Criterion to exactly 10 measurement iterations, because tables can be big (24 bits takes 1-2 min)
    group.sample_size(10); // It can't do less than 10

    // Time seems almost linear in the size of the table, so doesn't make sense to benchmark many values
    let table_sizes: &[u64] = &[1u64 << 16, 1u64 << 20];

    let G = E::G1::generator();

    for &table_size in table_sizes {
        group.bench_with_input(
            BenchmarkId::from_parameter(table_size),
            &table_size,
            |b, &_ts| {
                b.iter(|| {
                    let table = table::build::<E::G1>(G, table_size);
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
    //    use ark_bn254::Bn254;

    // bench_dlog::<Bn254>(c, "bn254");
    // bench_dlog_batch_size::<Bn254>(c, "bn254");
    // bench_table_build::<Bn254>(c, "bn254");

    eprintln!("[bsgs] Starting bench_dlog_vec_small_range...");
    //    bench_dlog_vec_small_range::<Bls12_381>(c, "bls12_381");
    bench_dlog_vec_small_range::<Bls12_381>(c, "bls12_381"); // uncomment when that fn is defined
    eprintln!("[bsgs] Starting bench_table_build...");
    //bench_table_build::<Bls12_381>(c, "bls12_381");
}

criterion_group!(
    name = benches;
    config = Criterion::default().without_plots();
    targets = criterion_benchmark
);
criterion_main!(benches);
