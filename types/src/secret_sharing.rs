// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

////////////////////////////////////////////////////////////
// Types for Secret Sharing
////////////////////////////////////////////////////////////

use crate::{account_address::AccountAddress, validator_verifier::ValidatorVerifier};
use aptos_batch_encryption::{
    schemes::fptx_weighted::FPTXWeighted, traits::BatchThresholdEncryption,
};
use aptos_crypto::hash::HashValue;
use serde::{Deserialize, Serialize};
use std::{collections::HashMap, sync::Arc};

pub type EncryptionKey = <FPTXWeighted as BatchThresholdEncryption>::EncryptionKey;
pub type DigestKey = <FPTXWeighted as BatchThresholdEncryption>::DigestKey;
pub type Ciphertext = <FPTXWeighted as BatchThresholdEncryption>::Ciphertext;
pub type Id = <FPTXWeighted as BatchThresholdEncryption>::Id;
pub type Round = <FPTXWeighted as BatchThresholdEncryption>::Round;
pub type Digest = <FPTXWeighted as BatchThresholdEncryption>::Digest;
pub type EvalProofsPromise = <FPTXWeighted as BatchThresholdEncryption>::EvalProofsPromise;
pub type EvalProof = <FPTXWeighted as BatchThresholdEncryption>::EvalProof;
pub type EvalProofs = <FPTXWeighted as BatchThresholdEncryption>::EvalProofs;
pub type MasterSecretKeyShare = <FPTXWeighted as BatchThresholdEncryption>::MasterSecretKeyShare;
pub type VerificationKey = <FPTXWeighted as BatchThresholdEncryption>::VerificationKey;
pub type SecretKeyShare = <FPTXWeighted as BatchThresholdEncryption>::DecryptionKeyShare;
pub type DecryptionKey = <FPTXWeighted as BatchThresholdEncryption>::DecryptionKey;

pub type Author = AccountAddress;

#[derive(Clone, Serialize, Deserialize, Debug, Default, PartialEq, Eq, Hash)]
pub struct SecretShareMetadata {
    pub epoch: u64,
    pub round: Round,
    pub timestamp: u64,
    pub block_id: HashValue,
    pub digest: Digest,
}

impl SecretShareMetadata {
    pub fn new(
        epoch: u64,
        round: Round,
        timestamp: u64,
        block_id: HashValue,
        digest: Digest,
    ) -> Self {
        Self {
            epoch,
            round,
            timestamp,
            block_id,
            digest,
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SecretShare {
    pub author: Author,
    pub metadata: SecretShareMetadata,
    pub share: SecretKeyShare,
}

impl SecretShare {
    pub fn new(author: Author, metadata: SecretShareMetadata, share: SecretKeyShare) -> Self {
        Self {
            author,
            metadata,
            share,
        }
    }

    pub fn verify(&self, config: &SecretShareConfig) -> anyhow::Result<()> {
        let index = config.get_id(self.author());
        let decryption_key_share = self.share().clone();
        // TODO(ibalajiarun): Check index out of bounds
        config.verification_keys[index]
            .verify_decryption_key_share(&self.metadata.digest, &decryption_key_share)?;
        Ok(())
    }

    pub fn aggregate<'a>(
        dec_shares: impl Iterator<Item = &'a SecretShare>,
        config: &SecretShareConfig,
    ) -> anyhow::Result<DecryptionKey> {
        let threshold = config.threshold();
        let shares: Vec<SecretKeyShare> = dec_shares
            .map(|dec_share| dec_share.share.clone())
            .take(threshold as usize)
            .collect();
        let decryption_key =
            <FPTXWeighted as BatchThresholdEncryption>::reconstruct_decryption_key(
                &shares,
                &config.config,
            )?;
        Ok(decryption_key)
    }

    pub fn author(&self) -> &Author {
        &self.author
    }

    pub fn share(&self) -> &SecretKeyShare {
        &self.share
    }

    pub fn metadata(&self) -> &SecretShareMetadata {
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
pub struct SecretSharedKey {
    pub metadata: SecretShareMetadata,
    pub key: DecryptionKey,
}

impl SecretSharedKey {
    pub fn new(metadata: SecretShareMetadata, key: DecryptionKey) -> Self {
        Self { metadata, key }
    }
}

/// This is temporary and meant to change in future PRs
#[derive(Clone)]
pub struct SecretShareConfig {
    _author: Author,
    _epoch: u64,
    validator: Arc<ValidatorVerifier>,
    digest_key: DigestKey,
    msk_share: MasterSecretKeyShare,
    verification_keys: Vec<VerificationKey>,
    config: <FPTXWeighted as BatchThresholdEncryption>::ThresholdConfig,
    encryption_key: EncryptionKey,
    weights: HashMap<Author, u64>,
}

impl SecretShareConfig {
    pub fn new(
        author: Author,
        epoch: u64,
        validator: Arc<ValidatorVerifier>,
        digest_key: DigestKey,
        msk_share: MasterSecretKeyShare,
        verification_keys: Vec<VerificationKey>,
        config: <FPTXWeighted as BatchThresholdEncryption>::ThresholdConfig,
        encryption_key: EncryptionKey,
        weights: HashMap<Author, u64>,
    ) -> Self {
        Self {
            _author: author,
            _epoch: epoch,
            validator,
            digest_key,
            msk_share,
            verification_keys,
            config,
            encryption_key,
            weights,
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
        self.config.get_threshold_config().t as u64
    }

    pub fn number_of_validators(&self) -> u64 {
        self.config.get_threshold_config().n as u64
    }

    pub fn get_peer_weight(&self, _peer: &Author) -> u64 {
        1
    }

    pub fn get_peer_weights(&self) -> &HashMap<Author, u64> {
        &self.weights
    }

    pub fn encryption_key(&self) -> &EncryptionKey {
        &self.encryption_key
    }
}
