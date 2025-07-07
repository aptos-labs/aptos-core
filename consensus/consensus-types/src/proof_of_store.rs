// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{payload::TDataInfo, utils::PayloadTxnsSize};
use anyhow::{bail, ensure, Context};
use aptos_crypto::{bls12381, CryptoMaterialError, HashValue};
use aptos_crypto_derive::{BCSCryptoHash, CryptoHasher};
use aptos_types::{
    aggregate_signature::AggregateSignature, ledger_info::SignatureWithStatus,
    quorum_store::BatchId, validator_signer::ValidatorSigner,
    validator_verifier::ValidatorVerifier, PeerId,
};
use mini_moka::sync::Cache;
use rand::{seq::SliceRandom, thread_rng};
use serde::{Deserialize, Serialize};
use std::{
    fmt::{Display, Formatter},
    hash::Hash,
    ops::Deref,
};

#[derive(
    Clone, Debug, Deserialize, Serialize, CryptoHasher, BCSCryptoHash, PartialEq, Eq, Hash,
)]
pub struct BatchInfo {
    author: PeerId,
    batch_id: BatchId,
    epoch: u64,
    expiration: u64,
    digest: HashValue,
    num_txns: u64,
    num_bytes: u64,
    gas_bucket_start: u64,
}

impl BatchInfo {
    pub fn new(
        author: PeerId,
        batch_id: BatchId,
        epoch: u64,
        expiration: u64,
        digest: HashValue,
        num_txns: u64,
        num_bytes: u64,
        gas_bucket_start: u64,
    ) -> Self {
        Self {
            author,
            batch_id,
            epoch,
            expiration,
            digest,
            num_txns,
            num_bytes,
            gas_bucket_start,
        }
    }

    pub fn epoch(&self) -> u64 {
        self.epoch
    }

    pub fn author(&self) -> PeerId {
        self.author
    }

    pub fn batch_id(&self) -> BatchId {
        self.batch_id
    }

    pub fn expiration(&self) -> u64 {
        self.expiration
    }

    pub fn digest(&self) -> &HashValue {
        &self.digest
    }

    pub fn num_txns(&self) -> u64 {
        self.num_txns
    }

    pub fn num_bytes(&self) -> u64 {
        self.num_bytes
    }

    pub fn size(&self) -> PayloadTxnsSize {
        PayloadTxnsSize::new(self.num_txns, self.num_bytes)
    }

    pub fn gas_bucket_start(&self) -> u64 {
        self.gas_bucket_start
    }
}

impl Display for BatchInfo {
    fn fmt(&self, f: &mut Formatter) -> std::fmt::Result {
        write!(f, "({}:{}:{})", self.author, self.batch_id, self.digest)
    }
}

impl TDataInfo for BatchInfo {
    fn num_txns(&self) -> u64 {
        self.num_txns()
    }

    fn num_bytes(&self) -> u64 {
        self.num_bytes()
    }

    fn info(&self) -> &BatchInfo {
        self
    }

