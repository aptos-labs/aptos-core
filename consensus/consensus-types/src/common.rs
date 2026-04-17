// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use crate::{
    payload::{OptBatches, OptQuorumStorePayload, PayloadExecutionLimit, TxnAndGasLimits},
    proof_of_store::{BatchInfoExt, BatchKind, ProofCache, ProofOfStore, TBatchInfo},
};
use anyhow::{bail, ensure, Context};
use aptos_crypto::{
    hash::{CryptoHash, CryptoHasher},
    HashValue,
};
use aptos_crypto_derive::CryptoHasher;
use aptos_logger::prelude::*;
use aptos_types::{
    account_address::AccountAddress,
    transaction::{ReplayProtector, SignedTransaction},
    validator_verifier::ValidatorVerifier,
    vm_status::DiscardedVMStatus,
    PeerId,
};
use once_cell::sync::OnceCell;
use rayon::prelude::*;
use serde::{Deserialize, Serialize};
use std::{
    collections::HashSet,
    fmt::{self, Write},
};

/// The round of a block is a consensus-internal counter, which starts with 0 and increases
/// monotonically. It is used for the protocol safety and liveness (please see the detailed
/// protocol description).
pub type Round = u64;
/// Author refers to the author's account address
pub type Author = AccountAddress;

#[derive(Clone, Copy, Debug, Eq, PartialEq, Deserialize, Serialize, Hash, Ord, PartialOrd)]
pub struct TransactionSummary {
    pub sender: AccountAddress,
    pub replay_protector: ReplayProtector,
    pub hash: HashValue,
}

impl TransactionSummary {
    pub fn new(sender: AccountAddress, replay_protector: ReplayProtector, hash: HashValue) -> Self {
        Self {
            sender,
            replay_protector,
            hash,
        }
    }
}

impl fmt::Display for TransactionSummary {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}:{}", self.sender, self.replay_protector,)
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Deserialize, Serialize, Hash, Ord, PartialOrd)]
pub struct TxnSummaryWithExpiration {
    pub sender: AccountAddress,
    pub replay_protector: ReplayProtector,
    pub expiration_timestamp_secs: u64,
    pub hash: HashValue,
}

impl TxnSummaryWithExpiration {
    pub fn new(
        sender: AccountAddress,
        replay_protector: ReplayProtector,
        expiration_timestamp_secs: u64,
        hash: HashValue,
    ) -> Self {
        Self {
            sender,
            replay_protector,
            expiration_timestamp_secs,
            hash,
        }
    }
}

impl fmt::Display for TxnSummaryWithExpiration {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}:{:?}", self.sender, self.replay_protector,)
    }
}

#[derive(Clone)]
pub struct TransactionInProgress {
    pub gas_unit_price: u64,
    pub count: u64,
}

impl TransactionInProgress {
    pub fn new(gas_unit_price: u64) -> Self {
        Self {
            gas_unit_price,
            count: 0,
        }
    }

    pub fn gas_unit_price(&self) -> u64 {
        self.gas_unit_price
    }

    pub fn decrement(&mut self) -> u64 {
        self.count -= 1;
        self.count
    }

    pub fn increment(&mut self) -> u64 {
        self.count += 1;
        self.count
    }
}

#[derive(Clone)]
pub struct RejectedTransactionSummary {
    pub sender: AccountAddress,
    pub replay_protector: ReplayProtector,
    pub hash: HashValue,
    pub reason: DiscardedVMStatus,
}

