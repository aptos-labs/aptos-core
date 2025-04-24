// Copyright © Aptos Foundation
// Parts of the project are originally copyright © Meta Platforms, Inc.
// SPDX-License-Identifier: Apache-2.0

use crate::{
    payload::{OptQuorumStorePayload, PayloadExecutionLimit, RaptrPayload},
    proof_of_store::{BatchInfo, ProofCache, ProofOfStore},
};
use anyhow::ensure;
use aptos_crypto::{
    hash::{CryptoHash, CryptoHasher},
    HashValue,
};
use aptos_crypto_derive::CryptoHasher;
use aptos_executor_types::ExecutorResult;
use aptos_infallible::Mutex;
use aptos_logger::prelude::*;
use aptos_types::{
    account_address::AccountAddress, transaction::SignedTransaction,
    validator_verifier::ValidatorVerifier, vm_status::DiscardedVMStatus, PeerId,
};
use once_cell::sync::OnceCell;
use rayon::prelude::*;
use serde::{Deserialize, Serialize};
use std::{
    collections::HashSet,
    fmt::{self, Write},
    ops::Deref,
    sync::Arc,
    u64,
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
    pub sequence_number: u64,
    pub hash: HashValue,
}

impl TransactionSummary {
    pub fn new(sender: AccountAddress, sequence_number: u64, hash: HashValue) -> Self {
        Self {
            sender,
            sequence_number,
            hash,
        }
    }
}

impl fmt::Display for TransactionSummary {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}:{}", self.sender, self.sequence_number,)
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Deserialize, Serialize, Hash, Ord, PartialOrd)]
pub struct TxnSummaryWithExpiration {
    pub sender: AccountAddress,
    pub sequence_number: u64,
    pub expiration_timestamp_secs: u64,
    pub hash: HashValue,
}

impl TxnSummaryWithExpiration {
    pub fn new(
        sender: AccountAddress,
        sequence_number: u64,
        expiration_timestamp_secs: u64,
        hash: HashValue,
    ) -> Self {
        Self {
            sender,
            sequence_number,
            expiration_timestamp_secs,
            hash,
        }
    }
}

impl fmt::Display for TxnSummaryWithExpiration {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}:{}", self.sender, self.sequence_number,)
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
    pub sequence_number: u64,
    pub hash: HashValue,
    pub reason: DiscardedVMStatus,
}

#[derive(Deserialize, Serialize, Clone, Debug, PartialEq, Eq)]
pub struct ProofWithData {
    pub proofs: Vec<ProofOfStore>,
}

impl ProofWithData {
    pub fn new(proofs: Vec<ProofOfStore>) -> Self {
        Self { proofs }
    }

    pub fn empty() -> Self {
        Self::new(vec![])
    }

    pub fn extend(&mut self, other: ProofWithData) {
        self.proofs.extend(other.proofs);
    }

    pub fn len(&self) -> usize {
        self.proofs
            .iter()
            .map(|proof| proof.num_txns() as usize)
            .sum()
    }

    pub fn num_bytes(&self) -> usize {
        self.proofs
            .iter()
            .map(|proof| proof.num_bytes() as usize)
            .sum()
    }

    pub fn is_empty(&self) -> bool {
        self.proofs.is_empty()
    }
}

#[derive(Deserialize, Serialize, Clone, Debug)]
pub struct ProofWithDataWithTxnLimit {
    pub proof_with_data: ProofWithData,
    pub max_txns_to_execute: Option<u64>,
}

impl PartialEq for ProofWithDataWithTxnLimit {
    fn eq(&self, other: &Self) -> bool {
        self.proof_with_data == other.proof_with_data
            && self.max_txns_to_execute == other.max_txns_to_execute
    }
}

impl Eq for ProofWithDataWithTxnLimit {}

impl ProofWithDataWithTxnLimit {
    pub fn new(proof_with_data: ProofWithData, max_txns_to_execute: Option<u64>) -> Self {
        Self {
            proof_with_data,
            max_txns_to_execute,
        }
    }

    pub fn extend(&mut self, other: ProofWithDataWithTxnLimit) {
        self.proof_with_data.extend(other.proof_with_data);
        // InQuorumStoreWithLimit TODO: what is the right logic here ???
        if self.max_txns_to_execute.is_none() {
            self.max_txns_to_execute = other.max_txns_to_execute;
        }
    }
}

