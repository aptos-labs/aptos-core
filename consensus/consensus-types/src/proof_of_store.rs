// Copyright (c) Aptos Foundation
// SPDX-License-Identifier: Innovation-Enabling Source Code License

use crate::{payload::TDataInfo, utils::PayloadTxnsSize};
use anyhow::{bail, ensure, Context};
use aptos_crypto::{bls12381, hash::CryptoHash, CryptoMaterialError, HashValue};
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
    fmt::{Debug, Display, Formatter},
    hash::Hash,
    ops::Deref,
};

pub trait TBatchInfo:
    Serialize + CryptoHash + Debug + Clone + Hash + Eq + PartialEq + Into<BatchInfoExt>
{
    fn epoch(&self) -> u64;

    fn expiration(&self) -> u64;

    fn num_txns(&self) -> u64;

    fn num_bytes(&self) -> u64;

    fn as_batch_info(&self) -> &BatchInfo;

    fn batch_id(&self) -> BatchId;

    fn author(&self) -> PeerId;

    fn digest(&self) -> &HashValue;

    fn gas_bucket_start(&self) -> u64;

    fn size(&self) -> PayloadTxnsSize;
}

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

impl From<BatchInfo> for BatchInfoExt {
    fn from(info: BatchInfo) -> Self {
        Self::V1 { info }
    }
}

impl TBatchInfo for BatchInfo {
    fn epoch(&self) -> u64 {
        self.epoch
    }

    fn expiration(&self) -> u64 {
        self.expiration
    }

    fn num_txns(&self) -> u64 {
        self.num_txns
    }

    fn num_bytes(&self) -> u64 {
        self.num_bytes
    }

    fn as_batch_info(&self) -> &BatchInfo {
        self
    }

    fn batch_id(&self) -> BatchId {
        self.batch_id
    }

    fn author(&self) -> PeerId {
        self.author
    }

    fn digest(&self) -> &HashValue {
        &self.digest
    }

    fn gas_bucket_start(&self) -> u64 {
        self.gas_bucket_start
    }

    fn size(&self) -> PayloadTxnsSize {
        PayloadTxnsSize::new(self.num_txns, self.num_bytes)
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

#[derive(
    Clone, Debug, Deserialize, Serialize, CryptoHasher, BCSCryptoHash, PartialEq, Eq, Hash,
)]
pub enum BatchInfoExt {
    V1 {
        info: BatchInfo,
    },
    V2 {
        info: BatchInfo,
        extra: ExtraBatchInfo,
    },
}

impl BatchInfoExt {
    pub fn new_v1(
        author: PeerId,
        batch_id: BatchId,
        epoch: u64,
        expiration: u64,
        digest: HashValue,
        num_txns: u64,
        num_bytes: u64,
        gas_bucket_start: u64,
    ) -> Self {
        Self::V1 {
            info: BatchInfo::new(
                author,
                batch_id,
                epoch,
                expiration,
                digest,
                num_txns,
                num_bytes,
                gas_bucket_start,
            ),
        }
    }

    pub fn new_v2(
        author: PeerId,
        batch_id: BatchId,
        epoch: u64,
        expiration: u64,
        digest: HashValue,
        num_txns: u64,
        num_bytes: u64,
        gas_bucket_start: u64,
        kind: BatchKind,
    ) -> Self {
        Self::V2 {
            info: BatchInfo::new(
                author,
                batch_id,
                epoch,
                expiration,
                digest,
                num_txns,
                num_bytes,
                gas_bucket_start,
            ),
            extra: ExtraBatchInfo { batch_kind: kind },
        }
    }

    pub fn info(&self) -> &BatchInfo {
        match self {
            BatchInfoExt::V1 { info } => info,
            BatchInfoExt::V2 { info, .. } => info,
        }
    }

    pub fn is_v2(&self) -> bool {
        matches!(self, Self::V2 { .. })
    }

    pub fn unpack_info(self) -> BatchInfo {
        match self {
            BatchInfoExt::V1 { info } => info,
            BatchInfoExt::V2 { info, .. } => info,
        }
    }
}

impl TBatchInfo for BatchInfoExt {
    fn epoch(&self) -> u64 {
        self.info().epoch()
    }

    fn expiration(&self) -> u64 {
        self.info().expiration()
    }

    fn num_txns(&self) -> u64 {
        self.info().num_txns()
    }

    fn num_bytes(&self) -> u64 {
        self.info().num_bytes()
    }

    fn as_batch_info(&self) -> &BatchInfo {
        self.info()
    }

    fn batch_id(&self) -> BatchId {
        self.info().batch_id()
    }

    fn author(&self) -> PeerId {
        self.info().author()
    }

    fn digest(&self) -> &HashValue {
        self.info().digest()
    }

    fn gas_bucket_start(&self) -> u64 {
        self.info().gas_bucket_start()
    }

    fn size(&self) -> PayloadTxnsSize {
        PayloadTxnsSize::new(self.info().num_txns(), self.info().num_bytes())
    }
}

impl TDataInfo for BatchInfoExt {
    fn num_txns(&self) -> u64 {
        self.info().num_txns()
    }

    fn num_bytes(&self) -> u64 {
        self.info().num_bytes()
    }

    fn info(&self) -> &BatchInfo {
        self.info()
    }

    fn signers(&self, _ordered_authors: &[PeerId]) -> Vec<PeerId> {
        vec![self.author()]
    }
}

#[derive(
    Clone, Debug, Deserialize, Serialize, CryptoHasher, BCSCryptoHash, PartialEq, Eq, Hash,
)]
pub struct ExtraBatchInfo {
    pub batch_kind: BatchKind,
}