/// Verify that transactions match the expected BatchKind:
/// - Encrypted: all txns must be encrypted with valid ciphertext
/// - Normal: no txns may be encrypted
/// - None (V1): no encrypted txns allowed
pub fn verify_batch_kind_transactions(
    kind: Option<BatchKind>,
    txns: &[SignedTransaction],
) -> anyhow::Result<()> {
    match kind {
        Some(BatchKind::Encrypted) => {
            txns.par_iter()
                .with_min_len(24)
                .try_for_each(|txn| -> anyhow::Result<()> {
                    ensure!(
                        txn.is_encrypted_txn(),
                        "Encrypted batch contains non-encrypted transaction"
                    );
                    let auth_key = txn.authenticator().sender().authentication_key().context(
                        "Encrypted transactions are not supported with this authenticator type",
                    )?;
                    txn.payload()
                        .as_encrypted_payload()
                        .expect("already verified is_encrypted_txn")
                        .verify(txn.sender(), auth_key)
                        .context("Encrypted transaction ciphertext verification failed")
                })?;
        },
        Some(BatchKind::Normal) => {
            for txn in txns {
                ensure!(
                    !txn.is_encrypted_txn(),
                    "Normal batch contains encrypted transaction"
                );
            }
        },
        None => {
            // V1 batches do not support encrypted transactions
            for txn in txns {
                ensure!(
                    !txn.is_encrypted_txn(),
                    "V1 batch contains encrypted transaction"
                );
            }
        },
    }
    Ok(())
}

/// Validates that a single batch's num_txns and num_bytes are within receiver-side limits.
pub fn verify_batch_info_limits<T: TBatchInfo>(
    batch: &T,
    max_batch_txns: u64,
    max_batch_bytes: u64,
) -> anyhow::Result<()> {
    ensure!(
        batch.num_txns() <= max_batch_txns,
        "Batch txn count {} exceeds limit {}",
        batch.num_txns(),
        max_batch_txns,
    );
    ensure!(
        batch.num_bytes() <= max_batch_bytes,
        "Batch byte count {} exceeds limit {}",
        batch.num_bytes(),
        max_batch_bytes,
    );
    Ok(())
}

/// The payload in block.
#[derive(Deserialize, Serialize, Clone, Debug, PartialEq, Eq)]
pub enum Payload {
    DirectMempool(Vec<SignedTransaction>),
    /// Deprecated: Do not use.
    #[doc(hidden)]
    DeprecatedInQuorumStore(()),
    /// Deprecated: Do not use.
    #[doc(hidden)]
    DeprecatedInQuorumStoreWithLimit(()),
    /// Deprecated: Do not use.
    #[doc(hidden)]
    DeprecatedQuorumStoreInlineHybrid(()),
    OptQuorumStore(OptQuorumStorePayload),
    /// Deprecated: Do not use.
    #[doc(hidden)]
    DeprecatedQuorumStoreInlineHybridV2(()),
}

impl Payload {
    pub fn transform_to_quorum_store_v2(
        self,
        max_txns_to_execute: Option<u64>,
        block_gas_limit_override: Option<u64>,
    ) -> Self {
        match self {
            Payload::DirectMempool(_) => self,
            Payload::OptQuorumStore(mut opt_qs_payload) => {
                opt_qs_payload.set_execution_limit(PayloadExecutionLimit::TxnAndGasLimits(
                    TxnAndGasLimits {
                        transaction_limit: max_txns_to_execute,
                        gas_limit: block_gas_limit_override,
                    },
                ));
                Payload::OptQuorumStore(opt_qs_payload)
            },
            // Deprecated variants - return self unchanged
            Payload::DeprecatedInQuorumStore(_)
            | Payload::DeprecatedInQuorumStoreWithLimit(_)
            | Payload::DeprecatedQuorumStoreInlineHybrid(..)
            | Payload::DeprecatedQuorumStoreInlineHybridV2(..) => unreachable!(),
        }
    }

    pub fn empty(quorum_store_enabled: bool) -> Self {
        if quorum_store_enabled {
            Payload::OptQuorumStore(OptQuorumStorePayload::empty())
        } else {
            Payload::DirectMempool(Vec::new())
        }
    }

    pub fn len(&self) -> usize {
        match self {
            Payload::DirectMempool(txns) => txns.len(),
            Payload::OptQuorumStore(opt_qs_payload) => opt_qs_payload.num_txns(),
            // Deprecated variants - return 0
            Payload::DeprecatedInQuorumStore(_)
            | Payload::DeprecatedInQuorumStoreWithLimit(_)
            | Payload::DeprecatedQuorumStoreInlineHybrid(..)
            | Payload::DeprecatedQuorumStoreInlineHybridV2(..) => unreachable!(),
        }
    }

