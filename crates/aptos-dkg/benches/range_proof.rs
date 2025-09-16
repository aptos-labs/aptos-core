// Copyright (c) Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use aptos_dkg::range_proofs::univariate_range_proof::{
    batch_prove, batch_verify, commit, setup, DST,
};
use blstrs::Scalar;
use criterion::{criterion_group, criterion_main, Criterion};
use ark_std::rand::{thread_rng, RngCore};

use ark_bn254::{
    // TODO: move this elsewhere
    g1::Config as G1Config,
    Bn254 as PairingSetting,
    Config,
    Fq,
    Fq12,
    Fr,
    G1Affine,
    G1Projective,
    G2Affine,
    G2Projective,
};

pub fn bench_groups(c: &mut Criterion) {
    let mut group = c.benchmark_group("range_proof");

    let ell = std::env::var("L")
        .unwrap_or(std::env::var("ELL").unwrap_or_default())
        .parse::<usize>()
        .unwrap_or(48);

    let n = std::env::var("N")
        .unwrap_or_default()
        .parse::<usize>()
        .unwrap_or(2048 - 1);

    group.bench_function(format!("prove/ell={ell}/n={n}").as_str(), move |b| {
        b.iter_with_setup(
            || {
                let mut rng = thread_rng();
                let pp = setup(ell, n);
                let zz: Vec<Fr> = (0..n)
                    .map(|_| {
                        let val = rng.next_u64() >> (64 - ell); // Keep lowest ell bits
                        Fr::from(val)
                    })
                    .collect();
                let (cc, r) = commit(&pp, &zz, &mut rng);
                (pp, zz, cc, r)
            },
            |(pp, z_vals, com, prover_state)| {
                let mut fs_t = merlin::Transcript::new(DST);
                let mut rng = thread_rng();
                let _proof = batch_prove(&mut rng, &pp, &z_vals, &com, &prover_state, &mut fs_t);
            },
        )
    });
    group.bench_function(format!("verify/ell={ell}/n={n}").as_str(), |b| {
        b.iter_with_setup(
            || {
                let mut rng = thread_rng();
                let pp = setup(ell, n);
                let zz: Vec<Fr> = (0..n)
                    .map(|_| {
                        let val = rng.next_u64() >> (64 - ell); // Keep lowest ell bits
                        Fr::from(val)
                    })
                    .collect();
                let (cc, r) = commit(&pp, &zz, &mut rng);
                let mut fs_t = merlin::Transcript::new(DST);
                let proof = batch_prove(&mut rng, &pp, &zz, &cc, &r, &mut fs_t);
                (pp, cc, proof)
            },
            |(pp, com, proof)| {
                let mut fs_t = merlin::Transcript::new(DST);
                batch_verify(&pp, &com, &proof, &mut fs_t).unwrap();
            },
        )
    });
}

criterion_group!(
    name = benches;
    config = Criterion::default().sample_size(10);
    targets = bench_groups);
criterion_main!(benches);