fn sum_max_txns_to_execute(m1: Option<u64>, m2: Option<u64>) -> Option<u64> {
    match (m1, m2) {
        (None, _) => m2,
        (_, None) => m1,
        (Some(m1), Some(m2)) => Some(m1 + m2),
    }
}

/// The payload in block.
#[derive(Deserialize, Serialize, Clone, Debug, PartialEq, Eq)]
pub enum Payload {
    DirectMempool(Vec<SignedTransaction>),
    InQuorumStore(ProofWithData),
    InQuorumStoreWithLimit(ProofWithDataWithTxnLimit),
    QuorumStoreInlineHybrid(
        Vec<(BatchInfo, Vec<SignedTransaction>)>,
        ProofWithData,
        Option<u64>,
    ),
    OptQuorumStore(OptQuorumStorePayload),
    Raptr(RaptrPayload),
}

impl From<RaptrPayload> for Payload {
    fn from(raikou_payload: RaptrPayload) -> Self {
        Payload::Raptr(raikou_payload)
    }
}

impl Payload {
    pub fn as_raptr_payload(&self) -> &RaptrPayload {
        match self {
            Payload::Raptr(raikou_payload) => raikou_payload,
            _ => unreachable!(),
        }
    }

    pub fn transform_to_quorum_store_v2(self, max_txns_to_execute: Option<u64>) -> Self {
        match self {
            Payload::InQuorumStore(proof_with_status) => Payload::InQuorumStoreWithLimit(
                ProofWithDataWithTxnLimit::new(proof_with_status, max_txns_to_execute),
            ),
            Payload::QuorumStoreInlineHybrid(inline_batches, proof_with_data, _) => {
                Payload::QuorumStoreInlineHybrid(
                    inline_batches,
                    proof_with_data,
                    max_txns_to_execute,
                )
            },
            Payload::InQuorumStoreWithLimit(_) => {
                panic!("Payload is already in quorumStoreV2 format");
            },
            Payload::DirectMempool(_) => {
                panic!("Payload is in direct mempool format");
            },
            Payload::OptQuorumStore(mut opt_qs_payload) => {
                opt_qs_payload.set_execution_limit(PayloadExecutionLimit::max_txns_to_execute(
                    max_txns_to_execute,
                ));
                Payload::OptQuorumStore(opt_qs_payload)
            },
            payload @ Payload::Raptr(_) => payload,
        }
    }

    pub fn empty(quorum_store_enabled: bool, allow_batches_without_pos_in_proposal: bool) -> Self {
        if quorum_store_enabled {
            if allow_batches_without_pos_in_proposal {
                Payload::QuorumStoreInlineHybrid(Vec::new(), ProofWithData::new(Vec::new()), None)
            } else {
                Payload::InQuorumStore(ProofWithData::new(Vec::new()))
            }
        } else {
            Payload::DirectMempool(Vec::new())
        }
    }

    pub fn len(&self) -> usize {
        match self {
            Payload::DirectMempool(txns) => txns.len(),
            Payload::InQuorumStore(proof_with_status) => proof_with_status.len(),
            Payload::InQuorumStoreWithLimit(proof_with_status) => {
                // here we return the actual length of the payload; limit is considered at the stage
                // where we prepare the block from the payload
                proof_with_status.proof_with_data.len()
            },
            Payload::QuorumStoreInlineHybrid(inline_batches, proof_with_data, _) => {
                proof_with_data.len()
                    + inline_batches
                        .iter()
                        .map(|(_, txns)| txns.len())
                        .sum::<usize>()
            },
            Payload::OptQuorumStore(opt_qs_payload) => opt_qs_payload.num_txns(),
            Payload::Raptr(raikou_payload) => raikou_payload.num_txns(),
        }
    }