    fn signers(&self, _ordered_authors: &[PeerId]) -> Vec<PeerId> {
        vec![self.author()]
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct SignedBatchInfoMsg {
    signed_infos: Vec<SignedBatchInfo>,
}

impl SignedBatchInfoMsg {
    pub fn new(signed_infos: Vec<SignedBatchInfo>) -> Self {
        Self { signed_infos }
    }

    pub fn verify(
        &self,
        sender: PeerId,
        max_num_batches: usize,
        max_batch_expiry_gap_usecs: u64,
        validator: &ValidatorVerifier,
    ) -> anyhow::Result<()> {
        ensure!(!self.signed_infos.is_empty(), "Empty message");
        ensure!(
            self.signed_infos.len() <= max_num_batches,
            "Too many batches: {} > {}",
            self.signed_infos.len(),
            max_num_batches
        );
        for signed_info in &self.signed_infos {
            signed_info.verify(sender, max_batch_expiry_gap_usecs, validator)?
        }
        Ok(())
    }

    pub fn epoch(&self) -> anyhow::Result<u64> {
        ensure!(!self.signed_infos.is_empty(), "Empty message");
        let epoch = self.signed_infos[0].epoch();
        for info in self.signed_infos.iter() {
            ensure!(
                info.epoch() == epoch,
                "Epoch mismatch: {} != {}",
                info.epoch(),
                epoch
            );
        }
        Ok(epoch)
    }

    pub fn take(self) -> Vec<SignedBatchInfo> {
        self.signed_infos
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct SignedBatchInfo {
    info: BatchInfo,
    signer: PeerId,
    signature: SignatureWithStatus,
}

impl SignedBatchInfo {
    pub fn new(
        batch_info: BatchInfo,
        validator_signer: &ValidatorSigner,
    ) -> Result<Self, CryptoMaterialError> {
        let signature = validator_signer.sign(&batch_info)?;

        Ok(Self {
            info: batch_info,
            signer: validator_signer.author(),
            signature: SignatureWithStatus::from(signature),
        })
    }

    pub fn new_with_signature(
        batch_info: BatchInfo,
        signer: PeerId,
        signature: bls12381::Signature,
    ) -> Self {
        Self {
            info: batch_info,
            signer,
            signature: SignatureWithStatus::from(signature),
        }
    }

    #[cfg(any(test, feature = "fuzzing"))]
    pub fn dummy(batch_info: BatchInfo, signer: PeerId) -> Self {
        Self::new_with_signature(batch_info, signer, bls12381::Signature::dummy_signature())
    }

    pub fn signer(&self) -> PeerId {
        self.signer
    }

    pub fn verify(
        &self,
        sender: PeerId,
        max_batch_expiry_gap_usecs: u64,
        validator: &ValidatorVerifier,
    ) -> anyhow::Result<()> {
        if sender != self.signer {
            bail!("Sender {} mismatch signer {}", sender, self.signer);
        }

        if self.expiration()
            > aptos_infallible::duration_since_epoch().as_micros() as u64
                + max_batch_expiry_gap_usecs
        {
            bail!(
                "Batch expiration too far in future: {} > {}",
                self.expiration(),
                aptos_infallible::duration_since_epoch().as_micros() as u64
                    + max_batch_expiry_gap_usecs
            );
        }

        Ok(validator.optimistic_verify(self.signer, &self.info, &self.signature)?)
    }

    pub fn signature(&self) -> &bls12381::Signature {
        self.signature.signature()
    }

    pub fn signature_with_status(&self) -> &SignatureWithStatus {
        &self.signature
    }

    pub fn batch_info(&self) -> &BatchInfo {
        &self.info
    }
}

impl Deref for SignedBatchInfo {
    type Target = BatchInfo;

    fn deref(&self) -> &Self::Target {
        &self.info
    }
}

#[derive(Debug, PartialEq)]
pub enum SignedBatchInfoError {
    WrongAuthor,
    WrongInfo((u64, u64)),
    DuplicatedSignature,
    InvalidAuthor,
    NotFound,
    AlreadyCommitted,
    NoTimeStamps,
    UnableToAggregate,
}

#[derive(Deserialize, Serialize, Clone, Debug, PartialEq, Eq)]
pub struct ProofOfStoreMsg {
    proofs: Vec<ProofOfStore>,
}

impl ProofOfStoreMsg {
    pub fn new(proofs: Vec<ProofOfStore>) -> Self {
        Self { proofs }
    }

    pub fn verify(
        &self,
        max_num_proofs: usize,
        validator: &ValidatorVerifier,
        cache: &ProofCache,
    ) -> anyhow::Result<()> {
        ensure!(!self.proofs.is_empty(), "Empty message");
        ensure!(
            self.proofs.len() <= max_num_proofs,
            "Too many proofs: {} > {}",
            self.proofs.len(),
            max_num_proofs
        );
        for proof in &self.proofs {
            proof.verify(validator, cache)?
        }
        Ok(())
    }

    pub fn epoch(&self) -> anyhow::Result<u64> {
        ensure!(!self.proofs.is_empty(), "Empty message");
        let epoch = self.proofs[0].epoch();
        for proof in self.proofs.iter() {
            ensure!(
                proof.epoch() == epoch,
                "Epoch mismatch: {} != {}",
                proof.epoch(),
                epoch
            );
        }
        Ok(epoch)
    }

    pub fn take(self) -> Vec<ProofOfStore> {
        self.proofs
    }
}

pub type ProofCache = Cache<BatchInfo, AggregateSignature>;

#[derive(Deserialize, Serialize, Clone, Debug, PartialEq, Eq)]
pub struct ProofOfStore {
    info: BatchInfo,
    multi_signature: AggregateSignature,
}

impl ProofOfStore {
    pub fn new(info: BatchInfo, multi_signature: AggregateSignature) -> Self {
        Self {
            info,
            multi_signature,
        }
    }

    pub fn verify(&self, validator: &ValidatorVerifier, cache: &ProofCache) -> anyhow::Result<()> {
        if let Some(signature) = cache.get(&self.info) {
            if signature == self.multi_signature {
                return Ok(());
            }
        }
        let result = validator
            .verify_multi_signatures(&self.info, &self.multi_signature)
            .context(format!(
                "Failed to verify ProofOfStore for batch: {:?}",
                self.info
            ));
        if result.is_ok() {
            cache.insert(self.info.clone(), self.multi_signature.clone());
        }
        result
    }

    pub fn shuffled_signers(&self, ordered_authors: &[PeerId]) -> Vec<PeerId> {
        let mut ret: Vec<PeerId> = self.multi_signature.get_signers_addresses(ordered_authors);
        ret.shuffle(&mut thread_rng());
        ret
    }

    pub fn info(&self) -> &BatchInfo {
        &self.info
    }

    pub fn multi_signature(&self) -> &AggregateSignature {
        &self.multi_signature
    }
}

impl Deref for ProofOfStore {
    type Target = BatchInfo;

    fn deref(&self) -> &Self::Target {
        &self.info
    }
}

impl TDataInfo for ProofOfStore {
    fn num_txns(&self) -> u64 {
        self.num_txns
    }

    fn num_bytes(&self) -> u64 {
        self.num_bytes
    }

    fn info(&self) -> &BatchInfo {
        self.info()
    }

    fn signers(&self, ordered_authors: &[PeerId]) -> Vec<PeerId> {
        self.shuffled_signers(ordered_authors)
    }
}
