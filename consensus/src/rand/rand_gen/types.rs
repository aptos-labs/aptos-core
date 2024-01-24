// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use anyhow::ensure;
use aptos_consensus_types::common::{Author, Round};
use aptos_crypto::bls12381::Signature;
use aptos_crypto_derive::{BCSCryptoHash, CryptoHasher};
use aptos_types::{
    aggregate_signature::AggregateSignature,
    randomness::{RandMetadata, Randomness},
    validator_verifier::ValidatorVerifier,
};
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use std::{collections::HashMap, fmt::Debug};

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub(super) struct MockShare;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub(super) struct MockAugData;

impl Share for MockShare {
    fn verify(
        &self,
        _rand_config: &RandConfig,
        _rand_metadata: &RandMetadata,
        _author: &Author,
    ) -> anyhow::Result<()> {
        Ok(())
    }

    fn generate(rand_config: &RandConfig, rand_metadata: RandMetadata) -> RandShare<Self>
    where
        Self: Sized,
    {
        RandShare::new(*rand_config.author(), rand_metadata, Self)
    }

    fn aggregate<'a>(
        _shares: impl Iterator<Item = &'a RandShare<Self>>,
        _rand_config: &RandConfig,
        rand_metadata: RandMetadata,
    ) -> Randomness
    where
        Self: Sized,
    {
        Randomness::new(rand_metadata, vec![])
    }
}

impl AugmentedData for MockAugData {
    fn generate(rand_config: &RandConfig) -> AugData<Self>
    where
        Self: Sized,
    {
        AugData::new(rand_config.epoch(), *rand_config.author(), Self)
    }

    fn augment(&self, _rand_config: &RandConfig, _author: &Author) {}

    fn verify(&self, _rand_config: &RandConfig, _author: &Author) -> anyhow::Result<()> {
        Ok(())
    }
}

pub trait Share:
    Clone + Debug + PartialEq + Send + Sync + Serialize + DeserializeOwned + 'static
{
    fn verify(
        &self,
        rand_config: &RandConfig,
        rand_metadata: &RandMetadata,
        author: &Author,
    ) -> anyhow::Result<()>;

    fn generate(rand_config: &RandConfig, rand_metadata: RandMetadata) -> RandShare<Self>
    where
        Self: Sized;

    fn aggregate<'a>(
        shares: impl Iterator<Item = &'a RandShare<Self>>,
        rand_config: &RandConfig,
        rand_metadata: RandMetadata,
    ) -> Randomness
    where
        Self: Sized;
}

pub trait AugmentedData:
    Clone + Debug + PartialEq + Send + Sync + Serialize + DeserializeOwned + 'static
{
    fn generate(rand_config: &RandConfig) -> AugData<Self>
    where
        Self: Sized;

    fn augment(&self, rand_config: &RandConfig, author: &Author);

    fn verify(&self, rand_config: &RandConfig, author: &Author) -> anyhow::Result<()>;
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
        self.share.verify(rand_config, &self.metadata, &self.author)
    }

    pub fn share_id(&self) -> ShareId {
        ShareId {
            epoch: self.epoch(),
            round: self.round(),
            author: self.author,
        }
    }
}

#[derive(Clone, Serialize, Deserialize)]
pub struct RequestShare {
    epoch: u64,
    rand_metadata: RandMetadata,
}

impl RequestShare {
    pub fn new(epoch: u64, rand_metadata: RandMetadata) -> Self {
        Self {
            epoch,
            rand_metadata,
        }
    }

    pub fn epoch(&self) -> u64 {
        self.epoch
    }

    pub fn rand_metadata(&self) -> &RandMetadata {
        &self.rand_metadata
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

impl<D: AugmentedData> AugData<D> {
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

    pub fn author(&self) -> &Author {
        &self.author
    }

    pub fn verify(&self, rand_config: &RandConfig, sender: Author) -> anyhow::Result<()> {
        ensure!(self.author == sender, "Invalid author");
        self.data.verify(rand_config, &self.author)?;
        Ok(())
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

impl<D: AugmentedData> CertifiedAugData<D> {
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

    pub fn author(&self) -> &Author {
        self.aug_data.author()
    }

    pub fn verify(&self, verifier: &ValidatorVerifier) -> anyhow::Result<()> {
        verifier.verify_multi_signatures(&self.aug_data, &self.signatures)?;
        Ok(())
    }

    pub fn data(&self) -> &D {
        &self.aug_data.data
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
    epoch: u64,
    author: Author,
    threshold: u64,
    weights: HashMap<Author, u64>,
}

impl RandConfig {
    pub fn new(epoch: u64, author: Author, weights: HashMap<Author, u64>) -> Self {
        let sum = weights.values().sum::<u64>();
        Self {
            epoch,
            author,
            weights,
            threshold: sum * 2 / 3 + 1,
        }
    }

    pub fn epoch(&self) -> u64 {
        self.epoch
    }

    pub fn author(&self) -> &Author {
        &self.author
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
