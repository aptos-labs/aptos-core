// Copyright (c) Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use aptos_dkg::{
    range_proofs::dekart_univariate::{batch_prove, batch_verify, DST},
    utils::test_utils,
};
use ark_ec::pairing::Pairing;
use ark_std::rand::thread_rng;
use criterion::{criterion_group, criterion_main, Criterion};

/// Generic benchmark function over any pairing curve
fn bench_range_proof<E: Pairing>(c: &mut Criterion, curve_name: &str) {
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
                let (pp, zz, cc, r) = test_utils::range_proof_random_instance(n, ell, &mut rng);
                (pp, zz, cc, r)
            },
            |(pp, z_vals, com, prover_state)| {
                let mut fs_t = merlin::Transcript::new(DST);
                let mut rng = thread_rng();
                let _proof =
                    batch_prove::<E, _>(&mut rng, &pp, &z_vals, &com, &prover_state, &mut fs_t);
            },
        )
    });

    group.bench_function(format!("verify/ell={ell}/n={n}").as_str(), |b| {
        b.iter_with_setup(
            || {
                let mut rng = thread_rng();
                let (pp, zz, cc, r) = test_utils::range_proof_random_instance(n, ell, &mut rng);
                let mut fs_t = merlin::Transcript::new(DST);
                let proof = batch_prove::<E, _>(&mut rng, &pp, &zz, &cc, &r, &mut fs_t);
                (pp, cc, proof)
            },
            |(pp, com, proof)| {
                let mut fs_t = merlin::Transcript::new(DST);
                batch_verify::<E>(&pp, &com, &proof, &mut fs_t).unwrap();
            },
        )
    });
}

// Specialize benchmark for a concrete pairing curve
fn bench_groups(c: &mut Criterion) {
    bench_range_proof::<ark_bn254::Bn254>(c, "BN254");
    bench_range_proof::<ark_bls12_381::Bls12_381>(c, "BLS12-381");
}

criterion_group!(
    name = benches;
    config = Criterion::default().sample_size(10);
    targets = bench_groups
);
criterion_main!(benches);
