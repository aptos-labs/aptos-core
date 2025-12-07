// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE
use aptos_batch_encryption::group::{
    Fr, G1Affine, G1Projective, G2Affine, G2Prepared, PairingSetting,
};
use ark_ec::{
    pairing::{Pairing, PairingOutput},
    VariableBaseMSM,
};
use ark_std::{rand::thread_rng, UniformRand};
use criterion::{criterion_group, criterion_main, BenchmarkId, Criterion};
use rayon::iter::{IntoParallelIterator as _, ParallelIterator as _};

pub fn msm(c: &mut Criterion) {
    let mut group = c.benchmark_group("msm");
    let mut rng = thread_rng();

    for f_size in [4, 8, 32, 128, 512] {
        let gs = vec![G1Affine::rand(&mut rng); f_size];
        let scalars = vec![Fr::rand(&mut rng); f_size];

        group.bench_with_input(
            BenchmarkId::from_parameter(f_size),
            &(gs, scalars),
            |b, input| {
                b.iter(|| G1Projective::msm(&input.0, &input.1));
            },
        );
    }
}

pub fn pairing(c: &mut Criterion) {
    let mut group = c.benchmark_group("pairing");
    let mut rng = thread_rng();

    for f_size in [1, 3, 128] {
        let g1s = vec![G1Affine::rand(&mut rng); f_size];
        let g2s = vec![G2Prepared::from(G2Affine::rand(&mut rng)); f_size];

        group.bench_with_input(
            BenchmarkId::from_parameter(f_size),
            &(g1s, g2s),
            |b, input| {
                b.iter(|| {
                    (0..128)
                        .into_par_iter()
                        .map(|_| {
                            let pad_ml =
                                PairingSetting::multi_miller_loop(&input.0, input.1.clone());

                            PairingSetting::final_exponentiation(pad_ml).unwrap()
                        })
                        .collect::<Vec<PairingOutput<ark_bn254::Bn254>>>()
                });
            },
        );
    }
}

pub fn gt_exp(c: &mut Criterion) {
    let mut group = c.benchmark_group("gt_exp");
    let mut rng = thread_rng();

    {
        let f_size = 1;
        let g1s = vec![G1Affine::rand(&mut rng); f_size];
        let g2s = vec![G2Affine::rand(&mut rng); f_size];
        let gt =
            PairingSetting::final_exponentiation(PairingSetting::multi_miller_loop(&g1s, &g2s))
                .unwrap();
        let fr = Fr::rand(&mut rng);

        group.bench_with_input(
            BenchmarkId::from_parameter(f_size),
            &(gt, fr),
            |b, input| {
                b.iter(|| input.0 * input.1);
            },
        );
    }
}

criterion_group!(benches, msm, pairing, gt_exp);
criterion_main!(benches);
