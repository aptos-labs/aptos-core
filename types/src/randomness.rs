// Copyright © Aptos Foundation
// Parts of the project are originally copyright © Meta Platforms, Inc.
// SPDX-License-Identifier: Apache-2.0

use once_cell::sync::OnceCell;
use crate::{block_info::Round, on_chain_config::OnChainConfig};
use aptos_crypto::HashValue;
use serde::{Deserialize, Serialize};
use aptos_crypto_derive::SilentDebug;
use aptos_dkg::weighted_vuf;
use aptos_dkg::weighted_vuf::traits::WeightedVUF;

pub type WVUF = weighted_vuf::pinkas::PinkasWUF;
// pub type WVUF = weighted_vuf::gjm21_insecure::g1::GjmInsecureWVUF;
pub type WvufPP = <WVUF as WeightedVUF>::PublicParameters;
pub type PK = <WVUF as WeightedVUF>::PubKey;
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
    pub seed: Vec<u8>,
}

impl OnChainConfig for PerBlockRandomness {
    const MODULE_IDENTIFIER: &'static str = "randomness";
    const TYPE_IDENTIFIER: &'static str = "PerBlockRandomness";
}

#[derive(Clone, Serialize, Deserialize, Debug, PartialEq, Eq)]
pub struct RandDecision {
    randomness: Randomness,
    eval: Evaluation,
    proof: Proof,
}

impl RandDecision {
    pub fn new(randomness: Randomness, eval: Evaluation, proof: Proof) -> Self {
        Self {
            randomness,
            eval,
            proof,
        }
    }

    pub fn randomness(&self) -> &Randomness {
        &self.randomness
    }

    pub fn evaluation(&self) -> &Evaluation {
        &self.eval
    }

    pub fn metadata(&self) -> &RandMetadata {
        &self.randomness.metadata
    }

    pub fn proof(&self) -> &Proof {
        &self.proof
    }

    pub fn epoch(&self) -> u64 {
        self.randomness.epoch()
    }

    pub fn round(&self) -> Round {
        self.randomness.round()
    }

    pub fn block_id(&self) -> HashValue {
        self.metadata().block_id
    }

    pub fn timestamp(&self) -> u64 {
        self.metadata().timestamp
    }

    // pub fn verify(&self, rand_config: &RandConfig) -> anyhow::Result<()> {
    //     // If the caller locally does not have all the certified apks corresponding to self.proof, the verification should fail.
    //     // Then RandShare multicast may be retried periodically and the caller will receive RandDecision.
    //     // Eventually the caller will receive certified apks to verify the proof in RandDecision.
    //     <WVUF as WeightedVUF>::verify_proof(
    //         &rand_config.vuf_pp,
    //         &rand_config.pk,
    //         rand_config.get_all_certified_apk(),
    //         self.randomness.metadata.to_bytes().as_slice(),
    //         &self.proof,
    //     )
    // }
}

#[derive(Clone, SilentDebug)]
pub struct RandKeys {
    // augmented secret / public key share of this validator, obtained from the DKG transcript of last epoch
    pub ask: ASK,
    pub apk: APK,
    // deltas of all validators which this validator signed,
    // needs to be persisted for unequivocation
    pub signed_deltas: Vec<Option<Delta>>,
    // certified augmented public key share of all validators,
    // obtained from all validators in the new epoch,
    // which necessary for verifying randomness shares
    pub certified_apks: Vec<OnceCell<APK>>,
    // public key share of all validators, obtained from the DKG transcript of last epoch
    pub pk_shares: Vec<PKShare>,
}

impl RandKeys {
    pub fn new(ask: ASK, apk: APK, pk_shares: Vec<PKShare>, num_validators: usize) -> Self {
        let signed_deltas = vec![None; num_validators];
        let certified_apks = vec![OnceCell::new(); num_validators];

        Self {
            ask,
            apk,
            signed_deltas,
            certified_apks,
            pk_shares,
        }
    }

    pub fn add_signed_delta(&mut self, index: usize, delta: Delta) -> anyhow::Result<()> {
        assert!(index < self.signed_deltas.len());
        if self.signed_deltas[index].is_some() {
            anyhow::bail!("Delta already signed for validator {}!", index);
        }
        self.signed_deltas[index] = Some(delta);
        Ok(())
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
