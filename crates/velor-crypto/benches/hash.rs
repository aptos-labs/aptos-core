// Copyright © Velor Foundation
// SPDX-License-Identifier: Apache-2.0

#[macro_use]
extern crate criterion;

use velor_crypto::{bls12381::DST_BLS_SIG_IN_G2_WITH_POP, test_utils::random_bytes};
use blake2::{
    digest::{Update, VariableOutput},
    Blake2bVar,
};
use blake2_rfc::blake2b::Blake2b;
use criterion::{
    measurement::Measurement, AxisScale, BenchmarkGroup, BenchmarkId, Criterion, PlotConfiguration,
    Throughput,
};
use digest::Digest;
use rand::thread_rng;
use sha2::{Sha256, Sha512};
use std::ptr::null;
use tiny_keccak::{Hasher as KeccakHasher, Keccak};

/// Runs all the benchmarks.
fn bench_group(c: &mut Criterion) {
    let mut group = c.benchmark_group("hash");

    let plot_config = PlotConfiguration::default().summary_scale(AxisScale::Logarithmic);

    group.sample_size(1000);
    group.plot_config(plot_config);

    let mut sizes = vec![0, 1];

    let mut size = *sizes.last().unwrap();
    for _ in 1..=10 {
        size *= 2;
        sizes.push(size);
    }

    for n in sizes {
        sha2_256(&mut group, n);
        sha2_512(&mut group, n);
        sha3_256(&mut group, n);
        hash_to_g1(&mut group, n, DST_BLS_SIG_IN_G2_WITH_POP);
        hash_to_g2(&mut group, n, DST_BLS_SIG_IN_G2_WITH_POP);
        keccak256(&mut group, n);
        blake2_blake2b_256(&mut group, n);
        blake2_rfc_blake2b_256(&mut group, n);
    }

    group.finish();
}

/// Benchmarks the time to hash an arbitrary message of size n bytes using SHA2-256
fn sha2_256<M: Measurement>(g: &mut BenchmarkGroup<M>, n: usize) {
    let mut rng = thread_rng();

    g.throughput(Throughput::Bytes(n as u64));

    g.bench_function(BenchmarkId::new("SHA2-256", n), move |b| {
        b.iter_with_setup(
            || random_bytes(&mut rng, n),
            |bytes| {
                assert_eq!(bytes.len(), n);

                let mut hasher = Sha256::new();

                hasher.update(bytes);

                let output = hasher.finalize();
                assert_eq!(output.as_slice().len(), 32);
            },
        )
    });
}

/// Benchmarks the time to hash an arbitrary message of size n bytes using SHA2-512
fn sha2_512<M: Measurement>(g: &mut BenchmarkGroup<M>, n: usize) {
    let mut rng = thread_rng();

    g.throughput(Throughput::Bytes(n as u64));

    g.bench_function(BenchmarkId::new("SHA2-512", n), move |b| {
        b.iter_with_setup(
            || random_bytes(&mut rng, n),
            |bytes| {
                assert_eq!(bytes.len(), n);

                let mut hasher = Sha512::new();

                hasher.update(bytes);

                let output = hasher.finalize();
                assert_eq!(output.as_slice().len(), 64);
            },
        )
    });
}

/// Benchmarks the time to hash an arbitrary message of size n bytes using SHA3-256
fn sha3_256<M: Measurement>(g: &mut BenchmarkGroup<M>, n: usize) {
    let mut rng = thread_rng();

    g.throughput(Throughput::Bytes(n as u64));

    g.bench_function(BenchmarkId::new("SHA3-256", n), move |b| {
        b.iter_with_setup(
            || random_bytes(&mut rng, n),
            |bytes| {
                assert_eq!(bytes.len(), n);

                let mut hasher = sha3::Sha3_256::new();

                hasher.update(bytes);

                let output = hasher.finalize();
                assert_eq!(output.as_slice().len(), 32);
            },
        )
    });
}