    pub fn len_for_execution(&self) -> u64 {
        match self {
            Payload::DirectMempool(txns) => txns.len() as u64,
            Payload::OptQuorumStore(opt_qs_payload) => {
                let num_txns = opt_qs_payload.num_txns() as u64;
                let max_txns_to_execute = opt_qs_payload.max_txns_to_execute().unwrap_or(u64::MAX);
                num_txns.min(max_txns_to_execute)
            },
            // Deprecated variants - return 0
            Payload::DeprecatedInQuorumStore(_)
            | Payload::DeprecatedInQuorumStoreWithLimit(_)
            | Payload::DeprecatedQuorumStoreInlineHybrid(..)
            | Payload::DeprecatedQuorumStoreInlineHybridV2(..) => unreachable!(),
        }
    }

    pub fn is_empty(&self) -> bool {
        match self {
            Payload::DirectMempool(txns) => txns.is_empty(),
            Payload::OptQuorumStore(opt_qs_payload) => opt_qs_payload.is_empty(),
            // Deprecated variants - return true (empty)
            Payload::DeprecatedInQuorumStore(_)
            | Payload::DeprecatedInQuorumStoreWithLimit(_)
            | Payload::DeprecatedQuorumStoreInlineHybrid(..)
            | Payload::DeprecatedQuorumStoreInlineHybridV2(..) => unreachable!(),
        }
    }

    pub fn extend(self, other: Payload) -> Self {
        match (self, other) {
            (Payload::DirectMempool(mut v1), Payload::DirectMempool(v2)) => {
                v1.extend(v2);
                Payload::DirectMempool(v1)
            },
            (Payload::OptQuorumStore(opt_qs1), Payload::OptQuorumStore(opt_qs2)) => {
                Payload::OptQuorumStore(opt_qs1.extend(opt_qs2))
            },
            // Deprecated or incompatible - return self
            (s, _) => s,
        }
    }

    pub fn is_direct(&self) -> bool {
        matches!(self, Payload::DirectMempool(_))
    }

    pub fn is_quorum_store(&self) -> bool {
        !matches!(self, Payload::DirectMempool(_))
    }

    /// This is potentially computationally expensive
    pub fn size(&self) -> usize {
        match self {
            Payload::DirectMempool(txns) => txns
                .par_iter()
                .with_min_len(100)
                .map(|txn| txn.raw_txn_bytes_len())
                .sum(),
            Payload::OptQuorumStore(opt_qs_payload) => opt_qs_payload.num_bytes(),
            // Deprecated variants - return 0 size
            Payload::DeprecatedInQuorumStore(_)
            | Payload::DeprecatedInQuorumStoreWithLimit(_)
            | Payload::DeprecatedQuorumStoreInlineHybrid(..)
            | Payload::DeprecatedQuorumStoreInlineHybridV2(..) => unreachable!(),
        }
    }

    fn verify_with_cache<T>(
        proofs: &[ProofOfStore<T>],
        validator: &ValidatorVerifier,
        proof_cache: &ProofCache,
    ) -> anyhow::Result<()>
    where
        T: TBatchInfo + Send + Sync + 'static,
        BatchInfoExt: From<T>,
    {
        let unverified: Vec<_> = proofs
            .iter()
            .filter(|proof| {
                proof_cache
                    .get(&BatchInfoExt::from(proof.info().clone()))
                    .is_none_or(|cached_proof| cached_proof != *proof.multi_signature())
            })
            .collect();
        unverified
            .par_iter()
            .with_min_len(2)
            .try_for_each(|proof| proof.verify(validator, proof_cache))?;
        Ok(())
    }

    pub fn verify_inline_batches<'a, T: TBatchInfo + 'a>(
        inline_batches: impl Iterator<Item = (&'a T, &'a Vec<SignedTransaction>)>,
        max_batch_txns: u64,
        max_batch_bytes: u64,
    ) -> anyhow::Result<()> {
        for (batch, payload) in inline_batches {
            verify_batch_info_limits(batch, max_batch_txns, max_batch_bytes)?;
            // TODO: Can cloning be avoided here?
            let computed_digest = BatchPayload::new(batch.author(), payload.clone()).hash();
            ensure!(
                computed_digest == *batch.digest(),
                "Hash of the received inline batch doesn't match the digest value for batch {:?}: {} != {}",
                batch,
                computed_digest,
                batch.digest()
            );
            verify_batch_kind_transactions(batch.batch_kind(), payload)?;
        }
        Ok(())
    }

