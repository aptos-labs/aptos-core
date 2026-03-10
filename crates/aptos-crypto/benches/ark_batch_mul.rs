// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use ark_bls12_381::{Fr, G1Projective as G};
use ark_ec::scalar_mul::{BatchMulPreprocessing, ScalarMul};
use ark_std::UniformRand;
use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion};
use rayon::prelude::*;

fn bench_batch_mul(c: &mut Criterion) {
    let mut rng = ark_std::test_rng();

    // Fixed parameters
    let n = 219usize;
    let g = G::rand(&mut rng);

    // Random scalars
    let scalars: Vec<Fr> = (0..n).map(|_| Fr::rand(&mut rng)).collect();

    // Preprocessing for method (3)
    let table = BatchMulPreprocessing::new(g, n);

    let mut group = c.benchmark_group("scalar_mul_219");

    // ------------------------------------------------------------
    // 1. Naive multiplication: g * v[i]
    // ------------------------------------------------------------
    group.bench_function(BenchmarkId::new("naive_par", n), |b| {
        b.iter(|| {
            let res: Vec<G> = scalars
                .par_iter()
                .map(|s| black_box(g) * black_box(s))
                .collect();
            black_box(res);
        });
    });

    // ------------------------------------------------------------
    // 2. batch_mul (preprocessing done internally each call)
    // ------------------------------------------------------------
    group.bench_function(BenchmarkId::new("batch_mul", n), |b| {
        b.iter(|| {
            let res = black_box(g).batch_mul(black_box(&scalars));
            black_box(res);
        });
    });

    // ------------------------------------------------------------
    // 3. batch_mul_with_preprocessing (reuse table)
    // ------------------------------------------------------------
    group.bench_function(BenchmarkId::new("batch_mul_with_preprocessing", n), |b| {
        b.iter(|| {
            let res = G::batch_mul_with_preprocessing(black_box(&table), black_box(&scalars));
            black_box(res);
        });
    });

    group.finish();
}

criterion_group!(benches, bench_batch_mul);
criterion_main!(benches);