/// Benchmarks the time to hash an arbitrary message of size n bytes using Keccak-256
fn keccak256<M: Measurement>(g: &mut BenchmarkGroup<M>, n: usize) {
    let mut rng = thread_rng();

    g.throughput(Throughput::Bytes(n as u64));

    g.bench_function(BenchmarkId::new("Keccak-256", n), move |b| {
        b.iter_with_setup(
            || random_bytes(&mut rng, n),
            |bytes| {
                assert_eq!(bytes.len(), n);

                let mut hasher = Keccak::v256();
                hasher.update(&bytes);

                let mut output = [0u8; 32];
                hasher.finalize(&mut output);

                assert_eq!(output.as_slice().len(), 32);
            },
        )
    });
}

/// Benchmarks the time to hash an arbitrary message of size n bytes into G_1 using the specified DST.
pub fn hash_to_g1<M: Measurement>(g: &mut BenchmarkGroup<M>, n: usize, dst: &[u8]) {
    let mut rng = thread_rng();

    g.throughput(Throughput::Bytes(n as u64));

    g.bench_function(BenchmarkId::new("hash_to_bls12381_g1", n), move |b| {
        b.iter_with_setup(
            || random_bytes(&mut rng, n),
            |mut bytes| unsafe {
                assert_eq!(bytes.len(), n);
                let mut elem = blst::blst_p1::default();

                let elem_ptr: *mut blst::blst_p1 = &mut elem;
                let bytes_ptr: *mut u8 = bytes.as_mut_ptr();

                blst::blst_hash_to_g1(
                    elem_ptr,
                    bytes_ptr,
                    bytes.len(),
                    dst.as_ptr(),
                    dst.len(),
                    null(),
                    0,
                );
            },
        )
    });
}

/// Benchmarks the time to hash an arbitrary message of size n bytes into G_2 using the specified DST.
pub fn hash_to_g2<M: Measurement>(g: &mut BenchmarkGroup<M>, n: usize, dst: &[u8]) {
    let mut rng = thread_rng();

    g.throughput(Throughput::Bytes(n as u64));

    g.bench_function(BenchmarkId::new("hash_to_bls12381_g2", n), move |b| {
        b.iter_with_setup(
            || random_bytes(&mut rng, n),
            |mut bytes| unsafe {
                assert_eq!(bytes.len(), n);
                let mut elem = blst::blst_p2::default();

                let elem_ptr: *mut blst::blst_p2 = &mut elem;
                let bytes_ptr: *mut u8 = bytes.as_mut_ptr();

                blst::blst_hash_to_g2(
                    elem_ptr,
                    bytes_ptr,
                    bytes.len(),
                    dst.as_ptr(),
                    dst.len(),
                    null(),
                    0,
                );
            },
        )
    });
}

fn blake2_blake2b_256<M: Measurement>(g: &mut BenchmarkGroup<M>, n: usize) {
    let mut rng = thread_rng();

    g.throughput(Throughput::Bytes(n as u64));

    g.bench_function(BenchmarkId::new("blake2b-256/crate-blake2", n), move |b| {
        b.iter_with_setup(
            || random_bytes(&mut rng, n),
            |bytes| {
                assert_eq!(bytes.len(), n);

                let mut hasher = Blake2bVar::new(32).unwrap();
                hasher.update(&bytes);
                let mut output = vec![0u8; 32];
                hasher.finalize_variable(&mut output).unwrap();

                assert_eq!(output.as_slice().len(), 32);
            },
        )
    });
}

fn blake2_rfc_blake2b_256<M: Measurement>(g: &mut BenchmarkGroup<M>, n: usize) {
    let mut rng = thread_rng();

    g.throughput(Throughput::Bytes(n as u64));

    g.bench_function(
        BenchmarkId::new("blake2b-256/crate-blake2-rfc", n),
        move |b| {
            b.iter_with_setup(
                || random_bytes(&mut rng, n),
                |bytes| {
                    assert_eq!(bytes.len(), n);

                    // Using the state context.
                    let mut context = Blake2b::new(32);
                    context.update(&bytes);
                    let hash = context.finalize();
                    assert_eq!(hash.as_bytes().len(), 32);
                },
            )
        },
    );
}

criterion_group!(
    name = hash_benches;
    config = Criterion::default(); //.measurement_time(Duration::from_secs(100));
    targets = bench_group);
criterion_main!(hash_benches);
