// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE
use aptos_batch_encryption::{
    group::{Fr, G1Projective},
    shared::algebra::multi_point_eval,
};
use ark_ec::PrimeGroup;
use ark_poly::{EvaluationDomain, Radix2EvaluationDomain};
use ark_std::{rand::thread_rng, UniformRand};
use criterion::{criterion_group, criterion_main, BenchmarkId, Criterion};

pub fn multi_point_eval(c: &mut Criterion) {
    let mut group = c.benchmark_group("multi_point_eval");
    let mut rng = thread_rng();

    for f_size in [4, 8, 32, 128, 512] {
        let f = vec![G1Projective::rand(&mut rng); f_size];
        let x_coords = vec![Fr::rand(&mut rng); f_size];

        group.bench_with_input(
            BenchmarkId::from_parameter(f_size),
            &(f, x_coords),
            |b, input| {
                b.iter(|| multi_point_eval::multi_point_eval(&input.0, &input.1));
            },
        );
    }
}

pub fn multi_point_eval_field(c: &mut Criterion) {
    let mut group = c.benchmark_group("multi_point_eval_field");
    let mut rng = thread_rng();

    for f_size in [4, 8, 32, 128, 512] {
        let f = vec![Fr::rand(&mut rng); f_size];
        let x_coords = vec![Fr::rand(&mut rng); f_size];

        group.bench_with_input(
            BenchmarkId::from_parameter(f_size),
            &(f, x_coords),
            |b, input| {
                b.iter(|| multi_point_eval::multi_point_eval(&input.0, &input.1));
            },
        );
    }
}

pub fn multi_point_eval_small(c: &mut Criterion) {
    let mut group = c.benchmark_group("multi_point_eval_small");
    let mut rng = thread_rng();

    for f_size in [4, 8, 32, 128, 512] {
        let f_evals = vec![G1Projective::generator(); f_size];
        let f = Radix2EvaluationDomain::<Fr>::new(f_size)
            .unwrap()
            .ifft(&f_evals);
        let x_coords = vec![Fr::rand(&mut rng); f_size];

        group.bench_with_input(
            BenchmarkId::from_parameter(f_size),
            &(f, x_coords),
            |b, input| {
                b.iter(|| multi_point_eval::multi_point_eval(&input.0, &input.1));
            },
        );
    }
}

criterion_group!(
    benches,
    multi_point_eval,
    multi_point_eval_field,
    multi_point_eval_small
);
criterion_main!(benches);
