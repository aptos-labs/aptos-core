// Copyright © Aptos Foundation
// Parts of the project are originally copyright © Meta Platforms, Inc.
// SPDX-License-Identifier: Apache-2.0

#[macro_use]
extern crate criterion;

use aptos_crypto::{
    ed25519::{Ed25519PrivateKey},
    traits::{Signature, SigningKey, Uniform},
};
use aptos_crypto_derive::{BCSCryptoHash, CryptoHasher};
use criterion::{measurement::Measurement, BenchmarkGroup, Criterion, Throughput};
use rand::{distributions, prelude::ThreadRng, thread_rng, Rng};
use serde::{Deserialize, Serialize};
use aptos_crypto::multi_ed25519::{MultiEd25519PrivateKey, MultiEd25519PublicKey};

#[derive(Debug, CryptoHasher, BCSCryptoHash, Serialize, Deserialize)]
pub struct TestAptosCrypto(pub String);

fn random_message(rng: &mut ThreadRng) -> TestAptosCrypto {
    TestAptosCrypto(
        rng.sample_iter(&distributions::Alphanumeric)
            .take(256)
            .map(char::from)
            .collect::<String>(),
    )
}

fn benchmark_groups(c: &mut Criterion) {
    let mut group = c.benchmark_group("multi_ed25519");

    sig_verify_struct(&mut group, 1024, 2048);

    group.finish();
}

fn sig_verify_struct<M: Measurement>(g: &mut BenchmarkGroup<M>, t: usize, n: usize) {
    let mut csprng: ThreadRng = thread_rng();

    let sks = (0..n)
        .map(|_| Ed25519PrivateKey::generate(&mut csprng))
        .collect::<Vec<Ed25519PrivateKey>>();

    //let pks = sks.iter().map(|x| x.public_key()).collect();

    let sk = MultiEd25519PrivateKey::new(sks, t).unwrap();
    let pk = MultiEd25519PublicKey::from(&sk);

    g.throughput(Throughput::Elements(t as u64));
    g.bench_function(format!("verify/{t}-out-of-{n}"), move |b| {
        b.iter_with_setup(
            || {
                let msg = random_message(&mut csprng);
                let sig = sk.sign(&msg).unwrap();
                (sig, msg)
            },
            |(sig, msg)|
                sig.verify(&msg, &pk),
        )
    });
}

criterion_group!(multi_ed25519_benches, benchmark_groups);
criterion_main!(multi_ed25519_benches);
