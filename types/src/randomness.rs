// Copyright © Aptos Foundation
// Parts of the project are originally copyright © Meta Platforms, Inc.
// SPDX-License-Identifier: Apache-2.0

use aptos_crypto::HashValue;
use aptos_dkg::{weighted_vuf::{self, traits::WeightedVUF}, pvss::{WeightedConfig, Player}};
use move_core_types::account_address::AccountAddress;
use serde::{Deserialize, Serialize};
use crate::{block_info::Round, validator_verifier::ValidatorVerifier};

// pub type WVUF = weighted_vuf::pinkas::PinkasWUF;
pub type WVUF = weighted_vuf::gjm21_naive::g1::GjmNaiveWVUF;
pub type WvufPP = <WVUF as WeightedVUF>::PublicParameters;
pub type PK = <WVUF as WeightedVUF>::PubKey;
pub type PKShare = <WVUF as WeightedVUF>::PubKeyShare;
pub type ASK = <WVUF as WeightedVUF>::AugmentedSecretKeyShare;
pub type APK = <WVUF as WeightedVUF>::AugmentedPubKeyShare;
pub type ProofShare = <WVUF as WeightedVUF>::ProofShare;
pub type Delta = <WVUF as WeightedVUF>::Delta;
pub type Evaluation = <WVUF as WeightedVUF>::Evaluation;
pub type Proof = <WVUF as WeightedVUF>::Proof;

#[derive(Clone, Serialize, Deserialize, Debug, PartialEq, Eq)]
pub enum Mode {
    // randomness optimistic path
    Optimistic,
    // randomness fallback path
    Fallback,
}

#[derive(Clone, Serialize, Deserialize, Debug, PartialEq, Eq)]
pub struct RandMetadataToSign {
    pub epoch: u64,
    pub round: Round,
}

#[derive(Clone, Serialize, Deserialize, Debug, PartialEq, Eq)]
pub struct RandMetadata {
    pub metadata_to_sign: RandMetadataToSign,
    // not used for signing
    pub block_id: HashValue,
    pub timestamp: u64,
}

impl RandMetadata {
    pub fn new(epoch: u64, round: Round, block_id: HashValue, timestamp: u64) -> Self {
        Self { metadata_to_sign: RandMetadataToSign { epoch, round}, block_id, timestamp }
    }
    
    pub fn to_bytes(&self) -> Vec<u8> {
        // only sign (epoch, round) to produce randomness
        bcs::to_bytes(&self.metadata_to_sign).expect("[RandMessage] RandMetadata serialization failed!")
    }

    pub fn round(&self) -> Round {
        self.metadata_to_sign.round
    }

    pub fn epoch(&self) -> u64 {
        self.metadata_to_sign.epoch
    }
}

#[derive(Clone, Serialize, Deserialize, Debug, PartialEq, Eq)]
pub struct Randomness {
    metadata: RandMetadata,
    randomness: Vec<u8>,
}

impl Randomness {
    pub fn new(metadata: RandMetadata, randomness: Vec<u8>) -> Self {
        Self { metadata, randomness }
    }

    // Only used for the execution interface of ordering_state_computer which does not actually execute
    pub fn default() -> Self {
        let metadata = RandMetadata::new(0, 0, HashValue::zero(), 0);
        let randomness = vec![];
        Self { metadata, randomness }
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
}

#[derive(Clone, Serialize, Deserialize, Debug, PartialEq, Eq)]
pub struct RandDecision {
    randomness: Randomness,
    eval: Evaluation,
    proof: Proof,
}

impl RandDecision {
    pub fn new(randomness: Randomness, eval: Evaluation, proof: Proof) -> Self {
        Self { randomness, eval, proof }
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

    pub fn verify(&self, rand_config: &RandConfig) -> anyhow::Result<()> {
        <WVUF as WeightedVUF>::verify_eval(&rand_config.vuf_pp, &rand_config.pk, self.randomness.metadata.to_bytes().as_slice(), &self.proof, &self.eval)?;
        Ok(())
    }
}

#[derive(Clone)]
pub struct RandKeys {
    // augmented secret key share of this validator, obtained from the DKG transcript of last epoch
    pub ask: ASK,
    // augmented public key share of all validators, obtained from all validators in the new epoch
    // necessary for verifying randomness shares
    pub apks: Vec<Option<APK>>,
    // public key share of all validators, obtained from the DKG transcript of last epoch
    pub pk_shares: Vec<PKShare>,
}

impl RandKeys {
    pub fn new(ask: ASK, apk: APK, pk_shares: Vec<PKShare>, my_index: usize, num_validators: usize) -> Self {
        let apks = (0..num_validators).map(|i| if i == my_index { Some(apk.clone()) } else { None }).collect();
        Self { ask, apks, pk_shares }
    }