    pub fn verify_opt_batches<T: TBatchInfo>(
        verifier: &ValidatorVerifier,
        opt_batches: &OptBatches<T>,
        max_batch_txns: u64,
        max_batch_bytes: u64,
    ) -> anyhow::Result<()> {
        let authors = verifier.address_to_validator_index();
        for batch in &opt_batches.batch_summary {
            verify_batch_info_limits(batch, max_batch_txns, max_batch_bytes)?;
            ensure!(
                authors.contains_key(&batch.author()),
                "Invalid author {} for batch {}",
                batch.author(),
                batch.digest()
            );
        }
        Ok(())
    }

    pub fn verify(
        &self,
        verifier: &ValidatorVerifier,
        proof_cache: &ProofCache,
        quorum_store_enabled: bool,
        opt_qs_v2_rx_enabled: bool,
        max_batch_txns: u64,
        max_batch_bytes: u64,
    ) -> anyhow::Result<()> {
        match (quorum_store_enabled, self) {
            (false, Payload::DirectMempool(_)) => Ok(()),
            (true, Payload::OptQuorumStore(OptQuorumStorePayload::V1(p))) => {
                let proof_with_data = p.proof_with_data();
                Self::verify_with_cache(&proof_with_data.batch_summary, verifier, proof_cache)?;
                for proof in &proof_with_data.batch_summary {
                    verify_batch_info_limits(proof.info(), max_batch_txns, max_batch_bytes)?;
                }
                Self::verify_inline_batches(
                    p.inline_batches()
                        .iter()
                        .map(|batch| (batch.info(), batch.transactions())),
                    max_batch_txns,
                    max_batch_bytes,
                )?;
                Self::verify_opt_batches(
                    verifier,
                    p.opt_batches(),
                    max_batch_txns,
                    max_batch_bytes,
                )?;
                Ok(())
            },
            (true, Payload::OptQuorumStore(OptQuorumStorePayload::V2(p))) => {
                ensure!(
                    opt_qs_v2_rx_enabled,
                    "OptQuorumStorePayload::V2 cannot be accepted yet"
                );
                let proof_with_data = p.proof_with_data();
                Self::verify_with_cache(&proof_with_data.batch_summary, verifier, proof_cache)?;
                for proof in &proof_with_data.batch_summary {
                    verify_batch_info_limits(proof.info(), max_batch_txns, max_batch_bytes)?;
                }
                Self::verify_inline_batches(
                    p.inline_batches()
                        .iter()
                        .map(|batch| (batch.info(), batch.transactions())),
                    max_batch_txns,
                    max_batch_bytes,
                )?;
                Self::verify_opt_batches(
                    verifier,
                    p.opt_batches(),
                    max_batch_txns,
                    max_batch_bytes,
                )?;
                Ok(())
            },
            (_, _) => Err(anyhow::anyhow!(
                "Wrong payload type. quorum_store_enabled={}, payload={}",
                quorum_store_enabled,
                self
            )),
        }
    }

    pub(crate) fn verify_epoch(&self, epoch: u64) -> anyhow::Result<()> {
        match self {
            Payload::DirectMempool(_) => Ok(()),
            Payload::OptQuorumStore(opt_quorum_store_payload) => {
                opt_quorum_store_payload.check_epoch(epoch)
            },
            // Deprecated variants - skip epoch verification
            Payload::DeprecatedInQuorumStore(_)
            | Payload::DeprecatedInQuorumStoreWithLimit(_)
            | Payload::DeprecatedQuorumStoreInlineHybrid(..)
            | Payload::DeprecatedQuorumStoreInlineHybridV2(..) => {
                bail!("Unsupported payload type {}", self)
            },
        }
    }
}

