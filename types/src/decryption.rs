// Copyright © Aptos Foundation
// Parts of the project are originally copyright © Meta Platforms, Inc.
// SPDX-License-Identifier: Apache-2.0

////////////////////////////////////////////////////////////
// Types for decryption
////////////////////////////////////////////////////////////


use aptos_crypto::hash::HashValue;
use serde::{Deserialize, Serialize};
use crate::{account_address::AccountAddress, validator_verifier::ValidatorVerifier};
use std::sync::Arc;
use aptos_batch_encryption::traits::BatchThresholdEncryption;
use aptos_batch_encryption::schemes::fptx::FPTX;
use aptos_batch_encryption::group::Fr;
use aptos_crypto::arkworks::shamir::ShamirThresholdConfig;
use once_cell::sync::Lazy;


pub type ThresholdConfig = ShamirThresholdConfig<Fr>;

pub type EncryptionKey = <FPTX as BatchThresholdEncryption>::EncryptionKey;
pub type DigestKey = <FPTX as BatchThresholdEncryption>::DigestKey;
pub type Ciphertext = <FPTX as BatchThresholdEncryption>::Ciphertext;
pub type Id = <FPTX as BatchThresholdEncryption>::Id;
pub type Round = <FPTX as BatchThresholdEncryption>::Round;
pub type Digest = <FPTX as BatchThresholdEncryption>::Digest;
pub type EvalProofsPromise = <FPTX as BatchThresholdEncryption>::EvalProofsPromise;
pub type EvalProofs = <FPTX as BatchThresholdEncryption>::EvalProofs;
pub type MasterSecretKeyShare = <FPTX as BatchThresholdEncryption>::MasterSecretKeyShare;
pub type VerificationKey = <FPTX as BatchThresholdEncryption>::VerificationKey;
pub type DecryptionKeyShare = <FPTX as BatchThresholdEncryption>::DecryptionKeyShare;
pub type DecryptionKey = <FPTX as BatchThresholdEncryption>::DecryptionKey;

pub type Author = AccountAddress;

pub const PROTOTYPE_SETUP_SEED: u64 = 233;
pub const PROTOTYPE_BATCH_SIZE: usize = 128;
pub const PROTOTYPE_NUMBER_OF_ROUNDS: usize = 1;
pub const PROTOTYPE_NUMBER_OF_VALIDATORS: usize = 50;
pub const PROTOTYPE_THRESHOLD_FAST_PATH: usize = 35;
pub const PROTOTYPE_THRESHOLD_SLOW_PATH: usize = 26;
pub const PROTOTYPE_DECRYPTION_POOL_SIZE: usize = 16;

pub static DECRYPTION_POOL: Lazy<Arc<rayon::ThreadPool>> = Lazy::new(|| {
    Arc::new(
        rayon::ThreadPoolBuilder::new()
            .num_threads(PROTOTYPE_DECRYPTION_POOL_SIZE) // More than 8 threads doesn't seem to help much
            .thread_name(|index| format!("decryption-{}", index))
            .build()
            .unwrap(),
    )
});

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
    pub digest: Digest,
}

impl DecMetadata {
    pub fn new(epoch: u64, round: Round, timestamp: u64, block_id: HashValue, digest: Digest) -> Self {
        Self { epoch, round, timestamp, block_id, digest }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct DecShare {
    pub author: Author,
    pub metadata: DecMetadata,
    pub share: DecryptionKeyShare,
}

impl DecShare {
    pub fn new(author: Author, metadata: DecMetadata, share: DecryptionKeyShare) -> Self {
        Self { author, metadata, share }
    }

    pub fn verify(&self, config: &DecConfig) -> anyhow::Result<()> {
        let index = config.get_id(self.author());
        let decryption_key_share = self.share().clone();
        config.verification_keys[index].verify_decryption_key_share(&self.metadata.digest, &decryption_key_share)?;
        Ok(())
    }

    pub fn aggregate<'a>(
        dec_shares: impl Iterator<Item = &'a DecShare>,
        config: &DecConfig,
    ) -> anyhow::Result<DecryptionKey> {
        let threshold = config.threshold();
        let shares: Vec<DecryptionKeyShare> = dec_shares
            .map(|dec_share| dec_share.share.clone())
            .take(threshold as usize)
            .collect();
        
        let decryption_key = <FPTX as BatchThresholdEncryption>::reconstruct_decryption_key(&shares, &config.config)?;
        Ok(decryption_key)
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

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct FastDecShare {
    pub share: DecShare,
}

impl FastDecShare {
    pub fn new(share: DecShare) -> Self {
        Self { share }
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
    // wconfig: WeightedConfig,
    digest_key: DigestKey,
    msk_share: MasterSecretKeyShare,
    verification_keys: Vec<VerificationKey>,
    config: ThresholdConfig,
    encryption_key: EncryptionKey,
}

impl DecConfig {
    pub fn new(author: Author, epoch: u64, validator: Arc<ValidatorVerifier>, digest_key: DigestKey, msk_share: MasterSecretKeyShare, verification_keys: Vec<VerificationKey>, config: ThresholdConfig, encryption_key: EncryptionKey) -> Self {
        Self { author, epoch, validator, digest_key, msk_share, verification_keys, config, encryption_key }
    }

    pub fn get_id(&self, peer: &Author) -> usize {
        *self
            .validator
            .address_to_validator_index()
            .get(peer)
            .expect("Peer should be in the index!")
    }

    pub fn digest_key(&self) -> &DigestKey {
        &self.digest_key
    }

    pub fn msk_share(&self) -> &MasterSecretKeyShare {
        &self.msk_share
    }

    pub fn threshold(&self) -> u64 {
        self.config.t as u64
    }

    pub fn number_of_validators(&self) -> u64 {
        self.config.n as u64
    }

    pub fn get_peer_weight(&self, _peer: &Author) -> u64 {
        // daniel todo: use weighted config
        1
    }

    pub fn encryption_key(&self) -> &EncryptionKey {
        &self.encryption_key
    }
}