    pub fn len_for_execution(&self) -> u64 {
        match self {
            Payload::DirectMempool(txns) => txns.len() as u64,
            Payload::InQuorumStore(proof_with_status) => proof_with_status.len() as u64,
            Payload::InQuorumStoreWithLimit(proof_with_status) => {
                // here we return the actual length of the payload; limit is considered at the stage
                // where we prepare the block from the payload
                (proof_with_status.proof_with_data.len() as u64)
                    .min(proof_with_status.max_txns_to_execute.unwrap_or(u64::MAX))
            },
            Payload::QuorumStoreInlineHybrid(
                inline_batches,
                proof_with_data,
                max_txns_to_execute,
            ) => ((proof_with_data.len()
                + inline_batches
                    .iter()
                    .map(|(_, txns)| txns.len())
                    .sum::<usize>()) as u64)
                .min(max_txns_to_execute.unwrap_or(u64::MAX)),
            Payload::OptQuorumStore(opt_qs_payload) => {
                opt_qs_payload.max_txns_to_execute().unwrap_or(u64::MAX)
            },
            Payload::Raptr(_raikou_payload) => u64::MAX,
        }
    }

    pub fn is_empty(&self) -> bool {
        match self {
            Payload::DirectMempool(txns) => txns.is_empty(),
            Payload::InQuorumStore(proof_with_status) => proof_with_status.proofs.is_empty(),
            Payload::InQuorumStoreWithLimit(proof_with_status) => {
                proof_with_status.proof_with_data.proofs.is_empty()
            },
            Payload::QuorumStoreInlineHybrid(inline_batches, proof_with_data, _) => {
                proof_with_data.proofs.is_empty() && inline_batches.is_empty()
            },
            Payload::OptQuorumStore(opt_qs_payload) => opt_qs_payload.is_empty(),
            Payload::Raptr(raikou_payload) => raikou_payload.is_empty(),
        }
    }

    pub fn extend(self, other: Payload) -> Self {
        match (self, other) {
            (Payload::DirectMempool(v1), Payload::DirectMempool(v2)) => {
                let mut v3 = v1;
                v3.extend(v2);
                Payload::DirectMempool(v3)
            },
            (Payload::InQuorumStore(p1), Payload::InQuorumStore(p2)) => {
                let mut p3 = p1;
                p3.extend(p2);
                Payload::InQuorumStore(p3)
            },
            (Payload::InQuorumStoreWithLimit(p1), Payload::InQuorumStoreWithLimit(p2)) => {
                let mut p3 = p1;
                p3.extend(p2);
                Payload::InQuorumStoreWithLimit(p3)
            },
            (
                Payload::QuorumStoreInlineHybrid(b1, p1, m1),
                Payload::QuorumStoreInlineHybrid(b2, p2, m2),
            ) => {
                let mut b3 = b1;
                b3.extend(b2);
                let mut p3 = p1;
                p3.extend(p2);
                // TODO: What's the right logic here?
                let m3 = sum_max_txns_to_execute(m1, m2);
                Payload::QuorumStoreInlineHybrid(b3, p3, m3)
            },
            (Payload::QuorumStoreInlineHybrid(b1, p1, m1), Payload::InQuorumStore(p2)) => {
                // TODO: How to update m1?
                let mut p3 = p1;
                p3.extend(p2);
                Payload::QuorumStoreInlineHybrid(b1, p3, m1)
            },
            (Payload::QuorumStoreInlineHybrid(b1, p1, m1), Payload::InQuorumStoreWithLimit(p2)) => {
                // TODO: What's the right logic here?
                let m3 = sum_max_txns_to_execute(m1, p2.max_txns_to_execute);
                let mut p3 = p1;
                p3.extend(p2.proof_with_data);
                Payload::QuorumStoreInlineHybrid(b1, p3, m3)
            },
            (Payload::InQuorumStore(p1), Payload::QuorumStoreInlineHybrid(b2, p2, m2)) => {
                let mut p3 = p1;
                p3.extend(p2);
                Payload::QuorumStoreInlineHybrid(b2, p3, m2)
            },
            (Payload::InQuorumStoreWithLimit(p1), Payload::QuorumStoreInlineHybrid(b2, p2, m2)) => {
                // TODO: What's the right logic here?
                let m3 = sum_max_txns_to_execute(p1.max_txns_to_execute, m2);
                let mut p3 = p1.proof_with_data;
                p3.extend(p2);
                Payload::QuorumStoreInlineHybrid(b2, p3, m3)
            },
            (
                Payload::QuorumStoreInlineHybrid(_inline_batches, _proofs, _limit),
                Payload::OptQuorumStore(_opt_qs),
            )
            | (
                Payload::OptQuorumStore(_opt_qs),
                Payload::QuorumStoreInlineHybrid(_inline_batches, _proofs, _limit),
            ) => {
                unimplemented!(
                    "Cannot extend OptQuorumStore with QuorumStoreInlineHybrid or viceversa"
                )
            },
            (Payload::OptQuorumStore(opt_qs1), Payload::OptQuorumStore(opt_qs2)) => {
                let opt_qs3 = opt_qs1.extend(opt_qs2);
                Payload::OptQuorumStore(opt_qs3)
            },
            (_, _) => unreachable!(),
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
            Payload::InQuorumStore(proof_with_status) => proof_with_status.num_bytes(),
            Payload::InQuorumStoreWithLimit(proof_with_status) => {
                proof_with_status.proof_with_data.num_bytes()
            },
            Payload::QuorumStoreInlineHybrid(inline_batches, proof_with_data, _) => {
                proof_with_data.num_bytes()
                    + inline_batches
                        .iter()
                        .map(|(batch_info, _)| batch_info.num_bytes() as usize)
                        .sum::<usize>()
            },
            Payload::OptQuorumStore(opt_qs_payload) => opt_qs_payload.num_bytes(),
            Payload::Raptr(raikou_payload) => raikou_payload.num_bytes(),
        }
    }

