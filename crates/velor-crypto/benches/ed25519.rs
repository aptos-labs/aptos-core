// Copyright © Velor Foundation
// Parts of the project are originally copyright © Meta Platforms, Inc.
// SPDX-License-Identifier: Apache-2.0

#[macro_use]
extern crate criterion;

use velor_crypto::{
    ed25519::{Ed25519PrivateKey, Ed25519PublicKey, Ed25519Signature},
    traits::{Signature, SigningKey, Uniform},
    PrivateKey,
};
use velor_crypto_derive::{BCSCryptoHash, CryptoHasher};
use criterion::{measurement::Measurement, BenchmarkGroup, Criterion, Throughput};
use curve25519_dalek::{constants::ED25519_BASEPOINT_POINT, scalar::Scalar};
use rand::{distributions, prelude::ThreadRng, thread_rng, Rng};
use serde::{Deserialize, Serialize};

#[derive(Debug, CryptoHasher, BCSCryptoHash, Serialize, Deserialize)]
pub struct TestVelorCrypto(pub String);

fn random_message(rng: &mut ThreadRng) -> TestVelorCrypto {
    TestVelorCrypto(
        rng.sample_iter(&distributions::Alphanumeric)
            .take(256)
            .collect::<String>(),
    )
}

fn benchmark_groups(c: &mut Criterion) {
    let mut group = c.benchmark_group("ed25519");

    group.sample_size(1000);

    sig_verify_struct(&mut group);
    sig_verify_zero_bytes(&mut group);
    pk_deserialize(&mut group);
    sig_deserialize(&mut group);
    small_subgroup_check(&mut group);

    group.finish();
}

fn sig_verify_struct<M: Measurement>(g: &mut BenchmarkGroup<M>) {
    let mut csprng: ThreadRng = thread_rng();

    let priv_key = Ed25519PrivateKey::generate(&mut csprng);
    let pub_key: Ed25519PublicKey = (&priv_key).into();

    g.throughput(Throughput::Elements(1));
    g.bench_function("sig_verify_struct", move |b| {
        b.iter_with_setup(
            || {
                let msg = random_message(&mut csprng);
                let sig = priv_key.sign(&msg).unwrap();
                (sig, msg)
            },
            |(sig, msg)| sig.verify(&msg, &pub_key),
        )
    });
}

/// Benchmarks the time to verify a signature on an empty message. (Used for gas estimation.)
fn sig_verify_zero_bytes<M: Measurement>(g: &mut BenchmarkGroup<M>) {
    let mut csprng: ThreadRng = thread_rng();

    let priv_key = Ed25519PrivateKey::generate(&mut csprng);
    let pub_key: Ed25519PublicKey = (&priv_key).into();

    g.throughput(Throughput::Elements(1));
    g.bench_function("sig_verify_zero_bytes", move |b| {
        b.iter_with_setup(
            || {
                // Just want to ensure a different signature each time, so signing a random message each time
                let msg = random_message(&mut csprng);
                priv_key.sign(&msg).unwrap()
            },
            |sig| sig.verify_arbitrary_msg(b"", &pub_key),
        )
    });
}

/// Benchmarks the time to check if an EdwardsPoint is in a small subgroup.
fn small_subgroup_check<M: Measurement>(g: &mut BenchmarkGroup<M>) {
    let point = ED25519_BASEPOINT_POINT;
    let mut csprng = thread_rng();

    g.throughput(Throughput::Elements(1_u64));
    g.bench_function("small_subgroup_check", move |b| {
        b.iter_with_setup(
            || Scalar::random(&mut csprng) * point,
            |h| h.is_small_order(),
        )
    });
}

/// Benchmarks the time to deserialize an Ed25519 public key from a sequence of bytes.
fn pk_deserialize<M: Measurement>(g: &mut BenchmarkGroup<M>) {
    let mut csprng = thread_rng();

    g.throughput(Throughput::Elements(1_u64));
    g.bench_function("pk_deserialize", move |b| {
        b.iter_with_setup(
            || {
                Ed25519PrivateKey::generate(&mut csprng)
                    .public_key()
                    .to_bytes()
            },
            |pk_bytes| Ed25519PublicKey::try_from(&pk_bytes[..]),
        )
    });
}

/// Benchmarks the time to deserialize an Ed25519 signature from a sequence of bytes.
fn sig_deserialize<M: Measurement>(g: &mut BenchmarkGroup<M>) {
    let mut csprng = thread_rng();

    g.throughput(Throughput::Elements(1_u64));
    g.bench_function("sig_deserialize", move |b| {
        b.iter_with_setup(
            || {
                Ed25519PrivateKey::generate(&mut csprng)
                    .sign(&TestVelorCrypto("Hello Velor!".to_string()))
                    .unwrap()
                    .to_bytes()
            },
            |sig_bytes| Ed25519Signature::try_from(&sig_bytes[..]),
        )
    });
}

criterion_group!(ed25519_benches, benchmark_groups);
criterion_main!(ed25519_benches);
