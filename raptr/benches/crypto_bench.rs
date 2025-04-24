// Copyright (c) Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use aptos_crypto::{bls12381, PrivateKey, SigningKey, Uniform};
use aptos_crypto_derive::{BCSCryptoHash, CryptoHasher};
use aptos_types::{validator_signer::ValidatorSigner, validator_verifier::ValidatorVerifier};
use criterion::{criterion_group, criterion_main, Criterion};
use raikou::framework::crypto::{SignatureVerifier, Signer};
use rand::{rngs::StdRng, SeedableRng};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

/// Helper: Create a deterministic private key for testing.
fn deterministic_main_private_key(node_id: usize) -> bls12381::PrivateKey {
    let mut seed = [0u8; 32];
    seed[..8].copy_from_slice(&node_id.to_le_bytes());
    let mut rng = StdRng::from_seed(seed);
    bls12381::PrivateKey::generate(&mut rng)
}

/// Helper: Setup TOTAL_NODES deterministic private keys and corresponding public keys.
fn setup_deterministic_keys(
    total_nodes: usize,
) -> (Vec<bls12381::PrivateKey>, Vec<bls12381::PublicKey>) {
    let mut priv_keys = Vec::with_capacity(total_nodes);
    let mut pub_keys = Vec::with_capacity(total_nodes);
    for i in 0..total_nodes {
        let pk = deterministic_main_private_key(i);
        pub_keys.push(pk.public_key());
        priv_keys.push(pk);
    }
    (priv_keys, pub_keys)
}

/// Message for the aggregate signature benchmarks.
/// The tag is embedded as part of the message.
#[derive(CryptoHasher, BCSCryptoHash, Serialize, Deserialize, Debug, PartialEq)]
struct AggMessage {
    base: u64,
    tag: u8,
}

/// Message for the multi-signature benchmarks (tagged or normal).
/// The message is identical across nodes.
#[derive(CryptoHasher, BCSCryptoHash, Serialize, Deserialize, Debug, PartialEq)]
struct BaseMessage {
    base: u64,
}

// ---------------------------------------------------------------------
// Benchmark 1: Aggregate Signatures (75/100, small tags) - Verification Only
// (Each node embeds a tag: i mod 9)
fn bench_aggregate_signatures(c: &mut Criterion) {
    const TOTAL_NODES: usize = 100;
    const PARTICIPANTS: usize = 75;

    // Setup deterministic keys.
    let (_priv_keys, public_keys) = setup_deterministic_keys(TOTAL_NODES);

    // Create messages with a small tag (i mod 9).
    let mut messages = Vec::with_capacity(PARTICIPANTS);
    for i in 0..PARTICIPANTS {
        messages.push(AggMessage {
            base: 42,
            tag: (i % 9) as u8,
        });
    }

    // Each participating node signs its corresponding message.
    let mut signatures = Vec::with_capacity(PARTICIPANTS);
    let (priv_keys, _) = setup_deterministic_keys(TOTAL_NODES);
    for i in 0..PARTICIPANTS {
        let sig = priv_keys[i]
            .sign(&messages[i])
            .expect("failed to sign aggregate message");
        signatures.push(sig);
    }

    let participating_nodes: Vec<usize> = (0..PARTICIPANTS).collect();
    let dummy_verifier = Arc::new(ValidatorVerifier::new(vec![]));
    let signature_verifier = SignatureVerifier::new(public_keys.clone(), dummy_verifier, 1);
    let message_refs: Vec<&AggMessage> = messages.iter().collect();

    // Pre-compute aggregated signature.
    let aggregated_sig = signature_verifier
        .aggregate_signatures(signatures.clone())
        .expect("aggregation failed");

    c.bench_function(
        "Aggregate Signatures (75/100, small tags) - Verification",
        |b| {
            b.iter(|| {
                signature_verifier
                    .verify_aggregate_signatures(
                        participating_nodes.clone(),
                        message_refs.clone(),
                        &aggregated_sig,
                    )
                    .expect("verification failed");
            })
        },
    );
}