impl fmt::Display for Payload {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Payload::DirectMempool(txns) => {
                write!(f, "DirectMempool(txns: {})", txns.len())
            },
            Payload::OptQuorumStore(opt_quorum_store) => {
                write!(f, "{}", opt_quorum_store)
            },
            Payload::DeprecatedInQuorumStore(_) => write!(f, "DeprecatedInQuorumStore"),
            Payload::DeprecatedInQuorumStoreWithLimit(_) => {
                write!(f, "DeprecatedInQuorumStoreWithLimit")
            },
            Payload::DeprecatedQuorumStoreInlineHybrid(..) => {
                write!(f, "DeprecatedQuorumStoreInlineHybrid")
            },
            Payload::DeprecatedQuorumStoreInlineHybridV2(..) => {
                write!(f, "DeprecatedQuorumStoreInlineHybridV2")
            },
        }
    }
}

#[derive(Clone, Debug, Deserialize, Serialize, CryptoHasher)]
pub struct BatchPayload {
    author: PeerId,
    txns: Vec<SignedTransaction>,
    #[serde(skip)]
    num_bytes: OnceCell<usize>,
}

impl CryptoHash for BatchPayload {
    type Hasher = BatchPayloadHasher;

    fn hash(&self) -> HashValue {
        let mut state = Self::Hasher::new();
        let bytes = bcs::to_bytes(&self).expect("Unable to serialize batch payload");
        self.num_bytes.get_or_init(|| bytes.len());
        state.update(&bytes);
        state.finish()
    }
}

impl BatchPayload {
    pub fn new(author: PeerId, txns: Vec<SignedTransaction>) -> Self {
        Self {
            author,
            txns,
            num_bytes: OnceCell::new(),
        }
    }

    pub fn into_transactions(self) -> Vec<SignedTransaction> {
        self.txns
    }

    pub fn txns(&self) -> &Vec<SignedTransaction> {
        &self.txns
    }

    pub fn num_txns(&self) -> usize {
        self.txns.len()
    }

    pub fn num_bytes(&self) -> usize {
        *self
            .num_bytes
            .get_or_init(|| bcs::serialized_size(&self).expect("unable to serialize batch payload"))
    }

    pub fn author(&self) -> PeerId {
        self.author
    }
}

/// The payload to filter.
#[derive(Deserialize, Serialize, Clone, Debug, PartialEq, Eq)]
pub enum PayloadFilter {
    DirectMempool(Vec<TransactionSummary>),
    InQuorumStore(HashSet<BatchInfoExt>),
    Empty,
}

impl From<&Vec<&Payload>> for PayloadFilter {
    fn from(exclude_payloads: &Vec<&Payload>) -> Self {
        if exclude_payloads.is_empty() {
            return PayloadFilter::Empty;
        }
        let direct_mode = exclude_payloads.iter().any(|payload| payload.is_direct());

        if direct_mode {
            let mut exclude_txns = Vec::new();
            for payload in exclude_payloads {
                if let Payload::DirectMempool(txns) = payload {
                    for txn in txns {
                        exclude_txns.push(TransactionSummary {
                            sender: txn.sender(),
                            replay_protector: txn.replay_protector(),
                            hash: txn.committed_hash(),
                        });
                    }
                }
            }
            PayloadFilter::DirectMempool(exclude_txns)
        } else {
            let mut exclude_batches = HashSet::new();
            for payload in exclude_payloads {
                match payload {
                    Payload::DirectMempool(_) => {
                        error!("DirectMempool payload in InQuorumStore filter");
                    },
                    Payload::OptQuorumStore(OptQuorumStorePayload::V1(p)) => {
                        for batch in p.inline_batches().iter() {
                            exclude_batches.insert(batch.info().clone().into());
                        }
                        for batch_info in &p.opt_batches().batch_summary {
                            exclude_batches.insert(batch_info.clone().into());
                        }
                        for proof in &p.proof_with_data().batch_summary {
                            exclude_batches.insert(proof.info().clone().into());
                        }
                    },
                    Payload::OptQuorumStore(OptQuorumStorePayload::V2(p)) => {
                        for batch in p.inline_batches().iter() {
                            exclude_batches.insert(batch.info().clone());
                        }
                        for batch_info in &p.opt_batches().batch_summary {
                            exclude_batches.insert(batch_info.clone());
                        }
                        for proof in &p.proof_with_data().batch_summary {
                            exclude_batches.insert(proof.info().clone());
                        }
                    },
                    // Deprecated variants - skip (no batches to exclude)
                    Payload::DeprecatedInQuorumStore(_)
                    | Payload::DeprecatedInQuorumStoreWithLimit(_)
                    | Payload::DeprecatedQuorumStoreInlineHybrid(..)
                    | Payload::DeprecatedQuorumStoreInlineHybridV2(..) => {},
                }
            }
            PayloadFilter::InQuorumStore(exclude_batches)
        }
    }
}

