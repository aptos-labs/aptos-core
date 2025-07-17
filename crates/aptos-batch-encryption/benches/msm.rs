use ark_ec::{pairing::Pairing, VariableBaseMSM};
use ark_std::{rand::thread_rng, UniformRand};
use aptos_batch_encryption::group::{PairingSetting, Fr, G1Affine, G2Affine, G1Projective};
use criterion::{criterion_group, criterion_main, BenchmarkId, Criterion};




pub fn msm(c: &mut Criterion) {
    let mut group = c.benchmark_group("msm");
    let mut rng = thread_rng();

    for f_size in [4, 8, 32, 128, 512 ] {
        let gs = vec![ G1Affine::rand(&mut rng); f_size ];
        let scalars = vec![ Fr::rand(&mut rng); f_size ];

        group.bench_with_input(BenchmarkId::from_parameter(f_size), &(gs, scalars), |b, input| {
            b.iter(||
                G1Projective::msm(&input.0, &input.1)
                );
        });
    }
}

pub fn pairing(c: &mut Criterion) {
    let mut group = c.benchmark_group("pairing");
    let mut rng = thread_rng();

    for f_size in [32, 128, 1449] {
        let g1s = vec![ G1Affine::rand(&mut rng); f_size ];
        let g2s = vec![ G2Affine::rand(&mut rng); f_size ];

        group.bench_with_input(BenchmarkId::from_parameter(f_size), &(g1s, g2s), |b, input| {
            b.iter(|| {
                let pad_ml = PairingSetting::multi_miller_loop(
                    &input.0,
                    &input.1);
                let pad = PairingSetting::final_exponentiation(pad_ml).unwrap();
                pad
            });
        });
    }
}

criterion_group!(benches, msm, pairing);
criterion_main!(benches);

