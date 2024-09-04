// Copyright (c) Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use blstrs::Scalar;
use criterion::{criterion_group, criterion_main, measurement::{Measurement, WallTime}, BenchmarkGroup, Criterion, Throughput, BenchmarkId};
use rand::thread_rng;
use rand_core::RngCore;
use aptos_dkg::range_proof::{batch_prove, batch_verify, commit, powers_of_tau, setup};

pub fn bench_groups(c: &mut Criterion) {
    let mut group = c.benchmark_group("range_proof");
    let num_chunks = std::env::var("NUM_CHUNKS").unwrap_or_default().parse::<usize>().unwrap_or(32);
    let batch_size = std::env::var("BATCH_SIZE").unwrap_or_default().parse::<usize>().unwrap_or(8191);
    group.bench_function(format!("prove/num_chunks={num_chunks}/batch_size={batch_size}").as_str(), move |b| {
        b.iter_with_setup(
            || {
                let mut rng = thread_rng();
                let n_ptau_required = batch_size + 1;
                let ptau = powers_of_tau(&mut rng, n_ptau_required);
                let pp = setup(ptau, num_chunks, batch_size);
                let z_vals: Vec<Scalar> = (0..batch_size).map(|_| {
                    let val = rng.next_u64() >> (64 - num_chunks);
                    Scalar::from(val)
                }).collect();
                let (com, prover_state) = commit(&pp, &z_vals, &mut rng);
                (pp, z_vals, com, prover_state)
            },
            |(pp, z_vals, com, prover_state)| {
                let mut rng = thread_rng();
                let _proof = batch_prove(&mut rng, &pp, &z_vals, &com, &prover_state);
            }
        )
    });
    group.bench_function(format!("verify/num_chunks={num_chunks}/batch_size={batch_size}").as_str(), |b| {
        b.iter_with_setup(
            || {
                let mut rng = thread_rng();
                let n_ptau_required = batch_size + 1;
                let ptau = powers_of_tau(&mut rng, n_ptau_required);
                let pp = setup(ptau, num_chunks, batch_size);
                let z_vals: Vec<Scalar> = (0..batch_size).map(|_| {
                    let val = rng.next_u64() >> (64 - num_chunks);
                    Scalar::from(val)
                }).collect();
                let (com, prover_state) = commit(&pp, &z_vals, &mut rng);
                let proof = batch_prove(&mut rng, &pp, &z_vals, &com, &prover_state);
                (pp, com, proof)
            },
            |(pp, com, proof)| {
                batch_verify(&pp, &com, &proof).unwrap();
            }
        )
    });
}

criterion_group!(
    name = benches;
    config = Criterion::default();
    targets = bench_groups);
criterion_main!(benches);
