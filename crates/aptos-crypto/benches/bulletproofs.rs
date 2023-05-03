// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

#[macro_use]
extern crate criterion;

use criterion::{measurement::Measurement, BenchmarkGroup, Criterion, Throughput, BenchmarkId};
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
    for n in [8, 16, 32, 64] {
        range_proof_deserialize(&mut group, n);
    }

    for n in [8, 16, 32, 64] {
        range_proof_verify(&mut group, n);
    }

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

fn range_proof_deserialize<M: Measurement>(g: &mut BenchmarkGroup<M>, n: usize) {
    let mut rng = thread_rng();

    let bp_gens = BulletproofGens::new(n, 1);
    let pc_gens = PedersenGens::default();
    let mut i: u64 = 1;

    g.throughput(Throughput::Elements(1));
    g.bench_function(BenchmarkId::new("range_proof_deserialize", n), move |b| {
        b.iter_with_setup(
            || {
                let v = rng.gen_range(0, u64::MAX);

                // Sigh, some RngCore incompatibilites I don't want to deal with right now.
                let v_blinding =
                    Scalar::hash_from_bytes::<sha3::Sha3_512>(i.to_le_bytes().to_vec().as_slice());
                i += 1;

                let mut t = merlin::Transcript::new(b"AptosBenchmark");

                let proof = RangeProof::prove_single(&bp_gens, &pc_gens, &mut t, v, &v_blinding, n).unwrap();
                proof.0.to_bytes()
            },
            |proof_bytes| {
                assert!(RangeProof::from_bytes(&proof_bytes[..]).is_ok());
            },
        )
    });
}


fn range_proof_verify<M: Measurement>(g: &mut BenchmarkGroup<M>, n: usize) {
    let mut rng = thread_rng();

    let bp_gens = BulletproofGens::new(n, 1);
    let pc_gens = PedersenGens::default();
    let mut i: u64 = 1;
    let max: u64 = match n {
        8 => u8::MAX as u64,
        16 => u16::MAX as u64,
        32 => u32::MAX as u64,
        64 => u64::MAX,
        _ => panic!(),
    };

    g.throughput(Throughput::Elements(1));
    g.bench_function(BenchmarkId::new("range_proof_verify", n), move |b| {
        b.iter_with_setup(
            || {
                let v = rng.gen_range(0, max);

                // Sigh, some RngCore incompatibilites I don't want to deal with right now.
                let v_blinding =
                    Scalar::hash_from_bytes::<sha3::Sha3_512>(i.to_le_bytes().to_vec().as_slice());
                i += 1;

                let mut t = merlin::Transcript::new(b"AptosBenchmark");

                RangeProof::prove_single(&bp_gens, &pc_gens, &mut t, v as u64, &v_blinding, n).unwrap()
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
