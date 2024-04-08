// Copyright © Aptos Foundation
// Parts of the project are originally copyright © Meta Platforms, Inc.
// SPDX-License-Identifier: Apache-2.0

use crate::{block_info::Round, on_chain_config::OnChainConfig};
use aptos_crypto::HashValue;
use aptos_crypto_derive::SilentDebug;
use aptos_dkg::{weighted_vuf, weighted_vuf::traits::WeightedVUF};
use once_cell::sync::OnceCell;
use serde::{Deserialize, Serialize};

pub type WVUF = weighted_vuf::pinkas::PinkasWUF;
pub type WvufPP = <WVUF as WeightedVUF>::PublicParameters;
pub type PK = <WVUF as WeightedVUF>::PubKey;
pub type SKShare = <WVUF as WeightedVUF>::SecretKeyShare;
pub type PKShare = <WVUF as WeightedVUF>::PubKeyShare;
pub type ASK = <WVUF as WeightedVUF>::AugmentedSecretKeyShare;
pub type APK = <WVUF as WeightedVUF>::AugmentedPubKeyShare;
pub type ProofShare = <WVUF as WeightedVUF>::ProofShare;
pub type Delta = <WVUF as WeightedVUF>::Delta;
pub type Evaluation = <WVUF as WeightedVUF>::Evaluation;
pub type Proof = <WVUF as WeightedVUF>::Proof;

#[derive(Clone, Serialize, Deserialize, Debug, PartialEq, Eq, Hash)]
pub struct RandMetadataToSign {
    pub epoch: u64,
    pub round: Round,
}

#[derive(Clone, Serialize, Deserialize, Debug, PartialEq, Eq, Hash)]
pub struct RandMetadata {
    pub metadata_to_sign: RandMetadataToSign,
    // not used for signing
    pub block_id: HashValue,
    pub timestamp: u64,
}

impl RandMetadata {
    pub fn new(epoch: u64, round: Round, block_id: HashValue, timestamp: u64) -> Self {
        Self {
            metadata_to_sign: RandMetadataToSign { epoch, round },
            block_id,
            timestamp,
        }
    }

    pub fn to_bytes(&self) -> Vec<u8> {
        // only sign (epoch, round) to produce randomness
        bcs::to_bytes(&self.metadata_to_sign)
            .expect("[RandMessage] RandMetadata serialization failed!")
    }

    pub fn round(&self) -> Round {
        self.metadata_to_sign.round
    }

    pub fn epoch(&self) -> u64 {
        self.metadata_to_sign.epoch
    }

    #[cfg(any(test, feature = "fuzzing"))]
    pub fn new_for_testing(round: Round) -> Self {
        Self::new(1, round, HashValue::zero(), 1)
    }
}

#[derive(Clone, Serialize, Deserialize, Debug, PartialEq, Eq)]
pub struct Randomness {
    metadata: RandMetadata,
    #[serde(with = "serde_bytes")]
    randomness: Vec<u8>,
}

impl Randomness {
    pub fn new(metadata: RandMetadata, randomness: Vec<u8>) -> Self {
        Self {
            metadata,
            randomness,
        }
    }

    pub fn metadata(&self) -> &RandMetadata {
        &self.metadata
    }

    pub fn epoch(&self) -> u64 {
        self.metadata.metadata_to_sign.epoch
    }

    pub fn round(&self) -> Round {
        self.metadata.metadata_to_sign.round
    }

    pub fn randomness(&self) -> &[u8] {
        &self.randomness
    }

    pub fn randomness_cloned(&self) -> Vec<u8> {
        self.randomness.clone()
    }
}

impl Default for Randomness {
    fn default() -> Self {
        let metadata = RandMetadata::new(0, 0, HashValue::zero(), 0);
        let randomness = vec![];
        Self {
            metadata,
            randomness,
        }
    }
}

#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
pub struct PerBlockRandomness {
    pub epoch: u64,
    pub round: u64,
    pub seed: Option<Vec<u8>>,
}

impl OnChainConfig for PerBlockRandomness {
    const MODULE_IDENTIFIER: &'static str = "randomness";
    const TYPE_IDENTIFIER: &'static str = "PerBlockRandomness";
}

#[derive(Clone, SilentDebug)]
pub struct RandKeys {
    // augmented secret / public key share of this validator, obtained from the DKG transcript of last epoch
    pub ask: ASK,
    pub apk: APK,
    // certified augmented public key share of all validators,
    // obtained from all validators in the new epoch,
    // which necessary for verifying randomness shares
    pub certified_apks: Vec<OnceCell<APK>>,
    // public key share of all validators, obtained from the DKG transcript of last epoch
    pub pk_shares: Vec<PKShare>,
}

impl RandKeys {
    pub fn new(ask: ASK, apk: APK, pk_shares: Vec<PKShare>, num_validators: usize) -> Self {
        let certified_apks = vec![OnceCell::new(); num_validators];

        Self {
            ask,
            apk,
            certified_apks,
            pk_shares,
        }
    }

    pub fn add_certified_apk(&self, index: usize, apk: APK) -> anyhow::Result<()> {
        assert!(index < self.certified_apks.len());
        if self.certified_apks[index].get().is_some() {
            return Ok(());
        }
        self.certified_apks[index].set(apk).unwrap();
        Ok(())
    }
}
