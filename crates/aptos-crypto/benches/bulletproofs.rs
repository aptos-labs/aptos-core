// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

#[macro_use]
extern crate criterion;

use criterion::{measurement::Measurement, BenchmarkGroup, Criterion, Throughput};
use rand::{thread_rng, Rng};

use aptos_crypto::bulletproofs::MAX_RANGE_BITS;
use bulletproofs::BulletproofGens;
use bulletproofs::PedersenGens;
use bulletproofs::RangeProof;
use curve25519_dalek_ng::scalar::Scalar;
use merlin::Transcript;

fn bench_group(c: &mut Criterion) {
    let mut group = c.benchmark_group("bulletproofs");

    range_prove(&mut group);
    range_proof_verify(&mut group);

    group.finish();
}

fn range_prove<M: Measurement>(g: &mut BenchmarkGroup<M>) {
    let mut rng = thread_rng();

    let pg = PedersenGens::default();
    let bg = BulletproofGens::new(MAX_RANGE_BITS, 1);

    g.throughput(Throughput::Elements(1));
    g.bench_function("range_prove", move |b| {
        b.iter_with_setup(
            || {
                let value = rng.gen_range(0u64, (2u128.pow(MAX_RANGE_BITS as u32) - 1u128) as u64);
                let blinder = Scalar::hash_from_bytes::<sha3::Sha3_512>(b"some random blinder");

                (value, blinder)
            },
            |(value, blinder)| {
                let mut t_prv = Transcript::new(b"some DST");
                assert!(RangeProof::prove_single(
                    &bg,
                    &pg,
                    &mut t_prv,
                    value,
                    &blinder,
                    MAX_RANGE_BITS
                )
                .is_ok());
            },
        )
    });
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
