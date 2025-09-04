// Copyright © Velor Foundation
// SPDX-License-Identifier: Apache-2.0

#[macro_use]
extern crate criterion;

use velor_crypto::test_utils::random_bytes;
use criterion::{measurement::Measurement, BenchmarkGroup, Criterion, Throughput};
use rand::{prelude::ThreadRng, thread_rng};

fn benchmark_groups(c: &mut Criterion) {
    let mut group = c.benchmark_group("secp256k1");

    group.sample_size(1000);

    ecdsa_recover(&mut group);

    group.finish();
}

/// Benchmarks the time to verify a signature. (Used for gas estimation.)
fn ecdsa_recover<M: Measurement>(g: &mut BenchmarkGroup<M>) {
    let mut csprng: ThreadRng = thread_rng();

    let sk_bytes = random_bytes(&mut csprng, 32);
    let secret_key = libsecp256k1::SecretKey::parse_slice(&sk_bytes[..]).unwrap();
    let pub_key = libsecp256k1::PublicKey::from_secret_key(&secret_key);

    g.throughput(Throughput::Elements(1));
    g.bench_function("ecdsa_recover", move |b| {
        b.iter_with_setup(
            || {
                let bytes = random_bytes(&mut csprng, 32);
                let msg = libsecp256k1::Message::parse_slice(&bytes[..]).unwrap();
                let sig = libsecp256k1::sign(&msg, &secret_key);
                (sig, msg)
            },
            |((sig, recovery_id), msg)| {
                let pk = libsecp256k1::recover(&msg, &sig, &recovery_id).unwrap();
                assert_eq!(pk.serialize(), pub_key.serialize());
            },
        )
    });
}

criterion_group!(secp256k1_benches, benchmark_groups);
criterion_main!(secp256k1_benches);
