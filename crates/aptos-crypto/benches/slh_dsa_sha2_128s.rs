// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

#[macro_use]
extern crate criterion;

use aptos_crypto::{
    slh_dsa_sha2_128s::{PrivateKey, PublicKey, Signature},
    traits::{Signature as SignatureTrait, SigningKey, Uniform},
    PrivateKey as PrivateKeyTrait,
};
use aptos_crypto_derive::{BCSCryptoHash, CryptoHasher};
use criterion::{measurement::Measurement, BenchmarkGroup, Criterion, Throughput};
use rand::{thread_rng, RngCore};
use serde::{Deserialize, Serialize};

#[derive(Debug, CryptoHasher, BCSCryptoHash, Serialize, Deserialize)]
pub struct Message32Bytes([u8; 32]);

impl Message32Bytes {
    fn new(bytes: [u8; 32]) -> Self {
        Message32Bytes(bytes)
    }
}

fn generate_random_message(csprng: &mut impl RngCore) -> Message32Bytes {
    let mut msg_bytes = [0u8; 32];
    csprng.fill_bytes(&mut msg_bytes);
    Message32Bytes::new(msg_bytes)
}

fn benchmark_groups(c: &mut Criterion) {
    let mut group = c.benchmark_group("slh_dsa/sha2-128s");

    group.sample_size(10);

    sig_deserialize(&mut group);
    pk_deserialize(&mut group);
    sign_32_bytes(&mut group);
    verify_32_bytes(&mut group);

    group.finish();
}

/// Benchmarks the time to deserialize an SLH-DSA signature from a sequence of bytes.
fn sig_deserialize<M: Measurement>(g: &mut BenchmarkGroup<M>) {
    let mut csprng = thread_rng();

    g.throughput(Throughput::Elements(1_u64));
    g.bench_function("sig_deserialize", move |b| {
        b.iter_with_setup(
            || {
                let priv_key = PrivateKey::generate(&mut csprng);
                let msg = generate_random_message(&mut csprng);
                priv_key.sign(&msg).unwrap().to_bytes()
            },
            |sig_bytes| Signature::try_from(&sig_bytes[..]),
        )
    });
}

/// Benchmarks the time to deserialize an SLH-DSA public key from a sequence of bytes.
fn pk_deserialize<M: Measurement>(g: &mut BenchmarkGroup<M>) {
    let mut csprng = thread_rng();

    g.throughput(Throughput::Elements(1_u64));
    g.bench_function("pk_deserialize", move |b| {
        b.iter_with_setup(
            || PrivateKey::generate(&mut csprng).public_key().to_bytes(),
            |pk_bytes| PublicKey::try_from(&pk_bytes[..]),
        )
    });
}

/// Benchmarks the time to sign a 32-byte random message.
fn sign_32_bytes<M: Measurement>(g: &mut BenchmarkGroup<M>) {
    let mut csprng = thread_rng();

    let priv_key = PrivateKey::generate(&mut csprng);

    g.throughput(Throughput::Elements(1_u64));
    g.bench_function("sign_32_bytes", move |b| {
        b.iter_with_setup(
            || {
                // Generate a random 32-byte message
                generate_random_message(&mut csprng)
            },
            |msg| priv_key.sign(&msg).unwrap(),
        )
    });
}

/// Benchmarks the time to verify a signature on a 32-byte random message.
fn verify_32_bytes<M: Measurement>(g: &mut BenchmarkGroup<M>) {
    let mut csprng = thread_rng();

    let priv_key = PrivateKey::generate(&mut csprng);
    let pub_key: PublicKey = (&priv_key).into();

    g.throughput(Throughput::Elements(1_u64));
    g.bench_function("verify_32_bytes", move |b| {
        b.iter_with_setup(
            || {
                // Generate a random 32-byte message
                let msg = generate_random_message(&mut csprng);
                let sig = priv_key.sign(&msg).unwrap();
                (sig, msg)
            },
            |(sig, msg)| sig.verify(&msg, &pub_key),
        )
    });
}

criterion_group!(slh_dsa_benches, benchmark_groups);
criterion_main!(slh_dsa_benches);
