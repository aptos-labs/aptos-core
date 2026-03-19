// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use aptos_crypto::HashValue;
use aptos_types::account_address::AccountAddress;
use std::sync::Arc;

/// Lifecycle stages a transaction passes through.
#[derive(Debug, Clone, Copy, PartialEq, Eq, strum_macros::AsRefStr)]
pub enum TransactionStage {
    MempoolInsert,
    QsBatchPull,
    QsBatchCreated,
    QsProofOfStore,
    BlockProposed,
    BlockOrdered,
    ExecutionStart,
    Executed,
    PreCommit,
    Certified,
    Committed,
    MempoolCommit,
    MempoolReject,
}

/// Batch inclusion type in a block proposal.
#[derive(Debug, Clone, Copy, PartialEq, Eq, strum_macros::AsRefStr)]
pub enum BatchInclusionType {
    Proof,
    Opt,
    Inline,
}

/// Execution outcome for a transaction.
#[derive(Debug, Clone, Copy, PartialEq, Eq, strum_macros::AsRefStr)]
pub enum ExecutionStatus {
    Keep,
    Retry,
    Discard,
}

/// A single batch creation event (one per pull round that produced batches)
/// with its timestamp, batch count, and back-pressure state.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BatchCreationRecord {
    /// Wall-clock timestamp (usecs) when batches were created in this pull round.
    pub timestamp_usecs: u64,
    /// Number of batch objects created in this pull round (one per gas bucket).
    pub num_batches: u64,
    /// Whether txn-count back-pressure was active at creation time.
    pub bp_txn: bool,
    /// Whether proof-count back-pressure was active at creation time.
    pub bp_proof: bool,
}

/// Context captured at each QS batch pull for diagnosing pull latency.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BatchPullInfo {
    /// Monotonic counter: how many pull rounds have fired since the batch
    /// generator started. Comparing the first pull_round for a txn against
    /// MempoolInsert time shows how many rounds elapsed before this txn
    /// was picked up.
    pub pull_round: u64,
    /// Total batches created by this generator since start. Comparing with
    /// the value at MempoolInsert time shows how many batches were created
    /// before this txn was first included — i.e., the txn "missed" that many
    /// batches while sitting in mempool.
    pub total_batches_created: u64,
    /// Number of transactions returned in this pull round.
    pub pulled_txn_count: u64,
    /// Max transactions allowed in this pull round (dynamic back-pressure limit).
    pub pull_max_txn: u64,
    /// Number of transactions already in-flight (excluded from this pull).
    pub excluded_txn_count: u64,
    /// Whether txn-count back-pressure is active.
    pub bp_txn_count: bool,
    /// Whether proof-count back-pressure is active.
    pub bp_proof_count: bool,
    /// Recent batch creation events (timestamps + BP state), capped at 200 entries.
    /// Used to show what the batch generator was doing between MempoolInsert
    /// and the first QsBatchPull, and how many batches had back-pressure active.
    pub recent_batches: Vec<BatchCreationRecord>,
    /// Min gas price across all txns in batches created since batch generator start.
    /// Compared with this txn's gas_unit_price to show if it was deprioritized.
    pub prev_batches_min_gas: Option<u64>,
    /// Max gas price across all txns in batches created since batch generator start.
    pub prev_batches_max_gas: Option<u64>,
    /// How many pull rounds returned zero txns (empty pulls) since last batch
    /// creation. High count = batch generator was polling but mempool had
    /// nothing (or back-pressure blocked pulls).
    pub empty_pulls_since_last_batch: u64,
    /// How many pull rounds had proof-count back-pressure active since last
    /// batch creation. Proof BP blocks normal pulls entirely (only force-pull
    /// at 250ms fires).
    pub bp_proof_rounds_since_last_batch: u64,
    /// How many pull rounds had txn-count back-pressure active since last
    /// batch creation. Txn BP reduces the dynamic pull limit.
    pub bp_txn_rounds_since_last_batch: u64,
}

/// Additional metadata for specific stages.
#[derive(Debug, Clone)]
pub enum StageMetadata {
    BatchInclusion(BatchInclusionType),
    Execution(ExecutionStatus),
    /// Arc-wrapped to avoid cloning the inner Vecs when the same pull info
    /// is recorded for multiple traced txns in the same pull round.
    BatchPull(Arc<BatchPullInfo>),
}

/// A single recorded stage in a transaction's lifecycle.
#[derive(Debug, Clone)]
pub struct StageRecord {
    pub stage: TransactionStage,
    pub timestamp_usecs: u64,
    pub attempt: u32,
    pub metadata: Option<StageMetadata>,
}

/// Complete trace of a transaction's lifecycle across attempts.
#[derive(Debug, Clone)]
pub struct TransactionTrace {
    pub hash: HashValue,
    pub sender: AccountAddress,
    pub insertion_time_usecs: u64,
    pub current_attempt: u32,
    /// Gas unit price of this transaction, recorded at first QsBatchPull.
    /// Used to diagnose prioritization: mempool sorts by gas price descending.
    pub gas_unit_price: Option<u64>,
    pub stages: Vec<StageRecord>,
}

impl TransactionTrace {
    pub fn new(hash: HashValue, sender: AccountAddress, now_usecs: u64) -> Self {
        Self {
            hash,
            sender,
            insertion_time_usecs: now_usecs,
            current_attempt: 1,
            gas_unit_price: None,
            stages: Vec::new(),
        }
    }

    pub fn record(&mut self, stage: TransactionStage, timestamp_usecs: u64) {
        self.stages.push(StageRecord {
            stage,
            timestamp_usecs,
            attempt: self.current_attempt,
            metadata: None,
        });
    }

    pub fn record_with_metadata(
        &mut self,
        stage: TransactionStage,
        timestamp_usecs: u64,
        metadata: StageMetadata,
    ) {
        self.stages.push(StageRecord {
            stage,
            timestamp_usecs,
            attempt: self.current_attempt,
            metadata: Some(metadata),
        });
    }
}
