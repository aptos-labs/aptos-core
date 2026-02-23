// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

#[macro_use]
extern crate criterion;

use bulletproofs::{BulletproofGens, PedersenGens, RangeProof};
use criterion::{measurement::Measurement, BenchmarkGroup, BenchmarkId, Criterion, Throughput};
use curve25519_dalek_ng::scalar::Scalar;
use merlin::Transcript;
use rand::{thread_rng, Rng};
use rand_core::RngCore;

const DST: &[u8] = b"dummy DST";

/// WARNING: Do not change this, since our range proof benchmark instructions in README.md rely on it.
const GROUP_NAME: &str = "bulletproofs";

/// WARNING: See `GROUP_NAME`.
const PROVE_BENCH_ID: &str = "range_prove";
/// WARNING: See `GROUP_NAME`.
const VERIFY_BENCH_ID: &str = "range_verify";

/// WARNING: See `GROUP_NAME`.
fn get_benchmark_subid(batch_size: usize, num_bits: usize) -> String {
    format!("batch={}/bits={}", batch_size, num_bits)
}

/// WARNING: These are the relevant batch sizes we want benchmarked to compare against DeKART
//const BATCH_SIZES: [usize; 11] = [2, 4, 8, 16, 32, 64, 128, 256, 512, 1024, 2048];
const BATCH_SIZES: [usize; 2] = [1024, 16384]; //, 131072, 1048576];

/// WARNING: These are the relevant bit widths we want benchmarked to compare against DeKART
const BIT_WIDTHS: [usize; 4] = [8, 16, 32, 64];

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
    let mut group = c.benchmark_group(GROUP_NAME);

    // NOTE: Commented out since these take < 1 microsecond.
    // for num_bits in [32, 64] {
    //     range_proof_deserialize(&mut group, num_bits);
    // }

    // WARNING: These test cases were picked to benchmark against univariate DeKART
    for batch_size in BATCH_SIZES {
        for num_bits in BIT_WIDTHS {
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
        BenchmarkId::new(PROVE_BENCH_ID, get_benchmark_subid(batch_size, num_bits)),
        move |b| {
            b.iter_with_setup(
                || get_values(num_bits, batch_size),
                |(v, b)| {
                    let mut t_prv = Transcript::new(DST);
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

#[allow(dead_code)]
/// Note: For now, this only benchmarks deserialization of a range proof for 1 value.
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

                    let mut t = Transcript::new(DST);

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
        BenchmarkId::new(VERIFY_BENCH_ID, get_benchmark_subid(batch_size, num_bits)),
        move |b| {
            b.iter_with_setup(
                || {
                    let (v, b) = get_values(num_bits, batch_size);

                    let mut t = Transcript::new(DST);

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
                    let mut t = Transcript::new(DST);

                    assert!(proof
                        .verify_multiple(&bp_gens, &pc_gens, &mut t, &comm, num_bits)
                        .is_ok());
                },
            )
        },
    );
}

criterion_group!(
    name = bulletproofs_benches;
    config = Criterion::default().sample_size(10);
    targets = bench_group);
criterion_main!(bulletproofs_benches);
