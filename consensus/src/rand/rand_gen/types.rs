// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use anyhow::{anyhow, bail, ensure};
use aptos_consensus_types::common::{Author, Round};
use aptos_crypto::bls12381::Signature;
use aptos_crypto_derive::{BCSCryptoHash, CryptoHasher};
use aptos_dkg::{
    pvss::{Player, WeightedConfigBlstrs},
    weighted_vuf::traits::WeightedVUF,
};
use aptos_experimental_runtimes::thread_manager::THREAD_MANAGER;
use aptos_logger::{debug, warn};
use aptos_types::{
    aggregate_signature::AggregateSignature,
    randomness::{
        Delta, PKShare, ProofShare, RandKeys, RandMetadata, Randomness, WvufPP, APK, PK, WVUF,
    },
    validator_verifier::ValidatorVerifier,
};
use dashmap::DashSet;
use fail::fail_point;
use rayon::prelude::*;
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use sha3::{Digest, Sha3_256};
use std::{collections::HashSet, fmt::Debug, sync::Arc};

pub const NUM_THREADS_FOR_WVUF_DERIVATION: usize = 8;
pub const FUTURE_ROUNDS_TO_ACCEPT: u64 = 200;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub(super) struct MockShare;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub(super) struct MockAugData;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Share {
    share: ProofShare,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct AugmentedData {
    delta: Delta,
    fast_delta: Option<Delta>,
}

impl TShare for Share {
    fn verify(
        &self,
        rand_config: &RandConfig,
        rand_metadata: &RandMetadata,
        author: &Author,
    ) -> anyhow::Result<()> {
        let index = *rand_config
            .validator
            .address_to_validator_index()
            .get(author)
            .ok_or_else(|| anyhow!("Share::verify failed with unknown author"))?;
        let maybe_apk = &rand_config.keys.certified_apks[index];
        if let Some(apk) = maybe_apk.get() {
            WVUF::verify_share(
                &rand_config.vuf_pp,
                apk,
                bcs::to_bytes(&rand_metadata)
                    .map_err(|e| anyhow!("Serialization failed: {}", e))?
                    .as_slice(),
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

    #[allow(clippy::unwrap_used)]
    fn generate(rand_config: &RandConfig, rand_metadata: RandMetadata) -> RandShare<Self>
    where
        Self: Sized,
    {
        let share = Share {
            share: WVUF::create_share(
                &rand_config.keys.ask,
                bcs::to_bytes(&rand_metadata).unwrap().as_slice(),
            ),
        };
        fail_point!("consensus::rand::corrupt_share", |_| {
            let corrupted = Share {
                share: WVUF::create_share(&rand_config.keys.ask, b"corrupted_message"),
            };
            RandShare::new(rand_config.author(), rand_metadata.clone(), corrupted)
        });
        RandShare::new(rand_config.author(), rand_metadata, share)
    }

    /// Aggregate pre-verified shares into randomness.
    /// Callers must run `pre_aggregate_verify` first to remove invalid shares.
    fn aggregate<'a>(
        shares: impl Iterator<Item = &'a RandShare<Self>>,
        rand_config: &RandConfig,
        rand_metadata: RandMetadata,
    ) -> anyhow::Result<Randomness>
    where
        Self: Sized,
    {
        let timer = std::time::Instant::now();
        let shares_vec: Vec<_> = shares.collect();
        let apks_and_proofs = Self::build_apks_and_proofs(&shares_vec, rand_config)?;

        let proof = WVUF::aggregate_shares(&rand_config.wconfig, &apks_and_proofs);
        let metadata_serialized = bcs::to_bytes(&rand_metadata).map_err(|e| {
            anyhow!("Share::aggregate failed with metadata serialization error: {e}")
        })?;

        let eval = WVUF::derive_eval(
            &rand_config.wconfig,
            &rand_config.vuf_pp,
            metadata_serialized.as_slice(),
            &rand_config.get_all_certified_apk(),
            &proof,
            THREAD_MANAGER.get_non_exe_cpu_pool(),
        )
        .map_err(|e| anyhow!("Share::aggregate failed with WVUF derive_eval error: {e}"))?;
        debug!("WVUF derivation time: {} ms", timer.elapsed().as_millis());
        let eval_bytes = bcs::to_bytes(&eval)
            .map_err(|e| anyhow!("Share::aggregate failed with eval serialization error: {e}"))?;
        let rand_bytes = Sha3_256::digest(eval_bytes.as_slice()).to_vec();
        Ok(Randomness::new(rand_metadata, rand_bytes))
    }

    fn pre_aggregate_verify<'a>(
        shares: impl Iterator<Item = &'a RandShare<Self>>,
        rand_config: &RandConfig,
        rand_metadata: &RandMetadata,
    ) -> HashSet<Author> {
        if !rand_config.optimistic_rand_share_verification {
            return HashSet::new();
        }

        let shares_vec: Vec<_> = shares.collect();

        // Try batch verification: build proof and verify in one shot.
        // If any step fails, fall back to individual verification.
        match Self::build_apks_and_proofs(&shares_vec, rand_config).and_then(|apks_and_proofs| {
            let proof = WVUF::aggregate_shares(&rand_config.wconfig, &apks_and_proofs);
            let metadata_serialized = bcs::to_bytes(rand_metadata)
                .map_err(|e| anyhow!("metadata serialization failed: {e}"))?;
            WVUF::verify_proof(
                &rand_config.vuf_pp,
                rand_config.pk(),
                &rand_config.get_all_certified_apk(),
                metadata_serialized.as_slice(),
                &proof,
                THREAD_MANAGER.get_non_exe_cpu_pool(),
            )
        }) {
            Ok(()) => return HashSet::new(),
            Err(e) => {
                // Batch verification failed; fall back to individual verification
                warn!(
                    "Batch verification failed for round {}, falling back to individual verification: {e}",
                    rand_metadata.round
                );
            },
        }

        let verification_results: Vec<bool> = THREAD_MANAGER.get_non_exe_cpu_pool().install(|| {
            shares_vec
                .par_iter()
                .map(|share| {
                    share
                        .share
                        .verify(rand_config, rand_metadata, &share.author)
                        .is_ok()
                })
                .collect()
        });

        let mut bad_authors = HashSet::new();
        for (share, is_valid) in shares_vec.iter().zip(verification_results) {
            if !is_valid {
                warn!(
                    "Share from {} failed individual verification, adding to pessimistic set",
                    share.author
                );
                rand_config.add_to_pessimistic_set(share.author);
                bad_authors.insert(share.author);
            }
        }
        bad_authors
    }
}

impl Share {
    fn build_apks_and_proofs(
        shares: &[&RandShare<Self>],
        rand_config: &RandConfig,
    ) -> anyhow::Result<Vec<(Player, APK, ProofShare)>> {
        let mut apks_and_proofs = vec![];
        for share in shares {
            let id = rand_config
                .validator
                .address_to_validator_index()
                .get(share.author())
                .copied()
                .ok_or_else(|| {
                    anyhow!(
                        "Share::aggregate failed with invalid share author: {}",
                        share.author
                    )
                })?;
            let apk = rand_config
                .get_certified_apk(share.author())
                .ok_or_else(|| {
                    anyhow!(
                        "Share::aggregate failed with missing apk for share from {}",
                        share.author
                    )
                })?;
            apks_and_proofs.push((Player { id }, apk.clone(), share.share().share));
        }
        Ok(apks_and_proofs)
    }
}

impl TAugmentedData for AugmentedData {
    fn generate(rand_config: &RandConfig) -> AugData<Self>
    where
        Self: Sized,
    {
        let delta = rand_config.get_my_delta().clone();
        rand_config
            .add_certified_delta(&rand_config.author(), delta.clone())
            .expect("Add self delta should succeed");

        let data = AugmentedData {
            delta: delta.clone(),
            fast_delta: None,
        };
        AugData::new(rand_config.epoch(), rand_config.author(), data)
    }

    fn augment(
        &self,
        rand_config: &RandConfig,
        author: &Author,
    ) {
        let AugmentedData { delta, .. } = self;
        rand_config
            .add_certified_delta(author, delta.clone())
            .expect("Add delta should succeed");
    }

    fn verify(
        &self,
        rand_config: &RandConfig,
        author: &Author,
    ) -> anyhow::Result<()> {
        rand_config
            .derive_apk(author, self.delta.clone())
            .map(|_| ())?;
        Ok(())
    }
}

impl TShare for MockShare {
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
    ) -> anyhow::Result<Randomness>
    where
        Self: Sized,
    {
        Ok(Randomness::new(rand_metadata, vec![]))
    }
}

impl TAugmentedData for MockAugData {
    fn generate(rand_config: &RandConfig) -> AugData<Self>
    where
        Self: Sized,
    {
        AugData::new(rand_config.epoch(), rand_config.author(), Self)
    }

    fn augment(
        &self,
        _rand_config: &RandConfig,
        _author: &Author,
    ) {
    }

    fn verify(
        &self,
        _rand_config: &RandConfig,
        _author: &Author,
    ) -> anyhow::Result<()> {
        Ok(())
    }
}

pub trait TShare:
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
    ) -> anyhow::Result<Randomness>
    where
        Self: Sized;

    /// Verify shares before aggregation. Returns the set of authors whose shares
    /// failed verification and should be removed before spawning the aggregation task.
    fn pre_aggregate_verify<'a>(
        _shares: impl Iterator<Item = &'a RandShare<Self>>,
        _rand_config: &RandConfig,
        _rand_metadata: &RandMetadata,
    ) -> HashSet<Author>
    where
        Self: Sized,
    {
        HashSet::new()
    }
}