// ---------------------------------------------------------------------
// Benchmark 2: Aggregate Signatures (75/100, unique tags) - Verification Only
// (Each node embeds its unique tag: i)
fn bench_aggregate_signatures_unique_tags(c: &mut Criterion) {
    const TOTAL_NODES: usize = 100;
    const PARTICIPANTS: usize = 75;

    // Setup deterministic keys.
    let (_priv_keys, public_keys) = setup_deterministic_keys(TOTAL_NODES);

    // Create messages with unique tags.
    let mut messages = Vec::with_capacity(PARTICIPANTS);
    for i in 0..PARTICIPANTS {
        messages.push(AggMessage {
            base: 42,
            tag: i as u8,
        });
    }

    // Each participating node signs its corresponding message.
    let mut signatures = Vec::with_capacity(PARTICIPANTS);
    let (priv_keys, _) = setup_deterministic_keys(TOTAL_NODES);
    for i in 0..PARTICIPANTS {
        let sig = priv_keys[i]
            .sign(&messages[i])
            .expect("failed to sign aggregate message");
        signatures.push(sig);
    }

    let participating_nodes: Vec<usize> = (0..PARTICIPANTS).collect();
    let dummy_verifier = Arc::new(ValidatorVerifier::new(vec![]));
    let signature_verifier = SignatureVerifier::new(public_keys.clone(), dummy_verifier, 1);
    let message_refs: Vec<&AggMessage> = messages.iter().collect();

    // Pre-compute aggregated signature.
    let aggregated_sig = signature_verifier
        .aggregate_signatures(signatures.clone())
        .expect("aggregation failed");

    c.bench_function(
        "Aggregate Signatures (75/100, unique tags) - Verification",
        |b| {
            b.iter(|| {
                signature_verifier
                    .verify_aggregate_signatures(
                        participating_nodes.clone(),
                        message_refs.clone(),
                        &aggregated_sig,
                    )
                    .expect("verification failed");
            })
        },
    );
}

// ---------------------------------------------------------------------
// Benchmark 3: Tagged Multi-Signatures (75/100, small tags) - Verification Only
// (Each node uses a tag key chosen as i mod 9.)
fn bench_tagged_multi_signatures(c: &mut Criterion) {
    const TOTAL_NODES: usize = 100;
    const PARTICIPANTS: usize = 75;
    const N_TAGS: usize = 9; // small tag space: 0 to 8

    // Create 100 signers with a small tag set.
    let mut signers = Vec::with_capacity(TOTAL_NODES);
    let mut public_keys = Vec::with_capacity(TOTAL_NODES);
    for i in 0..TOTAL_NODES {
        let mut seed = [0u8; 32];
        seed[..8].copy_from_slice(&i.to_le_bytes());
        let vs = ValidatorSigner::random(seed);
        public_keys.push(vs.public_key());
        signers.push(Signer::new(Arc::new(vs), i, N_TAGS));
    }

    // The base message is identical for all nodes.
    let msg = BaseMessage { base: 42 };

    let participating_nodes: Vec<usize> = (0..PARTICIPANTS).collect();
    let tags: Vec<usize> = participating_nodes.iter().map(|&i| i % N_TAGS).collect();

    let mut signatures = Vec::with_capacity(PARTICIPANTS);
    for &i in &participating_nodes {
        let tag = i % N_TAGS;
        let sig = signers[i]
            .sign_tagged(&msg, tag)
            .expect("failed to sign tagged message");
        signatures.push(sig);
    }

    let dummy_verifier = Arc::new(ValidatorVerifier::new(vec![]));
    let signature_verifier = SignatureVerifier::new(public_keys.clone(), dummy_verifier, N_TAGS);

    // Pre-compute aggregated signature.
    let aggregated_sig = signature_verifier
        .aggregate_signatures(signatures.clone())
        .expect("aggregation failed");

    c.bench_function(
        "Tagged Multi-Signatures (75/100, small tags) - Verification",
        |b| {
            b.iter(|| {
                signature_verifier
                    .verify_tagged_multi_signature(
                        participating_nodes.clone(),
                        &msg,
                        tags.clone(),
                        &aggregated_sig,
                    )
                    .expect("verification failed");
            })
        },
    );
}

