// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

#[macro_use]
extern crate criterion;

use criterion::{measurement::Measurement, BenchmarkGroup, Criterion, Throughput};
use curve25519_dalek::{constants::ED25519_BASEPOINT_POINT, scalar::Scalar};

use aptos_crypto_derive::{BCSCryptoHash, CryptoHasher};
use rand::{prelude::ThreadRng, thread_rng};
use serde::{Deserialize, Serialize};

#[derive(Debug, CryptoHasher, BCSCryptoHash, Serialize, Deserialize)]
pub struct TestAptosCrypto(pub String);

use aptos_crypto::{
    ed25519::{Ed25519PrivateKey, Ed25519PublicKey, Ed25519Signature},
    traits::{Signature, SigningKey, Uniform},
};

fn benchmark_groups(c: &mut Criterion) {
    let mut group = c.benchmark_group("ed25519");

    verify(&mut group);
    small_subgroup_check(&mut group);

    group.finish();
}

fn verify<M: Measurement>(g: &mut BenchmarkGroup<M>) {
    let mut csprng: ThreadRng = thread_rng();

    let priv_key = Ed25519PrivateKey::generate(&mut csprng);
    let pub_key: Ed25519PublicKey = (&priv_key).into();

    let msg = TestAptosCrypto("".to_string());
    let sig: Ed25519Signature = priv_key.sign(&msg).unwrap();

    g.throughput(Throughput::Elements(1));
    g.bench_function("Ed25519 signature verification", move |b| {
        b.iter(|| sig.verify(&msg, &pub_key))
    });
}

fn small_subgroup_check<M: Measurement>(g: &mut BenchmarkGroup<M>) {
    let point = ED25519_BASEPOINT_POINT;
    let mut csprng = thread_rng();

    g.throughput(Throughput::Elements(1_u64));
    g.bench_function("EdwardsPoint small subgroup check", move |b| {
        b.iter_with_setup(
            || Scalar::random(&mut csprng) * point,
            |h| h.is_small_order(),
        )
    });
}

criterion_group!(ed25519_benches, benchmark_groups);
criterion_main!(ed25519_benches);