pub trait TAugmentedData:
    Clone + Debug + PartialEq + Send + Sync + Serialize + DeserializeOwned + 'static
{
    fn generate(rand_config: &RandConfig) -> AugData<Self>
    where
        Self: Sized;

    fn augment(
        &self,
        rand_config: &RandConfig,
        author: &Author,
    );

    fn verify(
        &self,
        rand_config: &RandConfig,
        author: &Author,
    ) -> anyhow::Result<()>;
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

impl<S: TShare> RandShare<S> {
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
        self.metadata.round
    }

    pub fn epoch(&self) -> u64 {
        self.metadata.epoch
    }

    pub fn verify(&self, rand_config: &RandConfig) -> anyhow::Result<()> {
        self.share.verify(rand_config, &self.metadata, &self.author)
    }

    pub fn optimistic_verify(&self, rand_config: &RandConfig) -> anyhow::Result<()> {
        if rand_config.should_verify_optimistically(&self.author) {
            // Still perform cheap structural checks (author exists, APK certified)
            // to prevent invalid shares from reaching aggregation where they would
            // cause build_apks_and_proofs to fail hard, bypassing the fallback.
            rand_config.verify_structural(&self.author)?;
            Ok(())
        } else {
            self.verify(rand_config) // pessimistic: verify individually
        }
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
    metadata: RandMetadata,
}

impl RequestShare {
    pub fn new(metadata: RandMetadata) -> Self {
        Self { metadata }
    }

    pub fn epoch(&self) -> u64 {
        self.metadata.epoch
    }

    pub fn rand_metadata(&self) -> &RandMetadata {
        &self.metadata
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

impl<D: TAugmentedData> AugData<D> {
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

    pub fn verify(
        &self,
        rand_config: &RandConfig,
        sender: Author,
    ) -> anyhow::Result<()> {
        ensure!(self.author == sender, "Invalid author");
        self.data
            .verify(rand_config, &self.author)?;
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

    pub fn verify<D: TAugmentedData>(
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

impl<D: TAugmentedData> CertifiedAugData<D> {
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

#[derive(Clone)]
pub struct RandConfig {
    author: Author,
    epoch: u64,
    validator: Arc<ValidatorVerifier>,
    // public parameters of the weighted VUF
    vuf_pp: WvufPP,
    // key shares for weighted VUF
    keys: Arc<RandKeys>,
    // weighted config for weighted VUF
    wconfig: WeightedConfigBlstrs,
    // aggregate public key
    pk: PK,
    // whether to skip per-share verification and defer to batch verification at aggregation
    optimistic_rand_share_verification: bool,
    // authors that have been caught sending bad shares; always verify individually
    pessimistic_verify_set: Arc<DashSet<Author>>,
}

impl Debug for RandConfig {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "RandConfig {{ epoch: {}, author: {}, wconfig: {:?} }}",
            self.epoch, self.author, self.wconfig
        )
    }
}

impl RandConfig {
    pub fn new(
        author: Author,
        epoch: u64,
        validator: Arc<ValidatorVerifier>,
        vuf_pp: WvufPP,
        keys: RandKeys,
        wconfig: WeightedConfigBlstrs,
        pk: PK,
        optimistic_rand_share_verification: bool,
    ) -> Self {
        Self {
            author,
            epoch,
            validator,
            vuf_pp,
            keys: Arc::new(keys),
            wconfig,
            pk,
            optimistic_rand_share_verification,
            pessimistic_verify_set: Arc::new(DashSet::new()),
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
            .expect("Peer should be in the index!")
    }

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
        let apk = WVUF::augment_pubkey(&self.vuf_pp, self.get_pk_share(peer).clone(), delta)?;
        Ok(apk)
    }

    pub fn add_certified_delta(&self, peer: &Author, delta: Delta) -> anyhow::Result<()> {
        let apk = self.derive_apk(peer, delta)?;
        self.add_certified_apk(peer, apk)?;
        Ok(())
    }

    pub fn get_my_delta(&self) -> &Delta {
        WVUF::get_public_delta(&self.keys.apk)
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

    pub fn should_verify_optimistically(&self, author: &Author) -> bool {
        self.optimistic_rand_share_verification && !self.pessimistic_verify_set.contains(author)
    }

    pub fn add_to_pessimistic_set(&self, author: Author) {
        self.pessimistic_verify_set.insert(author);
    }

    pub fn pk(&self) -> &PK {
        &self.pk
    }

    /// Cheap structural validation: author is a known validator and has a certified APK.
    /// This catches shares that would cause `build_apks_and_proofs` to fail hard,
    /// without performing the expensive cryptographic pairing check.
    pub fn verify_structural(&self, author: &Author) -> anyhow::Result<()> {
        let index = *self
            .validator
            .address_to_validator_index()
            .get(author)
            .ok_or_else(|| anyhow!("Structural check failed: unknown author {}", author))?;
        ensure!(
            self.keys.certified_apks[index].get().is_some(),
            "Structural check failed: no certified APK for author {}",
            author
        );
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use aptos_consensus_types::common::Author;
    use aptos_crypto::{bls12381, Uniform};
    use aptos_dkg::{
        pvss::{traits::TranscriptCore, Player, WeightedConfigBlstrs},
        weighted_vuf::traits::WeightedVUF,
    };
    use aptos_types::{
        dkg::{real_dkg::maybe_dk_from_bls_sk, DKGSessionMetadata, DKGTrait, DefaultDKG},
        on_chain_config::OnChainRandomnessConfig,
        randomness::{RandKeys, RandMetadata, WvufPP, WVUF},
        validator_verifier::{
            ValidatorConsensusInfo, ValidatorConsensusInfoMoveStruct, ValidatorVerifier,
        },
    };
    use rand::thread_rng;
    use std::str::FromStr;

    struct MultiValidatorTestContext {
        authors: Vec<Author>,
        rand_configs: Vec<RandConfig>,
        #[allow(dead_code)]
        pk: PK,
    }

    impl MultiValidatorTestContext {
        fn new(weights: Vec<u64>, optimistic: bool) -> Self {
            let target_epoch = 1;
            let num_validators = weights.len();
            let mut rng = thread_rng();
            let authors: Vec<_> = (0..num_validators)
                .map(|i| Author::from_str(&format!("{:x}", i)).unwrap())
                .collect();
            let private_keys: Vec<bls12381::PrivateKey> = (0..num_validators)
                .map(|_| bls12381::PrivateKey::generate_for_testing())
                .collect();
            let public_keys: Vec<bls12381::PublicKey> =
                private_keys.iter().map(bls12381::PublicKey::from).collect();
            let dkg_decrypt_keys: Vec<<DefaultDKG as DKGTrait>::NewValidatorDecryptKey> =
                private_keys
                    .iter()
                    .map(|sk| maybe_dk_from_bls_sk(sk).unwrap())
                    .collect();
            let consensus_infos: Vec<ValidatorConsensusInfo> = (0..num_validators)
                .map(|idx| {
                    ValidatorConsensusInfo::new(
                        authors[idx],
                        public_keys[idx].clone(),
                        weights[idx],
                    )
                })
                .collect();
            let consensus_info_move_structs = consensus_infos
                .clone()
                .into_iter()
                .map(ValidatorConsensusInfoMoveStruct::from)
                .collect::<Vec<_>>();
            let verifier = ValidatorVerifier::new(consensus_infos);
            let dkg_session_metadata = DKGSessionMetadata {
                dealer_epoch: 999,
                randomness_config: OnChainRandomnessConfig::default_enabled().into(),
                dealer_validator_set: consensus_info_move_structs.clone(),
                target_validator_set: consensus_info_move_structs,
            };
            let dkg_pub_params = DefaultDKG::new_public_params(&dkg_session_metadata);
            let input_secret = <DefaultDKG as DKGTrait>::InputSecret::generate_for_testing();
            let transcript = DefaultDKG::generate_transcript(
                &mut rng,
                &dkg_pub_params,
                &input_secret,
                0,
                &private_keys[0],
                &public_keys[0],
            );

            let pk_shares = (0..num_validators)
                .map(|id| {
                    transcript
                        .main
                        .get_public_key_share(&dkg_pub_params.pvss_config.wconfig, &Player { id })
                })
                .collect::<Vec<_>>();
            let vuf_pub_params = WvufPP::from(&dkg_pub_params.pvss_config.pp);

            let aggregate_pk = transcript.main.get_dealt_public_key();

            // Decrypt keys for ALL validators
            let mut asks = vec![];
            let mut apks = vec![];
            for (i, dk) in dkg_decrypt_keys.iter().enumerate() {
                let (sk, pk) = DefaultDKG::decrypt_secret_share_from_transcript(
                    &dkg_pub_params,
                    &transcript,
                    i as u64,
                    dk,
                )
                .unwrap();
                let (ask, apk) =
                    WVUF::augment_key_pair(&vuf_pub_params, sk.main, pk.main, &mut rng);
                asks.push(ask);
                apks.push(apk);
            }
            let verifier_arc: Arc<ValidatorVerifier> = verifier.into();

            let weight_usize: Vec<usize> = weights.into_iter().map(|x| x as usize).collect();
            let half_total_weights = weight_usize.iter().copied().sum::<usize>() / 2;
            let weighted_config =
                WeightedConfigBlstrs::new(half_total_weights, weight_usize).unwrap();

            // Build a RandConfig for each validator
            let mut rand_configs = vec![];
            for i in 0..num_validators {
                let keys = RandKeys::new(
                    asks[i].clone(),
                    apks[i].clone(),
                    pk_shares.clone(),
                    num_validators,
                );
                let config = RandConfig::new(
                    authors[i],
                    target_epoch,
                    verifier_arc.clone(),
                    vuf_pub_params.clone(),
                    keys,
                    weighted_config.clone(),
                    aggregate_pk.clone(),
                    optimistic,
                );
                rand_configs.push(config);
            }

            // Certify all APKs in each config
            for config in &rand_configs {
                for j in 0..num_validators {
                    config
                        .add_certified_apk(&authors[j], apks[j].clone())
                        .expect("add_certified_apk should succeed");
                }
            }

            Self {
                authors,
                rand_configs,
                pk: aggregate_pk,
            }
        }
    }

    #[test]
    fn test_optimistic_share_aggregate_happy_path() {
        let ctx = MultiValidatorTestContext::new(vec![1, 1, 1, 1], true);
        let metadata = RandMetadata { epoch: 1, round: 1 };

        // Generate real shares from 3 validators
        let shares: Vec<_> = (0..3)
            .map(|i| Share::generate(&ctx.rand_configs[i], metadata.clone()))
            .collect();

        let result = Share::aggregate(shares.iter(), &ctx.rand_configs[0], metadata);
        assert!(
            result.is_ok(),
            "Aggregation should succeed: {:?}",
            result.err()
        );

        // Pessimistic set should be empty (no fallback needed)
        assert!(
            ctx.rand_configs[0].pessimistic_verify_set.is_empty(),
            "No authors should be in the pessimistic set"
        );
    }

    #[test]
    fn test_optimistic_share_aggregate_with_bad_share() {
        let ctx = MultiValidatorTestContext::new(vec![1, 1, 1, 1, 1], true);
        let metadata = RandMetadata { epoch: 1, round: 1 };

        // Generate real shares from 3 validators
        let mut shares: Vec<_> = (0..3)
            .map(|i| Share::generate(&ctx.rand_configs[i], metadata.clone()))
            .collect();

        // Create a corrupted share: use validator 4's key but attribute to validator 3
        let bad_author = ctx.authors[3];
        let wrong_share = Share::generate(&ctx.rand_configs[4], metadata.clone());
        shares.push(RandShare::new(
            bad_author,
            metadata.clone(),
            wrong_share.share().clone(),
        ));

        // Pre-verify removes the bad share
        let bad_authors =
            Share::pre_aggregate_verify(shares.iter(), &ctx.rand_configs[0], &metadata);
        assert!(bad_authors.contains(&bad_author));

        // Aggregate with only valid shares
        let valid_shares: Vec<_> = shares
            .iter()
            .filter(|s| !bad_authors.contains(s.author()))
            .cloned()
            .collect();
        let result = Share::aggregate(valid_shares.iter(), &ctx.rand_configs[0], metadata);
        assert!(
            result.is_ok(),
            "Aggregation should succeed after pre-verify: {:?}",
            result.err()
        );

        // The bad author should be in the pessimistic set
        assert!(
            ctx.rand_configs[0]
                .pessimistic_verify_set
                .contains(&bad_author),
            "Bad author should be in the pessimistic set"
        );
    }

    #[test]
    fn test_optimistic_verify_skips_for_good_author() {
        let ctx = MultiValidatorTestContext::new(vec![1, 1, 1, 1], true);
        let metadata = RandMetadata { epoch: 1, round: 1 };

        // Generate a real share
        let share = Share::generate(&ctx.rand_configs[0], metadata);

        // optimistic_verify should succeed (deferred verification)
        assert!(share.optimistic_verify(&ctx.rand_configs[0]).is_ok());
    }

    #[test]
    fn test_pessimistic_verify_for_bad_author() {
        let ctx = MultiValidatorTestContext::new(vec![1, 1, 1, 1], true);
        let metadata = RandMetadata { epoch: 1, round: 1 };

        // Add author to pessimistic set
        let bad_author = ctx.authors[1];
        ctx.rand_configs[0].add_to_pessimistic_set(bad_author);

        // Create a corrupted share: use validator 2's key but attribute to bad_author (validator 1)
        let wrong_share = Share::generate(&ctx.rand_configs[2], metadata.clone());
        let share = RandShare::new(bad_author, metadata, wrong_share.share().clone());

        // optimistic_verify should fail (falls through to individual verification)
        assert!(share.optimistic_verify(&ctx.rand_configs[0]).is_err());
    }

    #[test]
    fn test_optimistic_disabled_verifies_individually() {
        let ctx = MultiValidatorTestContext::new(vec![1, 1, 1, 1], false);
        let metadata = RandMetadata { epoch: 1, round: 1 };

        // Create a corrupted share: use validator 1's key but attribute to validator 0
        let wrong_share = Share::generate(&ctx.rand_configs[1], metadata.clone());
        let share = RandShare::new(ctx.authors[0], metadata, wrong_share.share().clone());

        // With optimization disabled, every share is verified individually
        assert!(share.optimistic_verify(&ctx.rand_configs[0]).is_err());
    }

    #[test]
    fn test_optimistic_pre_verify_insufficient_valid_shares() {
        // 4 validators with equal weight; threshold is >50%, so need at least 3 valid shares.
        let ctx = MultiValidatorTestContext::new(vec![1, 1, 1, 1], true);
        let metadata = RandMetadata { epoch: 1, round: 1 };

        // Only 1 real share from validator 0
        let real_share = Share::generate(&ctx.rand_configs[0], metadata.clone());

        // 3 corrupted shares: use wrong keys for validators 1, 2, 3
        let bad1 = Share::generate(&ctx.rand_configs[2], metadata.clone());
        let bad2 = Share::generate(&ctx.rand_configs[3], metadata.clone());
        let bad3 = Share::generate(&ctx.rand_configs[0], metadata.clone());
        let shares = [
            real_share,
            RandShare::new(ctx.authors[1], metadata.clone(), bad1.share().clone()),
            RandShare::new(ctx.authors[2], metadata.clone(), bad2.share().clone()),
            RandShare::new(ctx.authors[3], metadata.clone(), bad3.share().clone()),
        ];

        // Pre-verify identifies 3 bad authors
        let bad_authors =
            Share::pre_aggregate_verify(shares.iter(), &ctx.rand_configs[0], &metadata);
        assert_eq!(bad_authors.len(), 3);

        // Only 1 valid share remains — below threshold, so caller should not aggregate
        let valid_weight: u64 = shares
            .iter()
            .filter(|s| !bad_authors.contains(s.author()))
            .map(|s| ctx.rand_configs[0].get_peer_weight(s.author()))
            .sum();
        assert!(
            valid_weight < ctx.rand_configs[0].threshold(),
            "Valid weight {} should be below threshold {}",
            valid_weight,
            ctx.rand_configs[0].threshold()
        );
    }

    #[test]
    fn test_pre_aggregate_verify_happy_path() {
        let ctx = MultiValidatorTestContext::new(vec![1, 1, 1, 1], true);
        let metadata = RandMetadata { epoch: 1, round: 1 };

        let shares: Vec<_> = (0..3)
            .map(|i| Share::generate(&ctx.rand_configs[i], metadata.clone()))
            .collect();

        let bad_authors =
            Share::pre_aggregate_verify(shares.iter(), &ctx.rand_configs[0], &metadata);
        assert!(bad_authors.is_empty(), "All shares are valid");
        assert!(ctx.rand_configs[0].pessimistic_verify_set.is_empty());
    }

    #[test]
    fn test_pre_aggregate_verify_with_bad_share() {
        let ctx = MultiValidatorTestContext::new(vec![1, 1, 1, 1, 1], true);
        let metadata = RandMetadata { epoch: 1, round: 1 };

        let mut shares: Vec<_> = (0..3)
            .map(|i| Share::generate(&ctx.rand_configs[i], metadata.clone()))
            .collect();

        // Corrupted share: validator 4's key attributed to validator 3
        let bad_author = ctx.authors[3];
        let wrong_share = Share::generate(&ctx.rand_configs[4], metadata.clone());
        shares.push(RandShare::new(
            bad_author,
            metadata.clone(),
            wrong_share.share().clone(),
        ));

        let bad_authors =
            Share::pre_aggregate_verify(shares.iter(), &ctx.rand_configs[0], &metadata);
        assert_eq!(bad_authors.len(), 1);
        assert!(bad_authors.contains(&bad_author));
        assert!(ctx.rand_configs[0]
            .pessimistic_verify_set
            .contains(&bad_author));
    }

    #[test]
    fn test_pre_aggregate_verify_non_optimistic() {
        let ctx = MultiValidatorTestContext::new(vec![1, 1, 1, 1, 1], false);
        let metadata = RandMetadata { epoch: 1, round: 1 };

        // Include a bad share — should still return empty since optimistic is off
        let mut shares: Vec<_> = (0..3)
            .map(|i| Share::generate(&ctx.rand_configs[i], metadata.clone()))
            .collect();
        let wrong_share = Share::generate(&ctx.rand_configs[4], metadata.clone());
        shares.push(RandShare::new(
            ctx.authors[3],
            metadata.clone(),
            wrong_share.share().clone(),
        ));

        let bad_authors =
            Share::pre_aggregate_verify(shares.iter(), &ctx.rand_configs[0], &metadata);
        assert!(
            bad_authors.is_empty(),
            "Non-optimistic mode skips pre_aggregate_verify"
        );
    }

    /// Test that the key pair DB format change is backward compatible.
    /// Old format: (AugKeyPair, Option<AugKeyPair>) where AugKeyPair = (ASK, APK)
    /// New format: AugKeyPair = (ASK, APK)
    /// The deserialization logic in epoch_manager.rs tries old format first,
    /// then new format, then falls through to regenerate.
    #[test]
    fn test_key_pair_serialization_backward_compat() {
        use aptos_types::randomness::{APK, ASK};

        let ctx = MultiValidatorTestContext::new(vec![1, 1, 1, 1], false);
        let keys = &ctx.rand_configs[0].keys;
        let ask = keys.ask.clone();
        let apk = keys.apk.clone();

        type AugKeyPair = (ASK, APK);

        let key_pair: AugKeyPair = (ask.clone(), apk.clone());

        // Old format: (main_key_pair, fast_key_pair) where fast is Option
        let old_bytes_with_fast =
            bcs::to_bytes(&(key_pair.clone(), Some(key_pair.clone()))).unwrap();
        let old_bytes_without_fast =
            bcs::to_bytes(&(key_pair.clone(), Option::<AugKeyPair>::None)).unwrap();

        // New format: just the key pair
        let new_bytes = bcs::to_bytes(&key_pair).unwrap();

        // Old format with fast=Some should deserialize via old path
        let (main, _fast): (AugKeyPair, Option<AugKeyPair>) =
            bcs::from_bytes(&old_bytes_with_fast).unwrap();
        assert_eq!(bcs::to_bytes(&main.0).unwrap(), bcs::to_bytes(&ask).unwrap());
        assert_eq!(bcs::to_bytes(&main.1).unwrap(), bcs::to_bytes(&apk).unwrap());

        // Old format with fast=None should deserialize via old path
        let (main, fast): (AugKeyPair, Option<AugKeyPair>) =
            bcs::from_bytes(&old_bytes_without_fast).unwrap();
        assert_eq!(bcs::to_bytes(&main.0).unwrap(), bcs::to_bytes(&ask).unwrap());
        assert_eq!(bcs::to_bytes(&main.1).unwrap(), bcs::to_bytes(&apk).unwrap());
        assert!(fast.is_none());

        // New format should deserialize via new path
        let recovered: AugKeyPair = bcs::from_bytes(&new_bytes).unwrap();
        assert_eq!(
            bcs::to_bytes(&recovered.0).unwrap(),
            bcs::to_bytes(&ask).unwrap()
        );
        assert_eq!(
            bcs::to_bytes(&recovered.1).unwrap(),
            bcs::to_bytes(&apk).unwrap()
        );

        // New format should NOT deserialize as old format (ensures we need the fallback)
        assert!(bcs::from_bytes::<(AugKeyPair, Option<AugKeyPair>)>(&new_bytes).is_err());

        // Old format should NOT deserialize as new format (ensures ordering matters)
        // Old-with-fast bytes contain extra data, so trying to parse as just AugKeyPair
        // would either fail or silently consume only the prefix. BCS is strict, so it
        // should fail on trailing bytes.
        assert!(bcs::from_bytes::<AugKeyPair>(&old_bytes_with_fast).is_err());
    }
}
