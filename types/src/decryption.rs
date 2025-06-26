// Copyright © Aptos Foundation
// Parts of the project are originally copyright © Meta Platforms, Inc.
// SPDX-License-Identifier: Apache-2.0

////////////////////////////////////////////////////////////
// Types for decryption
////////////////////////////////////////////////////////////


use aptos_crypto::hash::HashValue;
use serde::{Deserialize, Serialize};
use crate::{decryption_traits::{BatchThresholdEncryption, ThresholdConfig, Plaintext}, account_address::AccountAddress, validator_verifier::ValidatorVerifier};
use rand::RngCore;
use rayon::ThreadPool;
use anyhow::Result;
use aptos_dkg::{
    pvss::{Player, WeightedConfig},
};
use std::sync::Arc;

pub struct FernandoBTE;

impl BatchThresholdEncryption for FernandoBTE {
    type EncryptionKey = ();
    type DigestKey = ();
    type Ciphertext = Vec<u8>;
    type RoundNumber = u64;
    type RoundNumberRange = (u64, u64);
    type Digest = HashValue;
    type DecryptionAuxInfo = ();
    type MasterSecretKeyShare = ();
    type DecryptionKeyShare = ();
    type DecryptionKey = ();
    type Id = HashValue;

    fn setup(rng: &mut impl RngCore, max_batch_size: usize, tc: &ThresholdConfig)
        -> (Self::EncryptionKey, Self::DigestKey, Vec<Self::MasterSecretKeyShare>) {
        unimplemented!()
    }

    fn encrypt(ek: &Self::EncryptionKey, msg: impl Plaintext, t: Self::RoundNumberRange)
        -> Self::Ciphertext {
        unimplemented!()
    }

    fn digest(&self, cts: &[Self::Ciphertext], pool: &ThreadPool)
        -> Result<(Self::Digest, Self::DecryptionAuxInfo)> {
        unimplemented!()
    }

    fn verify_ct(_unverified_ct: &Self::Ciphertext) -> Result<()> {
        Ok(())
    }

    fn ct_round_number_range(ct: &Self::Ciphertext) -> Self::RoundNumberRange {
        unimplemented!()
    }

    fn ct_id(ct: &Self::Ciphertext) -> Self::Id {
        unimplemented!()
    }

    fn prepare_decryption_aux_info(aux: &mut Self::DecryptionAuxInfo, pool: &ThreadPool) {
        unimplemented!()
    }

    fn derive_decryption_key_share(
        msk_share: &Self::MasterSecretKeyShare,
        config: &ThresholdConfig,
        digest: &Self::Digest,
        t: Self::RoundNumber
        ) -> Self::DecryptionKeyShare {
        unimplemented!()
    }

    fn reconstruct_decryption_key(shares: &[Self::DecryptionKeyShare], config: &ThresholdConfig)
        -> Result<Self::DecryptionKey> {
        unimplemented!()
    }

    fn decrypt(
        cts: &[Self::Ciphertext],
        aux_info: Self::DecryptionAuxInfo,
        pool: ThreadPool
        ) -> Result<Vec<impl Plaintext>> {
        Ok(vec![MyPlaintext])
    }
}
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct MyPlaintext;
impl Plaintext for MyPlaintext {}

pub type EncryptionKey = <FernandoBTE as BatchThresholdEncryption>::EncryptionKey;
pub type DigestKey = <FernandoBTE as BatchThresholdEncryption>::DigestKey;
pub type Ciphertext = <FernandoBTE as BatchThresholdEncryption>::Ciphertext;
pub type Id = <FernandoBTE as BatchThresholdEncryption>::Id;
pub type Round = <FernandoBTE as BatchThresholdEncryption>::RoundNumber;
pub type RoundRange = <FernandoBTE as BatchThresholdEncryption>::RoundNumberRange;
pub type Digest = <FernandoBTE as BatchThresholdEncryption>::Digest;
pub type DecryptionAuxInfo = <FernandoBTE as BatchThresholdEncryption>::DecryptionAuxInfo;
pub type MasterSecretKeyShare = <FernandoBTE as BatchThresholdEncryption>::MasterSecretKeyShare;
pub type DecryptionKeyShare = <FernandoBTE as BatchThresholdEncryption>::DecryptionKeyShare;
pub type DecryptionKey = <FernandoBTE as BatchThresholdEncryption>::DecryptionKey;

