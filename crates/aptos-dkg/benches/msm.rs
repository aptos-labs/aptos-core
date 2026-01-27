// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use ark_bls12_381::Bls12_381;
use ark_bn254::Bn254;
use ark_ec::{pairing::Pairing, CurveGroup, VariableBaseMSM};
use ark_ff::UniformRand;
use ark_std::{rand::thread_rng, One, Zero};
use criterion::{criterion_group, criterion_main, Criterion};

fn bench_two_term_msm<E: Pairing>(c: &mut Criterion, curve_name: &str) {
    let mut rng = thread_rng();

    // Generate random G1 points
    let ck_one_1 = E::G1::rand(&mut rng).into_affine();
    let ck_tau_1 = E::G1::rand(&mut rng).into_affine();

    // Generate random scalars
    let rho: <E as Pairing>::ScalarField = E::ScalarField::rand(&mut rng);
    let x = E::ScalarField::rand(&mut rng);
    let s0 = E::ScalarField::rand(&mut rng);

    // Benchmark direct arithmetic
    c.bench_function(&format!("direct arithmetic {}", curve_name), |b| {
        b.iter(|| {
            let _pi_2_direct = (ck_one_1 * rho) - (ck_tau_1 - ck_one_1 * x) * s0;
        });
    });

    // Benchmark MSM
    c.bench_function(&format!("msm {}", curve_name), |b| {
        b.iter(|| {
            let _pi_2_msm = E::G1::msm(&[ck_one_1, ck_tau_1], &[rho + s0 * x, -s0])
                .expect("MSM computation failed");
        });
    });
}

const N: usize = 219;

fn bench_large_msm_vs_sum_then_msm<E: Pairing>(c: &mut Criterion, curve_name: &str) {
    let mut rng = thread_rng();

    // First independent set of points
    let points_a: Vec<E::G1Affine> = (0..N)
        .map(|_| E::G1::rand(&mut rng).into_affine())
        .collect();

    // Second independent set of points
    let points_b: Vec<E::G1Affine> = (0..N)
        .map(|_| E::G1::rand(&mut rng).into_affine())
        .collect();

    // Scalars
    let ones: Vec<E::ScalarField> = vec![E::ScalarField::one(); N];
    let random_scalars: Vec<E::ScalarField> =
        (0..N).map(|_| E::ScalarField::rand(&mut rng)).collect();

    // Concatenate bases and scalars
    let mut all_points = Vec::with_capacity(2 * N);
    all_points.extend_from_slice(&points_a);
    all_points.extend_from_slice(&points_b);

    let mut all_scalars = Vec::with_capacity(2 * N);
    all_scalars.extend_from_slice(&ones);
    all_scalars.extend_from_slice(&random_scalars);

    // ------------------------------------------------------------
    // Case A: MSM(points_a, ones) + MSM(points_b, random)
    // ------------------------------------------------------------
    c.bench_function(
        &format!("single_msm_438 (219 ones + 219 random) {}", curve_name),
        |b| {
            b.iter(|| {
                let _res = E::G1::msm(&all_points, &all_scalars).expect("single MSM failed");
            });
        },
    );

    // ------------------------------------------------------------
    // Case B: sum(points_a) + MSM(points_b, random)
    // ------------------------------------------------------------
    c.bench_function(
        &format!("sum_219 + msm_219_random (distinct bases) {}", curve_name),
        |b| {
            b.iter(|| {
                let sum = points_a.iter().fold(E::G1::zero(), |acc, p| acc + p);

                let msm =
                    E::G1::msm(&points_b, &random_scalars).expect("MSM(points_b, random) failed");

                let _res = sum + msm;
            });
        },
    );
}

fn criterion_benchmark(c: &mut Criterion) {
    bench_two_term_msm::<Bn254>(c, "BN254");
    bench_two_term_msm::<Bls12_381>(c, "BLS12-381");
    bench_large_msm_vs_sum_then_msm::<Bn254>(c, "BN254");
    bench_large_msm_vs_sum_then_msm::<Bls12_381>(c, "BLS12-381");
}

criterion_group!(benches, criterion_benchmark);
criterion_main!(benches);
