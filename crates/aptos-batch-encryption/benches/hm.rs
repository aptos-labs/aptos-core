// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE
use aptos_batch_encryption::{group::{
    Fr, G1Affine, G1Projective, G2Affine, G2Prepared, PairingSetting,
}, schemes::hm::{self, HMDigestKey, TSPiece}, shared::algebra::fk_algorithm::FKDomain};
use ark_ec::{PrimeGroup as _, ScalarMul as _, VariableBaseMSM, pairing::Pairing};
use ark_poly::{EvaluationDomain, Radix2EvaluationDomain};
use ark_std::{rand::thread_rng, UniformRand, One};
use criterion::{criterion_group, criterion_main, BenchmarkId, Criterion};
use rayon::iter::{IndexedParallelIterator as _, IntoParallelRefIterator, ParallelIterator as _};

/// First rough-estimate benchmark for hyperinvertible matrix eval complexity
pub fn group_elt_fft(c: &mut Criterion) {
    let mut group = c.benchmark_group("group_elt_fft");
    let mut rng = thread_rng();

    for f_size in [32, 128, 256, 512] {
        let eval_domain : Radix2EvaluationDomain<Fr> = Radix2EvaluationDomain::new(f_size).unwrap();
        let gs = vec![G1Projective::rand(&mut rng); f_size];

        group.bench_with_input(
            BenchmarkId::from_parameter(f_size),
            &(eval_domain, gs),
            |b, input| {
                b.iter(|| input.0.fft(&input.1));
            },
        );
    }
}

pub fn combine_pieces(c: &mut Criterion) {
    let mut group = c.benchmark_group("combine_pieces");
    let mut rng = thread_rng();


    for num_pieces in [128, 256] {
        let dk = HMDigestKey::new(&mut rng, 128, num_pieces).unwrap();
        let pieces : Vec<TSPiece> = (0..num_pieces)
            .map(|_|
                TSPiece::generate(&mut rng, &dk, 0, 1)
            )
            .collect();

        group.bench_with_input(
            BenchmarkId::from_parameter(num_pieces),
            &pieces,
            |b, input| {
                b.iter(|| dk.combine_pieces(input));
            },
        );
    }
}

pub fn fk_prepare(c: &mut Criterion) {
    let mut group = c.benchmark_group("fk_prepare");
    let mut rng = thread_rng();

    for batch_size in [128] {
        let mut i = batch_size;
        let tau = Fr::rand(&mut rng);

        let mut tau_powers_fr = vec![Fr::one()];
        let mut cur = tau;
        for _ in 0..batch_size {
            tau_powers_fr.push(cur);
            cur *= &tau;
        }

        let tau_powers_g1: Vec<G1Projective> =
        G1Projective::generator().batch_mul(&tau_powers_fr)
            .into_iter()
            .map(G1Projective::from)
            .collect();


        group.bench_with_input(
            BenchmarkId::from_parameter(batch_size),
            &(tau_powers_g1),
            |b, input| {
                b.iter(||
                    FKDomain::new(batch_size, batch_size, vec![input.clone()])
                );
            },
        );
    }
}


pub fn batch_verify_pieces(c: &mut Criterion) {
    let mut group = c.benchmark_group("batch_verify_pieces");
    let mut rng = thread_rng();

    for f_size in [1, 3, 128, 256] {
        let g1s = vec![G1Affine::rand(&mut rng); f_size];
        let rand_exps = vec![Fr::rand(&mut rng); f_size];
        let g2s = vec![G2Prepared::from(G2Affine::rand(&mut rng)); f_size];

        group.bench_with_input(
            BenchmarkId::from_parameter(f_size),
            &(g1s, g2s, rand_exps),
            |b, input| {
                b.iter(|| {
                    let g1s : Vec<G1Projective> = input.0
                        .par_iter()
                        .zip(&input.2)
                        .map(|(g1, rand_fr)| *g1 * rand_fr)
                        .collect();

                    let pad_ml =
                    PairingSetting::multi_miller_loop(&g1s, input.1.clone());

                    PairingSetting::final_exponentiation(pad_ml).unwrap()
                });
            },
        );
    }
}

pub fn batch_verify_pieces_schnorr(c: &mut Criterion) {
    let mut group = c.benchmark_group("batch_verify_pieces_schnorr");
    let mut rng = thread_rng();

    for f_size in [1, 3, 128, 256] {
        let g1s_lhs = vec![G1Affine::rand(&mut rng); f_size];
        let g1s_rhs = vec![G1Affine::rand(&mut rng); f_size];
        let rand_exps = vec![Fr::rand(&mut rng); f_size];

        group.bench_with_input(
            BenchmarkId::from_parameter(f_size),
            &(g1s_lhs, g1s_rhs, rand_exps),
            |b, input| {
                b.iter(|| {
                    let rand_exps_neg : Vec<Fr> = input.2
                        .iter()
                        .map(|r| -*r)
                        .collect();
                    let mut exps_combined = input.2.clone();
                    exps_combined.extend_from_slice(&rand_exps_neg);
                    let mut bases_combined = input.0.clone();
                    bases_combined.extend_from_slice(&input.1);

                    G1Projective::msm(&bases_combined, &exps_combined)
                });
            },
        );
    }
}

criterion_group!(benches, group_elt_fft, fk_prepare, batch_verify_pieces, batch_verify_pieces_schnorr, combine_pieces);
criterion_main!(benches);