impl fmt::Display for PayloadFilter {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            PayloadFilter::DirectMempool(excluded_txns) => {
                let mut txns_str = "".to_string();
                for tx in excluded_txns.iter() {
                    write!(txns_str, "{} ", tx)?;
                }
                write!(f, "{}", txns_str)
            },
            PayloadFilter::InQuorumStore(excluded_proofs) => {
                let mut proofs_str = "".to_string();
                for proof in excluded_proofs.iter() {
                    write!(proofs_str, "{} ", proof.digest())?;
                }
                write!(f, "{}", proofs_str)
            },
            PayloadFilter::Empty => {
                write!(f, "Empty filter")
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        payload::{
            BatchPointer, InlineBatches, OptBatches, OptQuorumStorePayload, PayloadExecutionLimit,
        },
        proof_of_store::{BatchInfo, ProofCache},
    };
    use aptos_crypto::{ed25519::Ed25519PrivateKey, HashValue, PrivateKey, SigningKey, Uniform};
    use aptos_types::{
        chain_id::ChainId,
        secret_sharing::Ciphertext,
        transaction::{
            encrypted_payload::{EncryptedInner, EncryptedPayload},
            RawTransaction, Script, TransactionExtraConfig, TransactionPayload,
        },
        validator_verifier::random_validator_verifier,
    };

    const MAX_BATCH_TXNS: u64 = 100;
    const MAX_BATCH_BYTES: u64 = 1024 * 1024;

    fn make_batch_info(author: PeerId, num_txns: u64, num_bytes: u64) -> BatchInfo {
        BatchInfo::new(
            author,
            aptos_types::quorum_store::BatchId::new_for_test(1),
            1, // epoch
            1000,
            HashValue::random(),
            num_txns,
            num_bytes,
            0,
        )
    }

    fn make_batch_info_with_txns(author: PeerId, txns: &[SignedTransaction]) -> BatchInfo {
        let batch_payload = BatchPayload::new(author, txns.to_vec());
        let digest = batch_payload.hash();
        let num_bytes = batch_payload.num_bytes() as u64;
        BatchInfo::new(
            author,
            aptos_types::quorum_store::BatchId::new_for_test(1),
            1,
            1000,
            digest,
            txns.len() as u64,
            num_bytes,
            0,
        )
    }

    #[test]
    fn test_verify_batch_info_limits_accepts_valid() {
        let author = PeerId::random();
        let batch = make_batch_info(author, 50, 500_000);
        assert!(verify_batch_info_limits(&batch, MAX_BATCH_TXNS, MAX_BATCH_BYTES).is_ok());
    }

    #[test]
    fn test_verify_batch_info_limits_rejects_excess_txns() {
        let author = PeerId::random();
        let batch = make_batch_info(author, MAX_BATCH_TXNS + 1, 100);
        assert!(verify_batch_info_limits(&batch, MAX_BATCH_TXNS, MAX_BATCH_BYTES).is_err());
    }

    #[test]
    fn test_verify_batch_info_limits_rejects_excess_bytes() {
        let author = PeerId::random();
        let batch = make_batch_info(author, 1, MAX_BATCH_BYTES + 1);
        assert!(verify_batch_info_limits(&batch, MAX_BATCH_TXNS, MAX_BATCH_BYTES).is_err());
    }

    #[test]
    fn test_verify_batch_info_limits_rejects_overflow_values() {
        let author = PeerId::random();
        let batch = make_batch_info(author, u64::MAX, u64::MAX);
        assert!(verify_batch_info_limits(&batch, MAX_BATCH_TXNS, MAX_BATCH_BYTES).is_err());
    }

