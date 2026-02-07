// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use crate::{
    payload::{OptBatches, OptQuorumStorePayload, PayloadExecutionLimit, TxnAndGasLimits},
    proof_of_store::{BatchInfo, BatchInfoExt, ProofCache, ProofOfStore, TBatchInfo},
};
use anyhow::{bail, ensure};
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

/// Deprecated: Kept for BCS backward compatibility. Do not use.
#[doc(hidden)]
#[derive(Deserialize, Serialize, Clone, Debug, PartialEq, Eq)]
pub struct ProofWithData {
    pub proofs: Vec<ProofOfStore<BatchInfo>>,
}

/// Deprecated: Kept for BCS backward compatibility. Do not use.
#[doc(hidden)]
#[derive(Deserialize, Serialize, Clone, Debug, PartialEq, Eq)]
pub struct ProofWithDataWithTxnLimit {
    pub proof_with_data: ProofWithData,
    pub max_txns_to_execute: Option<u64>,
}

/// The payload in block.
#[derive(Deserialize, Serialize, Clone, Debug, PartialEq, Eq)]
pub enum Payload {
    DirectMempool(Vec<SignedTransaction>),
    /// Deprecated: kept for BCS backward compatibility. Do not use.
    #[doc(hidden)]
    DeprecatedInQuorumStore(ProofWithData),
    /// Deprecated: kept for BCS backward compatibility. Do not use.
    #[doc(hidden)]
    DeprecatedInQuorumStoreWithLimit(ProofWithDataWithTxnLimit),
    /// Deprecated: kept for BCS backward compatibility. Do not use.
    #[doc(hidden)]
    DeprecatedQuorumStoreInlineHybrid(
        Vec<(BatchInfo, Vec<SignedTransaction>)>,
        ProofWithData,
        Option<u64>,
    ),
    OptQuorumStore(OptQuorumStorePayload),
    /// Deprecated: kept for BCS backward compatibility. Do not use.
    #[doc(hidden)]
    DeprecatedQuorumStoreInlineHybridV2(
        Vec<(BatchInfo, Vec<SignedTransaction>)>,
        ProofWithData,
        PayloadExecutionLimit,
    ),
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
    ) -> anyhow::Result<()> {
        for (batch, payload) in inline_batches {
            // TODO: Can cloning be avoided here?
            let computed_digest = BatchPayload::new(batch.author(), payload.clone()).hash();
            ensure!(
                computed_digest == *batch.digest(),
                "Hash of the received inline batch doesn't match the digest value for batch {:?}: {} != {}",
                batch,
                computed_digest,
                batch.digest()
            );
        }
        Ok(())
    }

    pub fn verify_opt_batches<T: TBatchInfo>(
        verifier: &ValidatorVerifier,
        opt_batches: &OptBatches<T>,
    ) -> anyhow::Result<()> {
        let authors = verifier.address_to_validator_index();
        for batch in &opt_batches.batch_summary {
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
    ) -> anyhow::Result<()> {
        match (quorum_store_enabled, self) {
            (false, Payload::DirectMempool(_)) => Ok(()),
            (true, Payload::OptQuorumStore(OptQuorumStorePayload::V1(p))) => {
                let proof_with_data = p.proof_with_data();
                Self::verify_with_cache(&proof_with_data.batch_summary, verifier, proof_cache)?;
                Self::verify_inline_batches(
                    p.inline_batches()
                        .iter()
                        .map(|batch| (batch.info(), batch.transactions())),
                )?;
                Self::verify_opt_batches(verifier, p.opt_batches())?;
                Ok(())
            },
            (true, Payload::OptQuorumStore(OptQuorumStorePayload::V2(p))) => {
                ensure!(
                    opt_qs_v2_rx_enabled,
                    "OptQuorumStorePayload::V2 cannot be accepted yet"
                );
                let proof_with_data = p.proof_with_data();
                Self::verify_with_cache(&proof_with_data.batch_summary, verifier, proof_cache)?;
                Self::verify_inline_batches(
                    p.inline_batches()
                        .iter()
                        .map(|batch| (batch.info(), batch.transactions())),
                )?;
                Self::verify_opt_batches(verifier, p.opt_batches())?;
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
