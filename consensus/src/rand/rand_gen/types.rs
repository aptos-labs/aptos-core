// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use aptos_consensus_types::{
    common::{Author, Round},
    randomness::{RandMetadata, Randomness},
};
use aptos_crypto::bls12381::Signature;
use aptos_crypto_derive::{BCSCryptoHash, CryptoHasher};
use aptos_infallible::RwLock;
use aptos_types::{aggregate_signature::AggregateSignature, validator_verifier::ValidatorVerifier};
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use std::{collections::HashMap, fmt::Debug, sync::Arc};

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub(super) struct MockShare;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub(super) struct MockProof;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub(super) struct MockAugData;

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

impl AugmentedData for MockAugData {}

pub trait Share:
    Clone + Debug + PartialEq + Send + Sync + Serialize + DeserializeOwned + 'static
{
    fn verify(&self, rand_config: &RandConfig, rand_metadata: &RandMetadata) -> anyhow::Result<()>;
}

pub trait Proof:
    Clone + Debug + PartialEq + Send + Sync + Serialize + DeserializeOwned + 'static
{
    type Share: Share;
    fn verify(&self, rand_config: &RandConfig, rand_metadata: &RandMetadata) -> anyhow::Result<()>;

    fn aggregate<'a>(
        shares: impl Iterator<Item = &'a RandShare<Self::Share>>,
        rand_config: &RandConfig,
        rand_metadata: RandMetadata,
    ) -> RandDecision<Self>
    where
        Self: Sized;
}

pub trait AugmentedData:
    Clone + Debug + PartialEq + Send + Sync + Serialize + DeserializeOwned + 'static
{
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub struct ShareId {
    epoch: u64,
    round: Round,
    author: Author,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
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

    pub fn share_id(&self) -> ShareId {
        ShareId {
            epoch: self.epoch(),
            round: self.round(),
            author: self.author,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
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

    pub fn epoch(&self) -> u64 {
        self.epoch
    }

    pub fn into_maybe_decision(self) -> Option<RandDecision<P>> {
        self.maybe_decision
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, Hash, Eq)]
pub struct AugDataId {
    epoch: u64,
    author: Author,
}

impl AugDataId {
    pub fn new(epoch: u64, author: Author) -> Self {
        Self { epoch, author }
    }

    pub fn epoch(&self) -> u64 {
        self.epoch
    }

    pub fn author(&self) -> Author {
        self.author
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, CryptoHasher, BCSCryptoHash)]
pub struct AugData<D> {
    epoch: u64,
    author: Author,
    data: D,
}

impl<D> AugData<D> {
    pub fn new(epoch: u64, author: Author, data: D) -> Self {
        Self {
            epoch,
            author,
            data,
        }
    }

    pub fn epoch(&self) -> u64 {
        self.epoch
    }

    pub fn id(&self) -> AugDataId {
        AugDataId {
            epoch: self.epoch,
            author: self.author,
        }
    }
}

#[derive(Clone, Serialize, Deserialize)]
pub struct AugDataSignature {
    epoch: u64,
    signature: Signature,
}

impl AugDataSignature {
    pub fn new(epoch: u64, signature: Signature) -> Self {
        Self { epoch, signature }
    }

    pub fn epoch(&self) -> u64 {
        self.epoch
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

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
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

    pub fn epoch(&self) -> u64 {
        self.aug_data.epoch()
    }

    pub fn id(&self) -> AugDataId {
        self.aug_data.id()
    }
}

#[derive(Clone, Serialize, Deserialize)]
pub struct CertifiedAugDataAck {
    epoch: u64,
}

impl CertifiedAugDataAck {
    pub fn new(epoch: u64) -> Self {
        Self { epoch }
    }

    pub fn epoch(&self) -> u64 {
        self.epoch
    }
}

#[derive(Clone)]
pub struct RandConfig {
    author: Author,
    threshold: u64,
    weights: HashMap<Author, u64>,
    certified_data: Arc<RwLock<HashMap<Author, Vec<u8>>>>,
}

impl RandConfig {
    pub fn new(author: Author, weights: HashMap<Author, u64>) -> Self {
        let sum = weights.values().sum::<u64>();
        Self {
            author,
            weights,
            threshold: sum * 2 / 3 + 1,
            certified_data: Arc::new(RwLock::new(HashMap::new())),
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

    pub fn add_certified_data(&self, author: Author, data: Vec<u8>) {
        self.certified_data.write().insert(author, data);
    }
}
