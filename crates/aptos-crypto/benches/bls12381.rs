// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

#[macro_use]
extern crate criterion;

use aptos_crypto::{
    bls12381,
    bls12381::ProofOfPossession,
    test_utils::{random_keypairs, random_subset, KeyPair},
    traits::{Signature, SigningKey, Uniform},
    PrivateKey,
};
use aptos_crypto_derive::{BCSCryptoHash, CryptoHasher};
use criterion::{
    measurement::Measurement, AxisScale, BatchSize, BenchmarkGroup, BenchmarkId, Criterion,
    PlotConfiguration, Throughput,
};
use rand::{distributions, rngs::ThreadRng, thread_rng, Rng};
use serde::{Deserialize, Serialize};

#[derive(Debug, CryptoHasher, BCSCryptoHash, Serialize, Deserialize)]
struct TestAptosCrypto(String);

fn random_message(rng: &mut ThreadRng) -> TestAptosCrypto {
    TestAptosCrypto(
        rng.sample_iter(&distributions::Alphanumeric)
            .take(256)
            .collect::<String>(),
    )
}

fn bench_group(c: &mut Criterion) {
    let mut group = c.benchmark_group("bls12381");

    let plot_config = PlotConfiguration::default().summary_scale(AxisScale::Logarithmic);

    group.sample_size(1000);
    group.plot_config(plot_config);

    pk_deserialize(&mut group);
    sig_deserialize(&mut group);
    pk_subgroup_membership(&mut group);
    sig_subgroup_membership(&mut group);
    aggregate_one_sigshare(&mut group);
    aggregate_one_pk(&mut group);

    pop_create(&mut group);
    pop_create_with_pubkey(&mut group);
    pop_verify(&mut group);

    sign(&mut group);
    verify_signature_share(&mut group);

    let mut size = 128;
    for _ in 1..=4 {
        // Even single-threaded, this function has higher throughput that `aggregate_one_sigshare`
        aggregate_sigshare(&mut group, size);

        // Even single-threaded, this function has higher throughput than `aggregate_one_pk`. Seems
        // to be due to only making a single call to blst::PublicKey::from_aggregate (which calls a
        // $pk_to_aff function) for the entire batch.
        aggregate_pks(&mut group, size);

        verify_multisig(&mut group, size);
        verify_aggsig(&mut group, size);
        size *= 2;
    }

    group.finish();
}

/// Benchmarks the time to deserialize a BLS12-381 point representing a PK in G1. (Does not test for
/// prime-order subgroup membership.)
fn pk_deserialize<M: Measurement>(g: &mut BenchmarkGroup<M>) {
    let mut rng = thread_rng();

    g.throughput(Throughput::Elements(1));

    g.bench_function("pk_deserialize", move |b| {
        b.iter_with_setup(
            || {
                bls12381::PrivateKey::generate(&mut rng)
                    .public_key()
                    .to_bytes()
            },
            |pk_bytes| bls12381::PublicKey::try_from(&pk_bytes[..]),
        )
    });
}

/// Benchmarks the time to aggregate a BLS PK in G1. (Does not test for prime-order subgroup
/// membership.)
fn aggregate_one_pk<M: Measurement>(g: &mut BenchmarkGroup<M>) {
    let mut rng = thread_rng();

    g.throughput(Throughput::Elements(1));

    g.bench_function("aggregate_one_pk", move |b| {
        b.iter_with_setup(
            || {
                (
                    bls12381::PrivateKey::generate(&mut rng).public_key(),
                    bls12381::PrivateKey::generate(&mut rng).public_key(),
                )
            },
            |(pk1, pk2)| {
                bls12381::PublicKey::aggregate(vec![&pk1, &pk2]).unwrap();
            },
        )
    });
}

/// Benchmarks the time to deserialize a BLS12-381 point representing a signature in G2. (Does not test for
/// prime-order subgroup membership.)
fn sig_deserialize<M: Measurement>(g: &mut BenchmarkGroup<M>) {
    let mut rng = thread_rng();

    g.throughput(Throughput::Elements(1));

    g.bench_function("sig_deserialize", move |b| {
        b.iter_with_setup(
            || {
                let sk = bls12381::PrivateKey::generate(&mut rng);
                sk.sign(&TestAptosCrypto("Hello Aptos!".to_owned()))
                    .unwrap()
                    .to_bytes()
            },
            |sig_bytes| bls12381::Signature::try_from(&sig_bytes[..]),
        )
    });
}

/// Benchmarks the time to aggregate a BLS signature in G2. (Does not test for prime-order subgroup
/// membership.)
fn aggregate_one_sigshare<M: Measurement>(g: &mut BenchmarkGroup<M>) {
    let mut rng = thread_rng();

    g.throughput(Throughput::Elements(1));

    g.bench_function("aggregate_one_sigshare", move |b| {
        b.iter_with_setup(
            || {
                (
                    bls12381::PrivateKey::generate(&mut rng)
                        .sign(&TestAptosCrypto("Hello Aptos!".to_owned()))
                        .unwrap(),
                    bls12381::PrivateKey::generate(&mut rng)
                        .sign(&TestAptosCrypto("Hello Aptos!".to_owned()))
                        .unwrap(),
                )
            },
            |(sig1, sig2)| {
                bls12381::Signature::aggregate(vec![sig1, sig2]).unwrap();
            },
        )
    });
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

criterion_group!(
    name = bls12381_benches;
    config = Criterion::default(); //.measurement_time(Duration::from_secs(100));
    targets = bench_group);
criterion_main!(bls12381_benches);
