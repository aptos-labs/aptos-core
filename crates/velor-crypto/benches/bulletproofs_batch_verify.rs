// Copyright © Velor Foundation
// SPDX-License-Identifier: Apache-2.0

#[macro_use]
extern crate criterion;

use velor_crypto::bulletproofs::MAX_RANGE_BITS;
use bulletproofs::{BulletproofGens, PedersenGens, RangeProof};
use criterion::{measurement::Measurement, BenchmarkGroup, BenchmarkId, Criterion, Throughput};
use curve25519_dalek_ng::scalar::Scalar;
use merlin::Transcript;
use rand::{thread_rng, Rng};
use rand_core::RngCore;

fn get_values(num_bits: usize, batch_size: usize) -> (Vec<u64>, Vec<Scalar>) {
    let mut rng = thread_rng();

    let v = (0..batch_size)
        .map(|_| rng.gen_range(0u64, (2u128.pow(num_bits as u32) - 1u128) as u64))
        .collect::<Vec<u64>>();

    // Sigh, some RngCore incompatibilities I don't want to deal with right now.
    let b = (0..batch_size)
        .map(|_| {
            let mut scalar = [0u8; 32];
            rng.fill_bytes(&mut scalar);

            Scalar::from_bytes_mod_order(scalar)
        })
        .collect::<Vec<Scalar>>();

    (v, b)
}

fn bench_group(c: &mut Criterion) {
    let mut group = c.benchmark_group("bulletproofs_batch_verify");

    for batch_size in [1, 2, 4, 8, 16] {
        for num_bits in [8, 16, 32, 64] {
            range_batch_prove(&mut group, num_bits, batch_size);
            range_batch_verify(&mut group, num_bits, batch_size);
        }
    }

    group.finish();
}

fn range_batch_prove<M: Measurement>(
    g: &mut BenchmarkGroup<M>,
    num_bits: usize,
    batch_size: usize,
) {
    let pg = PedersenGens::default();
    let bg = BulletproofGens::new(MAX_RANGE_BITS, 16);

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
                    let mut dst = [0_u8; 256];
                    thread_rng().fill(&mut dst);
                    let mut t_prv = Transcript::new(&dst);
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

fn range_batch_verify<M: Measurement>(
    g: &mut BenchmarkGroup<M>,
    num_bits: usize,
    batch_size: usize,
) {
    let bp_gens = BulletproofGens::new(MAX_RANGE_BITS, 16);
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
                    let mut dst = [0_u8; 256];
                    thread_rng().fill(&mut dst);
                    let mut t = Transcript::new(&dst);

                    let (proof, comm) = RangeProof::prove_multiple(
                        &bp_gens,
                        &pc_gens,
                        &mut t,
                        v.as_slice(),
                        b.as_slice(),
                        num_bits,
                    )
                    .unwrap();

                    (dst, proof.to_bytes(), comm)
                },
                |(dst, proof_bytes, comm)| {
                    let mut t = Transcript::new(&dst);

                    let proof = RangeProof::from_bytes(&proof_bytes[..]).unwrap();

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