    fn verify_with_cache(
        proofs: &[ProofOfStore],
        validator: &ValidatorVerifier,
        proof_cache: &ProofCache,
    ) -> anyhow::Result<()> {
        let unverified: Vec<_> = proofs
            .iter()
            .filter(|proof| {
                proof_cache.get(proof.info()).map_or(true, |cached_proof| {
                    cached_proof != *proof.multi_signature()
                })
            })
            .collect();
        unverified
            .par_iter()
            .with_min_len(2)
            .try_for_each(|proof| proof.verify(validator, proof_cache))?;
        Ok(())
    }

    pub fn verify_inline_batches<'a>(
        inline_batches: impl Iterator<Item = (&'a BatchInfo, &'a Vec<SignedTransaction>)>,
    ) -> anyhow::Result<()> {
        for (batch, payload) in inline_batches {
            // TODO: Can cloning be avoided here?
            let computed_digest = BatchPayload::new(batch.author(), payload.clone()).hash();
            ensure!(
                computed_digest == *batch.digest(),
                "Hash of the received inline batch doesn't match the digest value for batch {}: {} != {}",
                batch,
                computed_digest,
                batch.digest()
            );
        }
        Ok(())
    }

    pub fn verify(
        &self,
        validator: &ValidatorVerifier,
        proof_cache: &ProofCache,
        quorum_store_enabled: bool,
    ) -> anyhow::Result<()> {
        match (quorum_store_enabled, self) {
            (false, Payload::DirectMempool(_)) => Ok(()),
            (true, Payload::InQuorumStore(proof_with_status)) => {
                Self::verify_with_cache(&proof_with_status.proofs, validator, proof_cache)
            },
            (true, Payload::InQuorumStoreWithLimit(proof_with_status)) => Self::verify_with_cache(
                &proof_with_status.proof_with_data.proofs,
                validator,
                proof_cache,
            ),
            (true, Payload::QuorumStoreInlineHybrid(inline_batches, proof_with_data, _)) => {
                Self::verify_with_cache(&proof_with_data.proofs, validator, proof_cache)?;
                Self::verify_inline_batches(
                    inline_batches.iter().map(|(info, txns)| (info, txns)),
                )?;
                Ok(())
            },
            (true, Payload::OptQuorumStore(opt_quorum_store)) => {
                let proof_with_data = opt_quorum_store.proof_with_data();
                Self::verify_with_cache(&proof_with_data.batch_summary, validator, proof_cache)?;
                Self::verify_inline_batches(
                    opt_quorum_store
                        .inline_batches()
                        .iter()
                        .map(|batch| (batch.info(), batch.transactions())),
                )?;
                Ok(())
            },
            (true, Payload::Raptr(raikou)) => {
                let proof_with_data = raikou.proof_with_data();
                Self::verify_with_cache(&proof_with_data.batch_summary, validator, proof_cache)?;
                Ok(())
            },
            (_, _) => Err(anyhow::anyhow!(
                "Wrong payload type. Expected Payload::InQuorumStore {} got {} ",
                quorum_store_enabled,
                self
            )),
        }
    }

    pub(crate) fn verify_epoch(&self, epoch: u64) -> anyhow::Result<()> {
        match self {
            Payload::DirectMempool(_) => return Ok(()),
            Payload::InQuorumStore(proof_with_data) => {
                ensure!(
                    proof_with_data.proofs.iter().all(|p| p.epoch() == epoch),
                    "Payload epoch doesn't match given epoch"
                );
            },
            Payload::InQuorumStoreWithLimit(proof_with_data_with_txn_limit) => {
                ensure!(
                    proof_with_data_with_txn_limit
                        .proof_with_data
                        .proofs
                        .iter()
                        .all(|p| p.epoch() == epoch),
                    "Payload epoch doesn't match given epoch"
                );
            },
            Payload::QuorumStoreInlineHybrid(inline_batches, proof_with_data, _) => {
                ensure!(
                    proof_with_data.proofs.iter().all(|p| p.epoch() == epoch),
                    "Payload proof epoch doesn't match given epoch"
                );
                ensure!(
                    inline_batches.iter().all(|b| b.0.epoch() == epoch),
                    "Payload inline batch epoch doesn't match given epoch"
                )
            },
            Payload::OptQuorumStore(opt_quorum_store_payload) => {
                opt_quorum_store_payload.check_epoch(epoch)?;
            },
            Payload::Raptr(_) => {},
        };
        Ok(())
    }
}

