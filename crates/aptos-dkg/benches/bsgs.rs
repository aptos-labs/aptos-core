// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Compare dlog_vec, dlog_vec_batched, and dlog_vec_batched_rolling_with_batch_size
//! for various batch sizes and target counts.

use aptos_dkg::dlog::{bsgs, table};
use ark_ec::{pairing::Pairing, CurveGroup, PrimeGroup};
use criterion::{criterion_group, criterion_main, BenchmarkId, Criterion};
use rand::{rngs::StdRng, Rng, SeedableRng};

/// (vec_len, range_limit, table_size)
fn configs() -> &'static [(usize, u64, usize)] {
    &[
        //        (8, 1 << 32, 1 << 25),
        //        (8, 1 << 32, 1 << 25),
        (8, 1 << 39, 1 << 24),
        (8, 1 << 39, 1 << 25),
        (7, 1 << 44, 1 << 25),
    ]
}

fn batch_sizes() -> &'static [usize] {
    &[64, 128, 256, 512, 1024, 2048] // 512, 768, 1024, 2048, 4096, 8192] // &[1, 8, 32, 128, 256, 512, 1024, 2048, 4096, 8192]
}

#[allow(non_snake_case)]
fn bench_dlog_comparison<E: Pairing>(c: &mut Criterion, curve_name: &str) {
    let mut rng = StdRng::seed_from_u64(42);
    let G = E::G1::generator();

    for &(vec_len, range_limit, table_size) in configs() {
        let group_name = format!(
            "dlog_bsgs_{}_len{}_range2^{}_table2^{}",
            curve_name,
            vec_len,
            range_limit.ilog2(),
            table_size.ilog2()
        );
        let mut group = c.benchmark_group(&group_name);
        group.throughput(criterion::Throughput::Elements(vec_len as u64));

        println!(
            "Building baby table for curve {} with table size {}",
            curve_name, table_size
        );
        let t0 = std::time::Instant::now();
        let baby_table = table::BabyStepTable::new(G.into_affine(), table_size);
        println!(
            "Baby table built for curve {} with table size {} in {:?} (~{:.3} GB)",
            curve_name,
            table_size,
            t0.elapsed(),
            baby_table.size_gb()
        );
        let xs: Vec<u64> = (0..vec_len)
            .map(|_| rng.gen_range(0, range_limit))
            .collect();
        let Hs: Vec<E::G1> = xs.iter().map(|&x| G * E::ScalarField::from(x)).collect();

        // Baseline: no batching across giant steps or targets
        group.bench_with_input(BenchmarkId::new("dlog_vec", ""), &(), |b, _| {
            b.iter(|| {
                let recovered =
                    bsgs::dlog_vec(&baby_table, &Hs, range_limit).expect("dlog_vec failed");
                assert_eq!(recovered, xs);
            });
        });

        // Per-target batching only (batch_size applies to giant-step chunks per target)
        for &batch_size in batch_sizes() {
            group.bench_with_input(
                BenchmarkId::new("dlog_vec_batched", batch_size),
                &batch_size,
                |b, &batch_size| {
                    b.iter(|| {
                        let recovered =
                            bsgs::dlog_vec_batched(&baby_table, &Hs, range_limit, batch_size)
                                .expect("dlog_vec_batched failed");
                        assert_eq!(recovered, xs);
                    });
                },
            );
        }

        // Cross-target batching (one batch of points over all targets per chunk)
        for &batch_size in batch_sizes() {
            group.bench_with_input(
                BenchmarkId::new("dlog_vec_batched_rolling_with_batch_size", batch_size),
                &batch_size,
                |b, &batch_size| {
                    b.iter(|| {
                        let recovered = bsgs::dlog_vec_batched_rolling_with_batch_size(
                            &baby_table,
                            &Hs,
                            range_limit,
                            batch_size,
                        )
                        .expect("dlog_vec_batched_rolling failed");
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
    let table_sizes: &[usize] = &[1 << 20];
    let G = E::G1::generator();

    for &table_size in table_sizes {
        group.bench_with_input(
            BenchmarkId::from_parameter(table_size),
            &table_size,
            |b, &table_size| {
                b.iter(|| {
                    let t = table::BabyStepTable::new(G.into_affine(), table_size);
                    assert_eq!(t.table_size, table_size);
                });
            },
        );
    }
    group.finish();
}

fn criterion_benchmark(c: &mut Criterion) {
    use ark_bls12_381::Bls12_381;

    bench_dlog_comparison::<Bls12_381>(c, "BLS12-381");
    bench_table_build::<Bls12_381>(c, "BLS12-381");
}

criterion_group!(
    name = benches;
    config = Criterion::default().without_plots();
    targets = criterion_benchmark
);
criterion_main!(benches);
