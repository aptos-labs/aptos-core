// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use anyhow::{bail, ensure};
use aptos_consensus_types::common::{Author, Round};
use aptos_crypto::bls12381::Signature;
use aptos_crypto_derive::{BCSCryptoHash, CryptoHasher};
use aptos_dkg::{
    pvss::{Player, WeightedConfig},
    weighted_vuf::traits::WeightedVUF,
};
use aptos_types::{
    aggregate_signature::AggregateSignature,
    randomness::{
        Delta, PKShare, ProofShare, RandKeys, RandMetadata, Randomness, WvufPP, APK, PK, WVUF,
    },
    validator_verifier::ValidatorVerifier,
};
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use sha3::{Digest, Sha3_256};
use std::{fmt::Debug, sync::Arc};

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub(super) struct MockShare;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub(super) struct MockAugData;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct RealShare {
    share: ProofShare,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct RealAugmentedData {
    delta: Delta,
}

impl Share for RealShare {
    fn verify(
        &self,
        rand_config: &RandConfig,
        rand_metadata: &RandMetadata,
        author: &Author,
    ) -> anyhow::Result<()> {
        let index = *rand_config
            .validator
            .address_to_validator_index()
            .get(&author)
            .unwrap();
        let maybe_apk = &rand_config.keys.certified_apks[index];
        if let Some(apk) = maybe_apk.get() {
            <WVUF as WeightedVUF>::verify_share(
                &rand_config.vuf_pp,
                apk,
                rand_metadata.to_bytes().as_slice(),
                &self.share,
            )?;
        } else {
            bail!(
                "[RandShare] No augmented public key for validator id {}, {}",
                index,
                author
            );
        }
        Ok(())
    }

    fn generate(rand_config: &RandConfig, rand_metadata: RandMetadata) -> RandShare<Self>
    where
        Self: Sized,
    {
        let share = RealShare {
            share: <WVUF as WeightedVUF>::create_share(
                &rand_config.keys.ask,
                rand_metadata.to_bytes().as_slice(),
            ),
        };
        RandShare::new(rand_config.author(), rand_metadata, share)
    }

    fn aggregate<'a>(
        shares: impl Iterator<Item = &'a RandShare<Self>>,
        rand_config: &RandConfig,
        rand_metadata: RandMetadata,
    ) -> Randomness
    where
        Self: Sized,
    {
        let mut apks_and_proofs = vec![];
        for share in shares {
            let id = *rand_config
                .validator
                .address_to_validator_index()
                .get(share.author())
                .unwrap();
            let apk = rand_config.get_certified_apk(share.author()).unwrap(); // needs to have apk to verify the share
            apks_and_proofs.push((Player { id }, apk.clone(), share.share().share.clone()));
        }

        let proof = <WVUF as WeightedVUF>::aggregate_shares(&rand_config.wconfig, &apks_and_proofs);
        let eval = <WVUF as WeightedVUF>::derive_eval(
            &rand_config.wconfig,
            &rand_config.vuf_pp,
            rand_metadata.to_bytes().as_slice(),
            &rand_config.get_all_certified_apk(),
            &proof,
        )
        .expect("All APK should exist");
        let eval_bytes = bcs::to_bytes(&eval).unwrap();
        let rand_bytes = Sha3_256::digest(eval_bytes.as_slice()).to_vec();
        Randomness::new(rand_metadata.clone(), rand_bytes)
    }
}

impl AugmentedData for RealAugmentedData {
    fn generate(rand_config: &RandConfig) -> AugData<Self>
    where
        Self: Sized,
    {
        let delta = rand_config.get_my_delta().clone();
        rand_config
            .add_certified_delta(&rand_config.author(), delta.clone())
            .expect("Add self delta should succeed");
        let data = RealAugmentedData {
            delta: delta.clone(),
        };
        AugData::new(rand_config.epoch(), rand_config.author(), data)
    }

    fn augment(&self, rand_config: &RandConfig, author: &Author) {
        let RealAugmentedData { delta } = self;
        rand_config
            .add_certified_delta(author, delta.clone())
            .expect("Add delta should succeed")
    }

    fn verify(&self, rand_config: &RandConfig, author: &Author) -> anyhow::Result<()> {
        rand_config
            .derive_apk(author, self.delta.clone())
            .map(|_| ())
    }
}

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
        RandShare::new(rand_config.author(), rand_metadata, Self)
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
        AugData::new(rand_config.epoch(), rand_config.author(), Self)
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

    pub fn share(&self) -> &S {
        &self.share
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

#[derive(Clone, Debug)]
pub struct RandConfig {
    pub author: Author,
    pub epoch: u64,
    pub validator: ValidatorVerifier,
    // public parameters of the weighted VUF
    pub vuf_pp: WvufPP,
    // public key for the weighted VUF
    pub pk: PK,
    // key shares for weighted VUF
    pub keys: Arc<RandKeys>,
    // weighted config for weighted VUF
    pub wconfig: WeightedConfig,
}

impl RandConfig {
    pub fn new(
        author: Author,
        epoch: u64,
        validator: ValidatorVerifier,
        vuf_pp: WvufPP,
        pk: PK,
        keys: RandKeys,
        wconfig: WeightedConfig,
    ) -> Self {
        Self {
            author,
            epoch,
            validator,
            vuf_pp,
            pk,
            keys: Arc::new(keys),
            wconfig,
        }
    }

    pub fn epoch(&self) -> u64 {
        self.epoch
    }

    pub fn author(&self) -> Author {
        self.author
    }

    pub fn get_id(&self, peer: &Author) -> usize {
        *self
            .validator
            .address_to_validator_index()
            .get(peer)
            .unwrap()
    }

    //
    // pub fn get_signed_delta(&self, peer: &Author) -> Option<&Delta> {
    //     let index = self.get_id(peer);
    //     self.keys.signed_deltas[index].as_ref()
    // }
    //
    // pub fn add_signed_delta(&mut self, peer: &Author, delta: Delta) -> anyhow::Result<()> {
    //     let index = self.get_id(peer);
    //     self.keys.add_signed_delta(index, delta)
    // }

    pub fn get_certified_apk(&self, peer: &Author) -> Option<&APK> {
        let index = self.get_id(peer);
        self.keys.certified_apks[index].get()
    }

    pub fn get_all_certified_apk(&self) -> Vec<Option<APK>> {
        self.keys
            .certified_apks
            .iter()
            .map(|cell| cell.get().cloned())
            .collect()
    }

    pub fn add_certified_apk(&self, peer: &Author, apk: APK) -> anyhow::Result<()> {
        let index = self.get_id(peer);
        self.keys.add_certified_apk(index, apk)
    }

    fn derive_apk(&self, peer: &Author, delta: Delta) -> anyhow::Result<APK> {
        let apk = <WVUF as WeightedVUF>::augment_pubkey(
            &self.vuf_pp,
            self.get_pk_share(peer).clone(),
            delta,
        )?;
        Ok(apk)
    }

    pub fn add_certified_delta(&self, peer: &Author, delta: Delta) -> anyhow::Result<()> {
        let apk = self.derive_apk(peer, delta)?;
        self.add_certified_apk(peer, apk)?;
        Ok(())
    }

    pub fn get_my_delta(&self) -> &Delta {
        <WVUF as WeightedVUF>::get_public_delta(&self.keys.apk)
    }

    pub fn get_pk_share(&self, peer: &Author) -> &PKShare {
        let index = self.get_id(peer);
        &self.keys.pk_shares[index]
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
