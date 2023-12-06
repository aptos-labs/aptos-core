// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use aptos_consensus_types::{
    common::{Author, Round},
    randomness::{RandMetadata, Randomness},
};
use aptos_crypto::bls12381::Signature;
use aptos_crypto_derive::{BCSCryptoHash, CryptoHasher};
use aptos_types::{aggregate_signature::AggregateSignature, validator_verifier::ValidatorVerifier};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub(super) struct MockShare;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub(super) struct MockProof;

impl Share for MockShare {
    fn verify(
        &self,
        _rand_config: &RandConfig,
        _rand_metadata: &RandMetadata,
    ) -> anyhow::Result<()> {
        Ok(())
    }
}

impl Proof for MockProof {
    type Share = MockShare;

    fn verify(
        &self,
        _rand_config: &RandConfig,
        _rand_metadata: &RandMetadata,
    ) -> anyhow::Result<()> {
        Ok(())
    }

    fn aggregate<'a>(
        _shares: impl Iterator<Item = &'a RandShare<Self::Share>>,
        _rand_config: &RandConfig,
        rand_metadata: RandMetadata,
    ) -> RandDecision<Self> {
        RandDecision::new(Randomness::new(rand_metadata, vec![0u8; 32]), Self)
    }
}

pub trait Share: Clone + Send + Sync {
    fn verify(&self, rand_config: &RandConfig, rand_metadata: &RandMetadata) -> anyhow::Result<()>;
}

pub trait Proof: Clone + Send + Sync {
    type Share: 'static;
    fn verify(&self, rand_config: &RandConfig, rand_metadata: &RandMetadata) -> anyhow::Result<()>;

    fn aggregate<'a>(
        shares: impl Iterator<Item = &'a RandShare<Self::Share>>,
        rand_config: &RandConfig,
        rand_metadata: RandMetadata,
    ) -> RandDecision<Self>
    where
        Self: Sized;
}

pub trait AugmentedData: Clone + Send + Sync + Serialize {}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct RandShare<S> {
    author: Author,
    metadata: RandMetadata,
    share: S,
}

impl<S: Share> RandShare<S> {
    pub fn new(author: Author, metadata: RandMetadata, share: S) -> Self {
        Self {
            author,
            metadata,
            share,
        }
    }

    pub fn author(&self) -> &Author {
        &self.author
    }

    pub fn metadata(&self) -> &RandMetadata {
        &self.metadata
    }

    pub fn round(&self) -> Round {
        self.metadata.round()
    }

    pub fn epoch(&self) -> u64 {
        self.metadata.epoch()
    }

    pub fn verify(&self, rand_config: &RandConfig) -> anyhow::Result<()> {
        self.share.verify(rand_config, &self.metadata)
    }
}

#[derive(Clone, Serialize, Deserialize)]
pub struct RandDecision<P> {
    randomness: Randomness,
    proof: P,
}

impl<P: Proof> RandDecision<P> {
    pub fn new(randomness: Randomness, proof: P) -> Self {
        Self { randomness, proof }
    }

    pub fn verify(&self, rand_config: &RandConfig) -> anyhow::Result<()> {
        self.proof.verify(rand_config, self.randomness.metadata())
    }

    pub fn randomness(&self) -> &Randomness {
        &self.randomness
    }

    pub fn rand_metadata(&self) -> &RandMetadata {
        self.randomness.metadata()
    }
}

#[derive(Clone, Serialize, Deserialize)]
pub struct ShareAck<P> {
    epoch: u64,
    maybe_decision: Option<RandDecision<P>>,
}

impl<P> ShareAck<P> {
    pub fn new(epoch: u64, maybe_decision: Option<RandDecision<P>>) -> Self {
        Self {
            epoch,
            maybe_decision,
        }
    }

    pub fn into_maybe_decision(self) -> Option<RandDecision<P>> {
        self.maybe_decision
    }
}

#[derive(Clone, Serialize, Deserialize, CryptoHasher, BCSCryptoHash)]
pub struct AugData<D> {
    epoch: u64,
    author: Author,
    data: D,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct AugDataSignature {
    signature: Signature,
}

impl AugDataSignature {
    pub fn new(signature: Signature) -> Self {
        Self { signature }
    }

    pub fn verify<D: AugmentedData>(
        &self,
        author: Author,
        verifier: &ValidatorVerifier,
        data: &AugData<D>,
    ) -> anyhow::Result<()> {
        Ok(verifier.verify(author, data, &self.signature)?)
    }

    pub fn into_signature(self) -> Signature {
        self.signature
    }
}

#[derive(Clone, Serialize, Deserialize)]
pub struct CertifiedAugData<D> {
    aug_data: AugData<D>,
    signatures: AggregateSignature,
}

impl<D> CertifiedAugData<D> {
    pub fn new(aug_data: AugData<D>, signatures: AggregateSignature) -> Self {
        Self {
            aug_data,
            signatures,
        }
    }
}

#[derive(Clone, Serialize, Deserialize)]
pub struct CertifiedAugDataAck {
    epoch: u64,
}

pub struct RandConfig {
    author: Author,
    threshold: u64,
    weights: HashMap<Author, u64>,
}

impl RandConfig {
    pub fn new(author: Author, weights: HashMap<Author, u64>) -> Self {
        let sum = weights.values().sum::<u64>();
        Self {
            author,
            weights,
            threshold: sum * 2 / 3 + 1,
        }
    }

    pub fn get_peer_weight(&self, author: &Author) -> u64 {
        *self
            .weights
            .get(author)
            .expect("Author should exist after verify")
    }

    pub fn threshold_weight(&self) -> u64 {
        self.threshold
    }
}
