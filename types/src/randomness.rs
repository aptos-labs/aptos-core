// Copyright © Aptos Foundation
// Parts of the project are originally copyright © Meta Platforms, Inc.
// SPDX-License-Identifier: Apache-2.0

use aptos_crypto::HashValue;
use aptos_dkg::{weighted_vuf::{self, traits::WeightedVUF}, pvss::{WeightedConfig, Player}};
use move_core_types::account_address::AccountAddress;
use serde::{Deserialize, Serialize};
use crate::{block_info::Round, validator_verifier::ValidatorVerifier};

// pub type WVUF = weighted_vuf::pinkas::PinkasWUF;
pub type WVUF = weighted_vuf::gjm21_insecure::g1::GjmInsecureWVUF;
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
        // rand todo: if the caller locally does not have all the certified apks in Proof, the verification should fail.
        // to fix after crypto API is fixed
        <WVUF as WeightedVUF>::verify_eval(&rand_config.vuf_pp, &rand_config.pk, self.randomness.metadata.to_bytes().as_slice(), &self.proof, &self.eval)?;
        Ok(())
    }
}

#[derive(Clone)]
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
    pub certified_apks: Vec<Option<APK>>,
    // public key share of all validators, obtained from the DKG transcript of last epoch
    pub pk_shares: Vec<PKShare>,
}

impl RandKeys {
    pub fn new(ask: ASK, apk: APK, pk_shares: Vec<PKShare>, num_validators: usize) -> Self {
        let signed_deltas = vec![None; num_validators];
        let certified_apks = vec![None; num_validators];

        Self { ask, apk, signed_deltas, certified_apks, pk_shares }
    }

    pub fn add_signed_delta(&mut self, index: usize, delta: Delta) -> anyhow::Result<()> {
        assert!(index < self.signed_deltas.len());
        if self.signed_deltas[index].is_some() {
            anyhow::bail!("Delta already signed for validator {}!", index);
        }
        self.signed_deltas[index] = Some(delta);
        Ok(())
    }

    pub fn add_certified_apk(&mut self, index: usize, apk: APK) -> anyhow::Result<()> {
        assert!(index < self.certified_apks.len());
        if self.certified_apks[index].is_some() {
            return Ok(());
        }
        self.certified_apks[index] = Some(apk);
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
    // key shares for weighted VUF
    pub keys: RandKeys,
    // weighted config for weighted VUF
    pub wconfig: WeightedConfig,
}

impl RandConfig {
    pub fn new(author: AccountAddress, validator: ValidatorVerifier, vuf_pp: WvufPP, pk: PK, keys: RandKeys, wconfig: WeightedConfig) -> Self {
        Self { author, validator, vuf_pp, pk, keys, wconfig}
    }

    pub fn get_id(&self, peer: &AccountAddress) -> usize {
        self.validator.address_to_validator_index().get(peer).unwrap().clone()
    }

    pub fn get_signed_delta(&self, peer: &AccountAddress) -> Option<&Delta> {
        let index = self.get_id(peer);
        self.keys.signed_deltas[index].as_ref()
    }

    pub fn add_signed_delta(&mut self, peer: &AccountAddress, delta: Delta) -> anyhow::Result<()> {
        let index = self.get_id(peer);
        self.keys.add_signed_delta(index, delta)
    }

    pub fn get_certified_apk(&self, peer: &AccountAddress) -> Option<&APK> {
        let index = self.get_id(peer);
        self.keys.certified_apks[index].as_ref()
    }

    pub fn add_certified_apk(&mut self, peer: &AccountAddress, apk: APK) -> anyhow::Result<()> {
        let index = self.get_id(peer);
        self.keys.add_certified_apk(index, apk)
    }

    pub fn add_certified_delta(&mut self, peer: &AccountAddress, delta: Delta) -> anyhow::Result<()> {
        let apk = <WVUF as WeightedVUF>::augment_pubkey(&self.vuf_pp, self.get_pk_share(peer).clone(), delta)?;
        self.add_certified_apk(peer, apk)?;
        Ok(())
    }

    pub fn get_my_delta(&self) -> &Delta {
        <WVUF as WeightedVUF>::get_public_delta(&self.keys.apk)
    }

    pub fn get_pk_share(&self, peer: &AccountAddress) -> &PKShare {
        let index = self.get_id(peer);
        &self.keys.pk_shares[index]
    }

    pub fn get_peer_weight(&self, peer: &AccountAddress) -> usize {
        let player = Player{ id: self.get_id(peer) };
        self.wconfig.get_player_weight(&player)
    }

    pub fn threshold(&self) -> usize {
        self.wconfig.get_threshold_weight()
    }
}