#[derive(
    Clone, Debug, Deserialize, Serialize, CryptoHasher, BCSCryptoHash, PartialEq, Eq, Hash,
)]
pub enum BatchKind {
    Normal,
    Encrypted,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct SignedBatchInfoMsg<T> {
    signed_infos: Vec<SignedBatchInfo<T>>,
}

impl<T> SignedBatchInfoMsg<T>
where
    T: TBatchInfo,
{
    pub fn new(signed_infos: Vec<SignedBatchInfo<T>>) -> Self {
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

    pub fn take(self) -> Vec<SignedBatchInfo<T>> {
        self.signed_infos
    }
}

impl From<SignedBatchInfoMsg<BatchInfo>> for SignedBatchInfoMsg<BatchInfoExt> {
    fn from(info: SignedBatchInfoMsg<BatchInfo>) -> Self {
        Self {
            signed_infos: info
                .signed_infos
                .into_iter()
                .map(|signed_info| signed_info.into())
                .collect(),
        }
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct SignedBatchInfo<T> {
    info: T,
    signer: PeerId,
    signature: SignatureWithStatus,
}

impl<T> SignedBatchInfo<T>
where
    T: TBatchInfo,
{
    pub fn new(
        batch_info: T,
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
        batch_info: T,
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
    pub fn dummy(batch_info: T, signer: PeerId) -> Self {
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

    pub fn batch_info(&self) -> &T {
        &self.info
    }
}

impl<T> Deref for SignedBatchInfo<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.info
    }
}

impl From<SignedBatchInfo<BatchInfo>> for SignedBatchInfo<BatchInfoExt> {
    fn from(signed_batch_info: SignedBatchInfo<BatchInfo>) -> Self {
        let SignedBatchInfo {
            info,
            signer,
            signature,
        } = signed_batch_info;
        Self {
            info: info.into(),
            signer,
            signature,
        }
    }
}

impl TryFrom<SignedBatchInfo<BatchInfoExt>> for SignedBatchInfo<BatchInfo> {
    type Error = anyhow::Error;

    fn try_from(signed_batch_info: SignedBatchInfo<BatchInfoExt>) -> Result<Self, Self::Error> {
        ensure!(
            matches!(signed_batch_info.batch_info(), &BatchInfoExt::V1 { .. }),
            "Batch must be V1 type"
        );
        let SignedBatchInfo {
            info,
            signer,
            signature,
        } = signed_batch_info;
        Ok(Self {
            info: info.unpack_info(),
            signer,
            signature,
        })
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
pub struct ProofOfStoreMsg<T> {
    proofs: Vec<ProofOfStore<T>>,
}

impl<T> ProofOfStoreMsg<T>
where
    T: TBatchInfo + Send + Sync + 'static,
{
    pub fn new(proofs: Vec<ProofOfStore<T>>) -> Self {
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

    pub fn take(self) -> Vec<ProofOfStore<T>> {
        self.proofs
    }
}

impl From<ProofOfStoreMsg<BatchInfo>> for ProofOfStoreMsg<BatchInfoExt> {
    fn from(proof_msg: ProofOfStoreMsg<BatchInfo>) -> Self {
        Self {
            proofs: proof_msg
                .proofs
                .into_iter()
                .map(|proof| proof.into())
                .collect(),
        }
    }
}

pub type ProofCache = Cache<BatchInfoExt, AggregateSignature>;

#[derive(Deserialize, Serialize, Clone, Debug, PartialEq, Eq)]
pub struct ProofOfStore<T> {
    info: T,
    multi_signature: AggregateSignature,
}

impl<T> ProofOfStore<T>
where
    T: TBatchInfo + Send + Sync + 'static,
{
    pub fn new(info: T, multi_signature: AggregateSignature) -> Self {
        Self {
            info,
            multi_signature,
        }
    }

    pub fn verify(&self, validator: &ValidatorVerifier, cache: &ProofCache) -> anyhow::Result<()> {
        let batch_info_ext: BatchInfoExt = self.info.clone().into();
        if let Some(signature) = cache.get(&batch_info_ext) {
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
            cache.insert(batch_info_ext, self.multi_signature.clone());
        }
        result
    }

    pub fn shuffled_signers(&self, ordered_authors: &[PeerId]) -> Vec<PeerId> {
        let mut ret: Vec<PeerId> = self.multi_signature.get_signers_addresses(ordered_authors);
        ret.shuffle(&mut thread_rng());
        ret
    }

    pub fn info(&self) -> &T {
        &self.info
    }

    pub fn multi_signature(&self) -> &AggregateSignature {
        &self.multi_signature
    }

    pub fn unpack(self) -> (T, AggregateSignature) {
        (self.info, self.multi_signature)
    }
}

impl<T> Deref for ProofOfStore<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.info
    }
}

impl<T> TDataInfo for ProofOfStore<T>
where
    T: TBatchInfo + Send + Sync + 'static,
{
    fn num_txns(&self) -> u64 {
        self.info.num_txns()
    }

    fn num_bytes(&self) -> u64 {
        self.info.num_bytes()
    }

    fn info(&self) -> &BatchInfo {
        self.info.as_batch_info()
    }

    fn signers(&self, ordered_authors: &[PeerId]) -> Vec<PeerId> {
        self.shuffled_signers(ordered_authors)
    }
}

impl From<ProofOfStore<BatchInfo>> for ProofOfStore<BatchInfoExt> {
    fn from(proof: ProofOfStore<BatchInfo>) -> Self {
        let (info, sig) = proof.unpack();
        Self {
            info: info.into(),
            multi_signature: sig,
        }
    }
}