    #[test]
    fn test_verify_opt_batches_rejects_oversized_batch() {
        let (signers, validators) = random_validator_verifier(1, None, false);
        let author = signers[0].author();

        let bad_batch = make_batch_info(author, 1, u64::MAX);
        let opt_batches: OptBatches<BatchInfo> = BatchPointer::new(vec![bad_batch]);

        assert!(Payload::verify_opt_batches(
            &validators,
            &opt_batches,
            MAX_BATCH_TXNS,
            MAX_BATCH_BYTES
        )
        .is_err());
    }

    #[test]
    fn test_verify_opt_batches_accepts_valid() {
        let (signers, validators) = random_validator_verifier(1, None, false);
        let author = signers[0].author();

        let batch = make_batch_info(author, 50, 500_000);
        let opt_batches: OptBatches<BatchInfo> = BatchPointer::new(vec![batch]);

        assert!(Payload::verify_opt_batches(
            &validators,
            &opt_batches,
            MAX_BATCH_TXNS,
            MAX_BATCH_BYTES
        )
        .is_ok());
    }

    #[test]
    fn test_verify_opt_batches_rejects_before_checking_author() {
        let (_signers, validators) = random_validator_verifier(1, None, false);
        // Use a random author NOT in the validator set, but with oversized batch.
        // The limit check should fire before the author check.
        let bad_author = PeerId::random();
        let bad_batch = make_batch_info(bad_author, MAX_BATCH_TXNS + 1, 100);
        let opt_batches: OptBatches<BatchInfo> = BatchPointer::new(vec![bad_batch]);

        let err =
            Payload::verify_opt_batches(&validators, &opt_batches, MAX_BATCH_TXNS, MAX_BATCH_BYTES)
                .unwrap_err();
        // Should fail on batch limit, not author
        assert!(err.to_string().contains("Batch txn count"));
    }

    #[test]
    fn test_verify_inline_batches_rejects_oversized_batch() {
        let author = PeerId::random();
        let bad_batch = make_batch_info(author, u64::MAX / 2 + 1, u64::MAX / 2 + 1);
        let empty_txns = vec![];
        let inline_batches = vec![(&bad_batch, &empty_txns)];

        assert!(Payload::verify_inline_batches(
            inline_batches.into_iter(),
            MAX_BATCH_TXNS,
            MAX_BATCH_BYTES,
        )
        .is_err());
    }

    fn create_normal_signed_transaction() -> SignedTransaction {
        let private_key = Ed25519PrivateKey::generate_for_testing();
        let public_key = private_key.public_key();
        let raw_transaction = RawTransaction::new(
            AccountAddress::random(),
            0,
            TransactionPayload::Script(Script::new(vec![], vec![], vec![])),
            0,
            0,
            0,
            ChainId::new(10),
        );
        let signature = private_key.sign(&raw_transaction).unwrap();
        SignedTransaction::new(raw_transaction, public_key, signature)
    }

    fn create_encrypted_signed_transaction() -> SignedTransaction {
        let private_key = Ed25519PrivateKey::generate_for_testing();
        let public_key = private_key.public_key();
        let encrypted_payload = EncryptedPayload::Encrypted(EncryptedInner {
            ciphertext: Ciphertext::random(),
            extra_config: TransactionExtraConfig::V1 {
                multisig_address: None,
                replay_protection_nonce: None,
            },
            payload_hash: HashValue::random(),
            encryption_epoch: 0,
            claimed_entry_fun: None,
        });
        let raw_transaction = RawTransaction::new(
            AccountAddress::random(),
            0,
            TransactionPayload::EncryptedPayload(encrypted_payload),
            0,
            0,
            0,
            ChainId::new(10),
        );
        SignedTransaction::new(
            raw_transaction,
            public_key,
            aptos_crypto::ed25519::Ed25519Signature::dummy_signature(),
        )
    }

    fn make_batch_info_ext_v2_with_txns(
        author: PeerId,
        txns: &[SignedTransaction],
        kind: BatchKind,
    ) -> BatchInfoExt {
        let batch_payload = BatchPayload::new(author, txns.to_vec());
        let digest = batch_payload.hash();
        let num_bytes = batch_payload.num_bytes() as u64;
        BatchInfoExt::new_v2(
            author,
            aptos_types::quorum_store::BatchId::new_for_test(1),
            1,
            1000,
            digest,
            txns.len() as u64,
            num_bytes,
            0,
            kind,
        )
    }

