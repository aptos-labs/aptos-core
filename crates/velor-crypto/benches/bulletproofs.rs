// Copyright © Velor Foundation
// SPDX-License-Identifier: Apache-2.0

#[macro_use]
extern crate criterion;

use bulletproofs::{BulletproofGens, PedersenGens, RangeProof};
use criterion::{measurement::Measurement, BenchmarkGroup, BenchmarkId, Criterion, Throughput};
use curve25519_dalek_ng::scalar::Scalar;
use merlin::Transcript;
use rand::{thread_rng, Rng};

fn get_values(num_bits: usize, batch_size: usize) -> (Vec<u64>, Vec<Scalar>) {
    let mut rng = thread_rng();

    let v = (0..batch_size)
        .map(|_| rng.gen_range(0u64, (2u128.pow(num_bits as u32) - 1u128) as u64))
        .collect::<Vec<u64>>();

    // Sigh, some RngCore incompatibilities I don't want to deal with right now.
    let b = (0..batch_size)
        .map(|_| Scalar::hash_from_bytes::<sha3::Sha3_512>(b"some random blinder"))
        .collect::<Vec<Scalar>>();

    (v, b)
}

fn bench_group(c: &mut Criterion) {
    let mut group = c.benchmark_group("bulletproofs");

    for num_bits in [32, 64] {
        range_proof_deserialize(&mut group, num_bits);
    }

    for batch_size in [1, 2] {
        for num_bits in [32, 64] {
            range_prove(&mut group, num_bits, batch_size);
            range_verify(&mut group, num_bits, batch_size);
        }
    }

    group.finish();
}

fn range_prove<M: Measurement>(g: &mut BenchmarkGroup<M>, num_bits: usize, batch_size: usize) {
    let pg = PedersenGens::default();
    let bg = BulletproofGens::new(num_bits, batch_size);

    g.throughput(Throughput::Elements(batch_size as u64));
    g.bench_function(
        BenchmarkId::new(
            "range_prove",
            format!("batch={}/bits={}", batch_size, num_bits),
        ),
        move |b| {
            b.iter_with_setup(
                || get_values(num_bits, batch_size),
                |(v, b)| {
                    let mut t_prv = Transcript::new(b"some DST");
                    assert!(RangeProof::prove_multiple(
                        &bg,
                        &pg,
                        &mut t_prv,
                        v.as_slice(),
                        b.as_slice(),
                        num_bits
                    )
                    .is_ok());
                },
            )
        },
    );
}

fn range_proof_deserialize<M: Measurement>(g: &mut BenchmarkGroup<M>, num_bits: usize) {
    let bp_gens = BulletproofGens::new(num_bits, 1);
    let pc_gens = PedersenGens::default();

    g.throughput(Throughput::Elements(1));
    g.bench_function(
        BenchmarkId::new("range_proof_deserialize", num_bits),
        move |b| {
            b.iter_with_setup(
                || {
                    let (v, b) = get_values(num_bits, 1);

                    let mut t = merlin::Transcript::new(b"VelorBenchmark");

                    let proof = RangeProof::prove_multiple(
                        &bp_gens,
                        &pc_gens,
                        &mut t,
                        v.as_slice(),
                        b.as_slice(),
                        num_bits,
                    )
                    .unwrap();
                    proof.0.to_bytes()
                },
                |proof_bytes| {
                    assert!(RangeProof::from_bytes(&proof_bytes[..]).is_ok());
                },
            )
        },
    );
}

fn range_verify<M: Measurement>(g: &mut BenchmarkGroup<M>, num_bits: usize, batch_size: usize) {
    let bp_gens = BulletproofGens::new(num_bits, batch_size);
    let pc_gens = PedersenGens::default();

    g.throughput(Throughput::Elements(batch_size as u64));
    g.bench_function(
        BenchmarkId::new(
            "range_verify",
            format!("batch={}/bits={}", batch_size, num_bits),
        ),
        move |b| {
            b.iter_with_setup(
                || {
                    let (v, b) = get_values(num_bits, batch_size);

                    let mut t = merlin::Transcript::new(b"VelorBenchmark");

                    RangeProof::prove_multiple(
                        &bp_gens,
                        &pc_gens,
                        &mut t,
                        v.as_slice(),
                        b.as_slice(),
                        num_bits,
                    )
                    .unwrap()
                },
                |(proof, comm)| {
                    let mut t = merlin::Transcript::new(b"VelorBenchmark");

                    assert!(proof
                        .verify_multiple(&bp_gens, &pc_gens, &mut t, &comm, num_bits)
                        .is_ok());
                },
            )
        },
    );
}

criterion_group!(bulletproofs_benches, bench_group);
criterion_main!(bulletproofs_benches);