    pub fn add_apk(&mut self, index: usize, apk: APK) -> anyhow::Result<()> {
        assert!(index < self.apks.len());
        self.apks[index] = Some(apk);
        Ok(())
    }
}

#[derive(Clone)]
#[allow(dead_code)]
pub struct RandConfig {
    pub author: AccountAddress,
    pub validator: ValidatorVerifier,
    // public parameters of the weighted VUF
    pub vuf_pp: WvufPP,
    // public key for the weighted VUF
    pub pk: PK,
    // key shares for randomness fallback path
    pub keys_f: RandKeys,
    // key shares for randomness optimistic path
    pub keys_o: RandKeys,
    // weighted config for randomness fallback path
    pub wc_f: WeightedConfig,
    // weighted config for randomness optimistic path
    pub wc_o: WeightedConfig,
}

impl RandConfig {
    pub fn new(author: AccountAddress, validator: ValidatorVerifier, vuf_pp: WvufPP, pk: PK, keys_f: RandKeys, keys_o: RandKeys, wc_f: WeightedConfig, wc_o: WeightedConfig) -> Self {
        Self { author, validator, vuf_pp, pk, keys_f, keys_o, wc_f, wc_o }
    }

    pub fn get_id(&self, peer: &AccountAddress) -> usize {
        self.validator.address_to_validator_index().get(peer).unwrap().clone()
    }

    pub fn add_apk(&mut self, peer: &AccountAddress, apk: APK, mode: &Mode) -> anyhow::Result<()> {
        let index = self.get_id(peer);
        match mode {
            Mode::Optimistic => self.keys_o.add_apk(index, apk),
            Mode::Fallback => self.keys_f.add_apk(index, apk),
        }
    }

    pub fn get_apk(&self, peer: &AccountAddress, mode: &Mode) -> Option<&APK> {
        let index = self.get_id(peer);
        match mode {
            Mode::Optimistic => self.keys_o.apks[index].as_ref(),
            Mode::Fallback => self.keys_f.apks[index].as_ref(),
        }
    }

    pub fn get_pk_share(&self, peer: &AccountAddress, mode: &Mode) -> &PKShare {
        let index = self.get_id(peer);
        match mode {
            Mode::Optimistic => &self.keys_o.pk_shares[index],
            Mode::Fallback => &self.keys_f.pk_shares[index],
        }
    }

    pub fn add_delta(&mut self, peer: &AccountAddress, delta: Delta, mode: &Mode) -> anyhow::Result<()> {
        if self.get_apk(peer, mode).is_none() {
            let apk = <WVUF as WeightedVUF>::augment_pubkey(&self.vuf_pp, self.get_pk_share(peer, mode).clone(), delta.clone())?;
            self.add_apk(peer, apk, mode)?;
        }
        Ok(())
    }

    pub fn get_delta(&self, peer: &AccountAddress, mode: &Mode) -> Option<&Delta> {
        self.get_apk(peer, mode).map(<WVUF as WeightedVUF>::get_public_delta)
    }

    pub fn get_peer_weight(&self, peer: &AccountAddress, mode: &Mode) -> usize {
        let player = Player{ id: self.get_id(peer) };
        match mode {
            Mode::Optimistic => self.wc_o.get_player_weight(&player),
            Mode::Fallback => self.wc_f.get_player_weight(&player),
        }
    }

    pub fn th_f(&self) -> usize {
        self.wc_f.get_threshold_weight()
    }

    pub fn th_o(&self) -> usize {
        self.wc_o.get_threshold_weight()
    }
}