    #[test]
    fn test_verify_inline_batches_rejects_encrypted_batch_with_normal_txn() {
        let author = PeerId::random();
        let txns = vec![create_normal_signed_transaction()];
        let batch = make_batch_info_ext_v2_with_txns(author, &txns, BatchKind::Encrypted);
        let inline_batches = vec![(&batch, &txns)];

        let err = Payload::verify_inline_batches(
            inline_batches.into_iter(),
            MAX_BATCH_TXNS,
            MAX_BATCH_BYTES,
        )
        .unwrap_err();
        assert!(
            err.to_string().contains("non-encrypted"),
            "Expected non-encrypted error, got: {}",
            err
        );
    }

    #[test]
    fn test_verify_inline_batches_rejects_normal_batch_with_encrypted_txn() {
        let author = PeerId::random();
        let txns = vec![create_encrypted_signed_transaction()];
        let batch = make_batch_info_ext_v2_with_txns(author, &txns, BatchKind::Normal);
        let inline_batches = vec![(&batch, &txns)];

        let err = Payload::verify_inline_batches(
            inline_batches.into_iter(),
            MAX_BATCH_TXNS,
            MAX_BATCH_BYTES,
        )
        .unwrap_err();
        assert!(
            err.to_string().contains("encrypted transaction"),
            "Expected encrypted transaction error, got: {}",
            err
        );
    }

    #[test]
    fn test_verify_inline_batches_rejects_invalid_ciphertext() {
        let author = PeerId::random();
        // Ciphertext::random() produces ciphertext that won't verify against the sender
        let txns = vec![create_encrypted_signed_transaction()];
        let batch = make_batch_info_ext_v2_with_txns(author, &txns, BatchKind::Encrypted);
        let inline_batches = vec![(&batch, &txns)];

        let err = Payload::verify_inline_batches(
            inline_batches.into_iter(),
            MAX_BATCH_TXNS,
            MAX_BATCH_BYTES,
        )
        .unwrap_err();
        assert!(
            err.to_string().contains("ciphertext verification failed"),
            "Expected ciphertext verification error, got: {}",
            err
        );
    }

    #[test]
    fn test_verify_inline_batches_rejects_v1_batch_with_encrypted_txn() {
        let author = PeerId::random();
        let txns = vec![create_encrypted_signed_transaction()];
        let batch = make_batch_info_with_txns(author, &txns);
        let inline_batches = vec![(&batch, &txns)];

        let err = Payload::verify_inline_batches(
            inline_batches.into_iter(),
            MAX_BATCH_TXNS,
            MAX_BATCH_BYTES,
        )
        .unwrap_err();
        assert!(
            err.to_string()
                .contains("V1 batch contains encrypted transaction"),
            "Expected V1 encrypted transaction error, got: {}",
            err
        );
    }

    #[test]
    fn test_payload_verify_rejects_encrypted_inline_batch_with_invalid_ciphertext() {
        let (signers, validators) = random_validator_verifier(1, None, false);
        let author = signers[0].author();
        let proof_cache = ProofCache::new(16);

        let txns = vec![create_encrypted_signed_transaction()];
        let batch_info = make_batch_info_ext_v2_with_txns(author, &txns, BatchKind::Encrypted);
        let inline_batches: InlineBatches<BatchInfoExt> = vec![(batch_info, txns)].into();

        let payload = Payload::OptQuorumStore(OptQuorumStorePayload::new_v2(
            inline_batches,
            BatchPointer::new(vec![]),
            BatchPointer::new(vec![]),
            PayloadExecutionLimit::None,
        ));

        let err = payload
            .verify(
                &validators,
                &proof_cache,
                true, // quorum_store_enabled
                true, // opt_qs_v2_rx_enabled
                MAX_BATCH_TXNS,
                MAX_BATCH_BYTES,
            )
            .unwrap_err();
        assert!(
            err.to_string().contains("ciphertext verification failed"),
            "Expected ciphertext verification error through Payload::verify, got: {}",
            err
        );
    }
}
