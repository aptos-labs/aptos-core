// Copyright (c) Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use aptos_dkg::{
    range_proofs::{
        dekart_univariate::{Proof, DST},
        traits::BatchedRangeProof,
    },
    utils::test_utils,
};
use ark_ec::pairing::Pairing;
use ark_std::rand::thread_rng;
use criterion::{criterion_group, criterion_main, Criterion};

/// Generic benchmark function over any pairing curve
fn bench_range_proof<E: Pairing, B: BatchedRangeProof<E>>(c: &mut Criterion, curve_name: &str) {
    let mut group = c.benchmark_group(format!("range_proof/{}", curve_name));

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
                let (pk, _, witnesses, comm, comm_r) =
                    test_utils::range_proof_random_instance::<E, B, _>(n, ell, &mut rng);
                (pk, witnesses, comm, comm_r)
            },
            |(pk, values, comm, r)| {
                let mut fs_t = merlin::Transcript::new(DST);
                let mut rng = thread_rng();
                let _proof = B::prove(&pk, &values, ell, &comm, &r, &mut fs_t, &mut rng);
            },
        )
    });

    group.bench_function(format!("verify/ell={ell}/n={n}").as_str(), |b| {
        b.iter_with_setup(
            || {
                let mut rng = thread_rng();
                let (pk, vk, values, comm, r) =
                    test_utils::range_proof_random_instance::<E, B, _>(n, ell, &mut rng);
                let mut fs_t = merlin::Transcript::new(DST);
                let proof = B::prove(&pk, &values, ell, &comm, &r, &mut fs_t, &mut rng);
                (vk, comm, proof)
            },
            |(vk, comm, proof)| {
                let mut fs_t = merlin::Transcript::new(DST);
                proof.verify(&vk, &comm, &mut fs_t).unwrap();
            },
        )
    });
}

// Specialize benchmark for a concrete pairing curve
fn bench_groups(c: &mut Criterion) {
    use ark_bls12_381::Bls12_381;
    use ark_bn254::Bn254;

    bench_range_proof::<Bn254, Proof<Bn254>>(c, "BN254");
    bench_range_proof::<Bls12_381, Proof<Bls12_381>>(c, "BLS12-381");
}

criterion_group!(
    name = benches;
    config = Criterion::default().sample_size(10);
    targets = bench_groups
);
criterion_main!(benches);
