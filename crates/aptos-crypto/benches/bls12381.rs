// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

#[macro_use]
extern crate criterion;

use criterion::{
    measurement::Measurement, BatchSize, BenchmarkGroup, BenchmarkId, Criterion, Throughput,
};
use rand::{distributions::Alphanumeric, rngs::ThreadRng, thread_rng, Rng};

use aptos_crypto_derive::{BCSCryptoHash, CryptoHasher};
use serde::{Deserialize, Serialize};

use aptos_crypto::{
    bls12381,
    bls12381::ProofOfPossession,
    test_utils::{random_keypairs, random_subset, KeyPair},
    traits::{Signature, SigningKey, Uniform},
};

#[derive(Debug, CryptoHasher, BCSCryptoHash, Serialize, Deserialize)]
struct TestAptosCrypto(String);

fn random_message(rng: &mut ThreadRng) -> TestAptosCrypto {
    TestAptosCrypto(
        rng.sample_iter(&Alphanumeric)
            .take(256)
            .map(char::from)
            .collect::<String>(),
    )
}

fn bench_group(c: &mut Criterion) {
    let mut group = c.benchmark_group("bls12381");

    pk_subgroup_membership(&mut group);
    sig_subgroup_membership(&mut group);

    pop_create(&mut group);
    pop_create_with_pubkey(&mut group);
    pop_verify(&mut group);

    sign(&mut group);
    verify_signature_share(&mut group);

    let mut size = 128;
    for _ in 1..=4 {
        aggregate_sigshare(&mut group, size);
        aggregate_pks(&mut group, size);
        verify_multisig(&mut group, size);
        verify_aggsig(&mut group, size);
        size *= 2;
    }

    group.finish();
}

fn pk_subgroup_membership<M: Measurement>(g: &mut BenchmarkGroup<M>) {
    let mut rng = thread_rng();

    g.throughput(Throughput::Elements(1));
    g.bench_function("pk_prime_order_subgroup_check", move |b| {
        b.iter_with_setup(
            || {
                let kp = KeyPair::<bls12381::PrivateKey, bls12381::PublicKey>::generate(&mut rng);
                kp.public_key
            },
            |pk| pk.subgroup_check(),
        )
    });
}

fn sig_subgroup_membership<M: Measurement>(g: &mut BenchmarkGroup<M>) {
    let mut rng = thread_rng();

    g.throughput(Throughput::Elements(1));
    g.bench_function("sig_prime_order_subgroup_check", move |b| {
        b.iter_with_setup(
            || {
                let kp = KeyPair::<bls12381::PrivateKey, bls12381::PublicKey>::generate(&mut rng);

                // Currently, there's no better way of sampling a group element here
                kp.private_key
                    .sign(&TestAptosCrypto("Hello Aptos!".to_owned()))
                    .unwrap()
            },
            |sig| sig.subgroup_check(),
        )
    });
}

fn pop_create<M: Measurement>(g: &mut BenchmarkGroup<M>) {
    let mut rng = thread_rng();

    let priv_key = bls12381::PrivateKey::generate(&mut rng);

    g.throughput(Throughput::Elements(1));
    g.bench_function("pop_create", move |b| {
        b.iter(|| ProofOfPossession::create(&priv_key))
    });
}

fn pop_create_with_pubkey<M: Measurement>(g: &mut BenchmarkGroup<M>) {
    let mut rng = thread_rng();

    let kp = KeyPair::<bls12381::PrivateKey, bls12381::PublicKey>::generate(&mut rng);

    g.throughput(Throughput::Elements(1));
    g.bench_function("pop_create_with_pubkey", move |b| {
        b.iter(|| ProofOfPossession::create_with_pubkey(&kp.private_key, &kp.public_key))
    });
}

fn pop_verify<M: Measurement>(g: &mut BenchmarkGroup<M>) {
    let mut rng = thread_rng();

    let kp = KeyPair::<bls12381::PrivateKey, bls12381::PublicKey>::generate(&mut rng);
    let pop = ProofOfPossession::create_with_pubkey(&kp.private_key, &kp.public_key);

    g.throughput(Throughput::Elements(1));
    g.bench_function("pop_verify", move |b| {
        b.iter(|| {
            let result = pop.verify(&kp.public_key);
            assert!(result.is_ok());
        })
    });
}

fn sign<M: Measurement>(g: &mut BenchmarkGroup<M>) {
    let mut rng = thread_rng();

    let priv_key = bls12381::PrivateKey::generate(&mut rng);
    let msg = random_message(&mut rng);

    g.throughput(Throughput::Elements(1));
    g.bench_function("sign", move |b| b.iter(|| priv_key.sign(&msg).unwrap()));
}

fn verify_signature_share<M: Measurement>(g: &mut BenchmarkGroup<M>) {
    let mut rng = thread_rng();

    let keypair = KeyPair::<bls12381::PrivateKey, bls12381::PublicKey>::generate(&mut rng);
    let msg = random_message(&mut rng);
    let sig = keypair.private_key.sign(&msg).unwrap();

    g.throughput(Throughput::Elements(1));
    g.bench_function("verify_signature_share", move |b| {
        b.iter(|| {
            let result = sig.verify(&msg, &keypair.public_key);
            assert!(result.is_ok());
        })
    });
}

