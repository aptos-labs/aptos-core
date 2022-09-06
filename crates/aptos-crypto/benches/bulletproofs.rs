// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

#[macro_use]
extern crate criterion;

use criterion::{measurement::Measurement, BenchmarkGroup, Criterion, Throughput};
use rand::{thread_rng, Rng};

use bulletproofs::BulletproofGens;
use bulletproofs::PedersenGens;
use bulletproofs::RangeProof;
use curve25519_dalek_ng::scalar::Scalar;

fn bench_group(c: &mut Criterion) {
    let mut group = c.benchmark_group("bulletproofs");

    range_proof_verify(&mut group);

    group.finish();
}

fn range_proof_verify<M: Measurement>(g: &mut BenchmarkGroup<M>) {
    let mut rng = thread_rng();

    let n = 64;
    let bp_gens = BulletproofGens::new(n, 1);
    let pc_gens = PedersenGens::default();
    let mut i: u64 = 1;

    g.throughput(Throughput::Elements(1));
    g.bench_function("range_proof_verify", move |b| {
        b.iter_with_setup(
            || {
                let v = rng.gen_range(0, u64::MAX);

                // Sigh, some RngCore incompatibilites I don't want to deal with right now.
                let v_blinding =
                    Scalar::hash_from_bytes::<sha3::Sha3_512>(i.to_le_bytes().to_vec().as_slice());
                i += 1;

                let mut t = merlin::Transcript::new(b"AptosBenchmark");

                RangeProof::prove_single(&bp_gens, &pc_gens, &mut t, v, &v_blinding, n).unwrap()
            },
            |(proof, comm)| {
                let mut t = merlin::Transcript::new(b"AptosBenchmark");

                assert!(proof
                    .verify_single(&bp_gens, &pc_gens, &mut t, &comm, n)
                    .is_ok());
            },
        )
    });
}

criterion_group!(bulletproofs_benches, bench_group);
criterion_main!(bulletproofs_benches);
