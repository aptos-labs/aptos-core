// Copyright © Aptos Foundation
// Parts of the project are originally copyright © Meta Platforms, Inc.
// SPDX-License-Identifier: Apache-2.0

use crate::{block_info::Round, on_chain_config::OnChainConfig};
use aptos_crypto::HashValue;
use aptos_crypto_derive::SilentDebug;
use aptos_dkg::{weighted_vuf, weighted_vuf::traits::WeightedVUF};
use aptos_infallible::RwLock;
use serde::{Deserialize, Serialize};
use std::sync::Arc;

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

#[derive(Clone, Serialize, Deserialize, Debug, Default, PartialEq, Eq, Hash)]
pub struct RandMetadata {
    pub epoch: u64,
    pub round: Round,
}

#[derive(Clone, Serialize, Deserialize, Debug, PartialEq, Eq, Hash)]
pub struct FullRandMetadata {
    pub metadata: RandMetadata,
    // not used for signing
    pub block_id: HashValue,
    pub timestamp: u64,
}

impl FullRandMetadata {
    pub fn new(epoch: u64, round: Round, block_id: HashValue, timestamp: u64) -> Self {
        Self {
            metadata: RandMetadata { epoch, round },
            block_id,
            timestamp,
        }
    }

    pub fn round(&self) -> Round {
        self.metadata.round
    }

    pub fn epoch(&self) -> u64 {
        self.metadata.epoch
    }
}

#[derive(Clone, Serialize, Deserialize, Debug, Default, PartialEq, Eq)]
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
        self.metadata.epoch
    }

    pub fn round(&self) -> Round {
        self.metadata.round
    }

    pub fn randomness(&self) -> &[u8] {
        &self.randomness
    }

    pub fn randomness_cloned(&self) -> Vec<u8> {
        self.randomness.clone()
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
    pub apks: Vec<Arc<RwLock<Option<APK>>>>,
    // public key share of all validators, obtained from the DKG transcript of last epoch
    pub pk_shares: Vec<PKShare>,
}

impl RandKeys {
    pub fn new(ask: ASK, apk: APK, pk_shares: Vec<PKShare>, num_validators: usize) -> Self {
        let apks = (0..num_validators)
            .map(|_| Arc::new(RwLock::new(None)))
            .collect();

        Self {
            ask,
            apk,
            apks,
            pk_shares,
        }
    }

    pub fn set_apk(&self, index: usize, apk: APK) -> anyhow::Result<()> {
        assert!(index < self.apks.len());
        self.apks[index].write().replace(apk);
        Ok(())
    }
}