pub type Author = AccountAddress;

#[derive(Clone, Debug, Deserialize, Serialize)]
pub enum DecryptionMessage {
    DecryptionKeyShare(DecShare),
    FastDecryptionKeyShare(FastDecShare),
}

#[derive(Clone, Serialize, Deserialize, Debug, Default, PartialEq, Eq, Hash)]
pub struct DecMetadata {
    pub epoch: u64,
    pub round: Round,
    pub timestamp: u64,
    pub block_id: HashValue,
}

impl DecMetadata {
    pub fn new(epoch: u64, round: Round, timestamp: u64, block_id: HashValue) -> Self {
        Self { epoch, round, timestamp, block_id }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct DecShare {
    pub author: Author,
    pub metadata: DecMetadata,
    pub share: DecryptionKeyShare,
}

impl DecShare {
    pub fn new_for_testing(author: Author, metadata: DecMetadata) -> Self {
        Self {
            author,
            metadata,
            share: DecryptionKeyShare::default(),
        }
    }

    pub fn verify(&self, config: &DecConfig) -> anyhow::Result<()> {
        Ok(())
    }

    pub fn aggregate<'a>(
        shares: impl Iterator<Item = &'a DecShare>,
        config: &DecConfig,
        metadata: DecMetadata,
    ) -> anyhow::Result<DecryptionKey> {
        // TODO: implement aggregation
        Ok(DecryptionKey::default())
    }

    pub fn author(&self) -> &Author {
        &self.author
    }

    pub fn share(&self) -> &DecryptionKeyShare {
        &self.share
    }

    pub fn metadata(&self) -> &DecMetadata {
        &self.metadata
    }

    pub fn round(&self) -> Round {
        self.metadata.round
    }

    pub fn epoch(&self) -> u64 {
        self.metadata.epoch
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct FastDecShare {
    pub share: DecShare,
}

impl FastDecShare {
    pub fn new(share: DecShare) -> Self {
        Self { share }
    }

    pub fn new_for_testing(author: Author, metadata: DecMetadata) -> Self {
        Self { share: DecShare::new_for_testing(author, metadata) }
    }

    pub fn share(&self) -> DecShare {
        self.share.clone()
    }

    pub fn author(&self) -> &Author {
        self.share.author()
    }

    pub fn metadata(&self) -> &DecMetadata {
        self.share.metadata()
    }

    pub fn round(&self) -> Round {
        self.share.round()
    }

    pub fn epoch(&self) -> u64 {
        self.share.epoch()
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct DecKey {
    pub metadata: DecMetadata,
    pub key: DecryptionKey,
}

impl DecKey {
    pub fn new(metadata: DecMetadata, key: DecryptionKey) -> Self {
        Self { metadata, key }
    }
}

#[derive(Clone)]
pub struct DecConfig {
    author: Author,
    epoch: u64,
    validator: Arc<ValidatorVerifier>,
    wconfig: WeightedConfig,
}

impl DecConfig {
    pub fn new(author: Author, epoch: u64, validator: Arc<ValidatorVerifier>, wconfig: WeightedConfig) -> Self {
        Self { author, epoch, validator, wconfig }
    }

    pub fn get_id(&self, peer: &Author) -> usize {
        *self
            .validator
            .address_to_validator_index()
            .get(peer)
            .expect("Peer should be in the index!")
    }

    pub fn get_peer_weight(&self, peer: &Author) -> u64 {
        let player = Player {
            id: self.get_id(peer),
        };
        self.wconfig.get_player_weight(&player) as u64
    }

    pub fn threshold(&self) -> u64 {
        self.wconfig.get_threshold_weight() as u64
    }
}
