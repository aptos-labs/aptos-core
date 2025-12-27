// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use aptos_batch_encryption::group::Fr;
use aptos_consensus_types::common::Author;
use aptos_crypto::arkworks::shamir::ShamirThresholdConfig;
use aptos_types::{
    secret_sharing::{
        DigestKey, EncryptionKey, MasterSecretKeyShare, SecretShareMetadata, VerificationKey,
    },
    validator_verifier::ValidatorVerifier,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

pub const FUTURE_ROUNDS_TO_ACCEPT: u64 = 200;

pub type ThresholdConfig = ShamirThresholdConfig<Fr>;

#[derive(Clone, Serialize, Deserialize)]
pub struct RequestSecretShare {
    metadata: SecretShareMetadata,
}

impl RequestSecretShare {
    pub fn new(metadata: SecretShareMetadata) -> Self {
        Self { metadata }
    }

    pub fn epoch(&self) -> u64 {
        self.metadata.epoch
    }

    pub fn metadata(&self) -> &SecretShareMetadata {
        &self.metadata
    }
}

#[derive(Clone)]
pub struct SecretSharingConfig {
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

impl SecretSharingConfig {
    pub fn new(
        author: Author,
        epoch: u64,
        validator: Arc<ValidatorVerifier>,
        digest_key: DigestKey,
        msk_share: MasterSecretKeyShare,
        verification_keys: Vec<VerificationKey>,
        config: ThresholdConfig,
        encryption_key: EncryptionKey,
    ) -> Self {
        Self {
            author,
            epoch,
            validator,
            digest_key,
            msk_share,
            verification_keys,
            config,
            encryption_key,
        }
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