fn aggregate_pks<M: Measurement>(g: &mut BenchmarkGroup<M>, size: usize) {
    let mut rng = thread_rng();

    // pick a bunch of random keypairs
    let key_pairs: Vec<KeyPair<bls12381::PrivateKey, bls12381::PublicKey>> =
        random_keypairs(&mut rng, size);

    g.throughput(Throughput::Elements(size as u64));
    g.bench_with_input(
        BenchmarkId::new("aggregate_pks", size),
        &size,
        |b, &_size| {
            b.iter_batched(
                || {
                    let mut pks = vec![];
                    for kp in key_pairs.iter() {
                        pks.push(&kp.public_key);
                    }

                    pks
                },
                |pks| {
                    let result = bls12381::PublicKey::aggregate(pks);
                    assert!(result.is_ok());
                },
                BatchSize::SmallInput,
            );
        },
    );
}

fn aggregate_sigshare<M: Measurement>(g: &mut BenchmarkGroup<M>, size: usize) {
    let mut rng = thread_rng();

    // pick a bunch of random keypairs
    let key_pairs: Vec<KeyPair<bls12381::PrivateKey, bls12381::PublicKey>> =
        random_keypairs(&mut rng, size);

    g.throughput(Throughput::Elements(size as u64));
    g.bench_with_input(
        BenchmarkId::new("aggregate_sigshare", size),
        &size,
        |b, &_size| {
            // pick a random message to aggregate a multisignature on
            let msg = random_message(&mut rng);

            b.iter_batched(
                || {
                    // each signer computes a signature share on the random message
                    let mut sigshares = vec![];
                    for kp in key_pairs.iter() {
                        sigshares.push(kp.private_key.sign(&msg).unwrap());
                    }
                    sigshares
                },
                |sigshares| {
                    let result = bls12381::Signature::aggregate(sigshares);
                    assert!(result.is_ok());
                },
                BatchSize::SmallInput,
            );
        },
    );
}

/// Benchmarks the time to verify a multisignature from the perspective of a verifier who has the
/// public keys of `n` signers and receives a multisignature from `size` of them
fn verify_multisig<M: Measurement>(g: &mut BenchmarkGroup<M>, size: usize) {
    let mut rng = thread_rng();

    // pick `n` random keypairs
    let mut key_pairs = vec![];
    let n = size * 2;

    for _ in 0..n {
        key_pairs.push(KeyPair::<bls12381::PrivateKey, bls12381::PublicKey>::generate(&mut rng));
    }

    g.throughput(Throughput::Elements(size as u64));
    g.bench_with_input(
        BenchmarkId::new("verify_multisig", size),
        &size,
        |b, &size| {
            // pick a random message to aggregate a multisignature on
            let msg = random_message(&mut rng);

            b.iter_batched(
                || {
                    // pick a random subset of signers
                    let subset = random_subset(&mut rng, n, size);

                    // each of the selected signers computes a signature share on the random message
                    let mut sigshares = vec![];
                    let mut pks = vec![];

                    for i in subset {
                        sigshares.push(key_pairs[i].private_key.sign(&msg).unwrap());
                        pks.push(&key_pairs[i].public_key)
                    }

                    let multisig = bls12381::Signature::aggregate(sigshares).unwrap();

                    (pks, multisig)
                },
                |(pks, multisig)| {
                    let aggpk = bls12381::PublicKey::aggregate(pks).unwrap();

                    let result = multisig.verify(&msg, &aggpk);

                    assert!(result.is_ok());
                },
                BatchSize::SmallInput,
            );
        },
    );
}

/// Benchmarks the time to verify an aggregate signature from the perspective of a verifier who
/// receives an aggregate signature from `n` signers.
fn verify_aggsig<M: Measurement>(g: &mut BenchmarkGroup<M>, n: usize) {
    let mut rng = thread_rng();

    // pick `n` random keypairs
    let mut key_pairs = vec![];

    for _ in 0..n {
        key_pairs.push(KeyPair::<bls12381::PrivateKey, bls12381::PublicKey>::generate(&mut rng));
    }

    g.throughput(Throughput::Elements(n as u64));
    g.bench_with_input(BenchmarkId::new("verify_aggsig", n), &n, |b, &_n| {
        b.iter_batched(
            || {
                // each of the signers computes a signature share on a random message
                let mut sigshares = vec![];
                let mut pks = vec![];
                let mut msgs = vec![];

                for kp in key_pairs.iter() {
                    msgs.push(random_message(&mut rng));
                    sigshares.push(kp.private_key.sign(msgs.last().unwrap()).unwrap());
                    pks.push(&kp.public_key)
                }

                let aggsig = bls12381::Signature::aggregate(sigshares).unwrap();

                (msgs, pks, aggsig)
            },
            |(msgs, pks, aggsig)| {
                let msgs_refs = msgs.iter().collect::<Vec<&TestAptosCrypto>>();

                let result = aggsig.verify_aggregate(&msgs_refs, &pks);

                assert!(result.is_ok());
            },
            BatchSize::SmallInput,
        );
    });
}

criterion_group!(bls12381_benches, bench_group);
criterion_main!(bls12381_benches);
