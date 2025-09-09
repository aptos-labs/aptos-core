// Copyright (c) Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use aptos_dkg::range_proof::{batch_prove, batch_verify, commit, setup, DST};
use blstrs::Scalar;
use criterion::{criterion_group, criterion_main, Criterion};
use rand::thread_rng;
use rand_core::RngCore;

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
                let zz: Vec<Scalar> = (0..n)
                    .map(|_| {
                        let val = rng.next_u64() >> (64 - ell);
                        Scalar::from(val)
                    })
                    .collect();
                let (cc, r) = commit(&pp, &zz, &mut rng);
                let fs_t = merlin::Transcript::new(DST);
                (pp, zz, cc, r, fs_t)
            },
            |(pp, z_vals, com, prover_state, mut fs_t)| {
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
                let zz: Vec<Scalar> = (0..n)
                    .map(|_| {
                        let val = rng.next_u64() >> (64 - ell);
                        Scalar::from(val)
                    })
                    .collect();
                let (cc, r) = commit(&pp, &zz, &mut rng);
                let mut fs_t = merlin::Transcript::new(DST);
                let proof = batch_prove(&mut rng, &pp, &zz, &cc, &r, &mut fs_t);
                let fs_t = merlin::Transcript::new(DST);
                (pp, cc, proof, fs_t)
            },
            |(pp, com, proof, mut fs_t)| {
                batch_verify(&pp, &com, &proof, &mut fs_t).unwrap();
            },
        )
    });
}

criterion_group!(
    name = benches;
    config = Criterion::default().sample_size(50);
    targets = bench_groups);
criterion_main!(benches);