impl fmt::Display for Payload {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Payload::DirectMempool(txns) => {
                write!(f, "InMemory txns: {}", txns.len())
            },
            Payload::InQuorumStore(proof_with_status) => {
                write!(f, "InMemory proofs: {}", proof_with_status.proofs.len())
            },
            Payload::InQuorumStoreWithLimit(proof_with_status) => {
                write!(
                    f,
                    "InMemory proofs: {}",
                    proof_with_status.proof_with_data.proofs.len()
                )
            },
            Payload::QuorumStoreInlineHybrid(inline_batches, proof_with_data, _) => {
                write!(
                    f,
                    "Inline txns: {}, InMemory proofs: {}",
                    inline_batches
                        .iter()
                        .map(|(_, txns)| txns.len())
                        .sum::<usize>(),
                    proof_with_data.proofs.len()
                )
            },
            Payload::OptQuorumStore(opt_quorum_store) => {
                write!(f, "{}", opt_quorum_store)
            },
            Payload::Raptr(raikou_payload) => write!(f, "{}", raikou_payload),
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
    InQuorumStore(HashSet<BatchInfo>),
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
                            sequence_number: txn.sequence_number(),
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
                    Payload::InQuorumStore(proof_with_status) => {
                        for proof in &proof_with_status.proofs {
                            exclude_batches.insert(proof.info().clone());
                        }
                    },
                    Payload::InQuorumStoreWithLimit(proof_with_status) => {
                        for proof in &proof_with_status.proof_with_data.proofs {
                            exclude_batches.insert(proof.info().clone());
                        }
                    },
                    Payload::QuorumStoreInlineHybrid(inline_batches, proof_with_data, _) => {
                        for proof in &proof_with_data.proofs {
                            exclude_batches.insert(proof.info().clone());
                        }
                        for (batch_info, _) in inline_batches {
                            exclude_batches.insert(batch_info.clone());
                        }
                    },
                    Payload::DirectMempool(_) => {
                        error!("DirectMempool payload in InQuorumStore filter");
                    },
                    Payload::OptQuorumStore(opt_qs_payload) => {
                        for batch in opt_qs_payload.inline_batches().iter() {
                            exclude_batches.insert(batch.info().clone());
                        }
                        for batch_info in &opt_qs_payload.opt_batches().batch_summary {
                            exclude_batches.insert(batch_info.clone());
                        }
                        for proof in &opt_qs_payload.proof_with_data().batch_summary {
                            exclude_batches.insert(proof.info().clone());
                        }
                    },
                    Payload::Raptr(raikou_payload) => {
                        for sub_block in raikou_payload.sub_blocks() {
                            for batch_info in sub_block.deref() {
                                exclude_batches.insert(batch_info.clone());
                            }
                        }
                        for proof in &raikou_payload.proof_with_data().batch_summary {
                            exclude_batches.insert(proof.info().clone());
                        }
                    },
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