// ---------------------------------------------------------------------
// Benchmark 4: Tagged Multi-Signatures (75/100, unique tags) - Verification Only
// (Each node uses its unique tag key; here N_TAGS equals TOTAL_NODES.)
fn bench_tagged_multi_signatures_unique(c: &mut Criterion) {
    const TOTAL_NODES: usize = 100;
    const PARTICIPANTS: usize = 75;
    const N_TAGS: usize = TOTAL_NODES; // allow unique tag per node

    // Create 100 signers with a larger tag space.
    let mut signers = Vec::with_capacity(TOTAL_NODES);
    let mut public_keys = Vec::with_capacity(TOTAL_NODES);
    for i in 0..TOTAL_NODES {
        let mut seed = [0u8; 32];
        seed[..8].copy_from_slice(&i.to_le_bytes());
        let vs = ValidatorSigner::random(seed);
        public_keys.push(vs.public_key());
        signers.push(Signer::new(Arc::new(vs), i, N_TAGS));
    }

    let msg = BaseMessage { base: 42 };

    let participating_nodes: Vec<usize> = (0..PARTICIPANTS).collect();
    // Use each node's index as its unique tag.
    let tags: Vec<usize> = participating_nodes.iter().map(|&i| i).collect();

    let mut signatures = Vec::with_capacity(PARTICIPANTS);
    for &i in &participating_nodes {
        let sig = signers[i]
            .sign_tagged(&msg, i)
            .expect("failed to sign tagged message");
        signatures.push(sig);
    }

    let dummy_verifier = Arc::new(ValidatorVerifier::new(vec![]));
    let signature_verifier = SignatureVerifier::new(public_keys.clone(), dummy_verifier, N_TAGS);

    // Pre-compute aggregated signature.
    let aggregated_sig = signature_verifier
        .aggregate_signatures(signatures.clone())
        .expect("aggregation failed");

    c.bench_function(
        "Tagged Multi-Signatures (75/100, unique tags) - Verification",
        |b| {
            b.iter(|| {
                signature_verifier
                    .verify_tagged_multi_signature(
                        participating_nodes.clone(),
                        &msg,
                        tags.clone(),
                        &aggregated_sig,
                    )
                    .expect("verification failed");
            })
        },
    );
}

// ---------------------------------------------------------------------
// Benchmark 5: Normal Multi-Signatures (75/100, no tags) - Verification Only
// (Each participating node signs the same message using its normal key.)
fn bench_multi_signatures(c: &mut Criterion) {
    const TOTAL_NODES: usize = 100;
    const PARTICIPANTS: usize = 75;

    let (priv_keys, public_keys) = setup_deterministic_keys(TOTAL_NODES);

    // All participating nodes sign the same message.
    let msg = BaseMessage { base: 42 };

    let mut signatures = Vec::with_capacity(PARTICIPANTS);
    for i in 0..PARTICIPANTS {
        let sig = priv_keys[i]
            .sign(&msg)
            .expect("failed to sign multi message");
        signatures.push(sig);
    }

    let participating_nodes: Vec<usize> = (0..PARTICIPANTS).collect();
    let dummy_verifier = Arc::new(ValidatorVerifier::new(vec![]));
    let signature_verifier = SignatureVerifier::new(public_keys.clone(), dummy_verifier, 1);

    // Pre-compute aggregated signature.
    let aggregated_sig = signature_verifier
        .aggregate_signatures(signatures.clone())
        .expect("aggregation failed");

    c.bench_function("Multi-Signatures (75/100, no tags) - Verification", |b| {
        b.iter(|| {
            signature_verifier
                .verify_multi_signature(participating_nodes.clone(), &msg, &aggregated_sig)
                .expect("verification failed");
        })
    });
}

criterion_group!(
    benches,
    bench_aggregate_signatures,
    bench_aggregate_signatures_unique_tags,
    bench_tagged_multi_signatures,
    bench_tagged_multi_signatures_unique,
    bench_multi_signatures
);
criterion_main!(benches);
