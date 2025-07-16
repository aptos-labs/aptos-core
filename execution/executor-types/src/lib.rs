// Copyright © Aptos Foundation
// Parts of the project are originally copyright © Meta Platforms, Inc.
// SPDX-License-Identifier: Apache-2.0
#![forbid(unsafe_code)]

use anyhow::Result;
use aptos_crypto::HashValue;
use aptos_scratchpad::SparseMerkleTree;
use aptos_types::{
    account_config::{NEW_EPOCH_EVENT_MOVE_TYPE_TAG, NEW_EPOCH_EVENT_V2_MOVE_TYPE_TAG},
    block_executor::{config::BlockExecutorConfigFromOnchain, partitioner::ExecutableBlock},
    contract_event::ContractEvent,
    dkg::DKG_START_EVENT_MOVE_TYPE_TAG,
    jwks::OBSERVED_JWK_UPDATED_MOVE_TYPE_TAG,
    ledger_info::LedgerInfoWithSignatures,
    state_store::state_key::StateKey,
    transaction::{
        Transaction, TransactionInfo, TransactionListWithProof, TransactionOutputListWithProof,
        Version,
    },
    write_set::WriteSet,
};
pub use error::{ExecutorError, ExecutorResult};
pub use ledger_update_output::LedgerUpdateOutput;
use state_compute_result::StateComputeResult;
use std::{
    collections::BTreeSet,
    ops::Deref,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
};

mod error;
pub mod execution_output;
mod ledger_update_output;
mod metrics;
pub mod planned;
pub mod state_checkpoint_output;
pub mod state_compute_result;
pub mod transactions_with_output;

pub trait ChunkExecutorTrait: Send + Sync {
    /// Verifies the transactions based on the provided proofs and ledger info. If the transactions
    /// are valid, executes them and returns the executed result for commit.
    #[cfg(any(test, feature = "fuzzing"))]
    fn execute_chunk(
        &self,
        txn_list_with_proof: TransactionListWithProof,
        // Target LI that has been verified independently: the proofs are relative to this version.
        verified_target_li: &LedgerInfoWithSignatures,
        epoch_change_li: Option<&LedgerInfoWithSignatures>,
    ) -> Result<()> {
        self.enqueue_chunk_by_execution(txn_list_with_proof, verified_target_li, epoch_change_li)?;

        self.update_ledger()
    }

    /// Similar to `execute_chunk`, but instead of executing transactions, apply the transaction
    /// outputs directly to get the executed result.
    #[cfg(any(test, feature = "fuzzing"))]
    fn apply_chunk(
        &self,
        txn_output_list_with_proof: TransactionOutputListWithProof,
        // Target LI that has been verified independently: the proofs are relative to this version.
        verified_target_li: &LedgerInfoWithSignatures,
        epoch_change_li: Option<&LedgerInfoWithSignatures>,
    ) -> Result<()> {
        self.enqueue_chunk_by_transaction_outputs(
            txn_output_list_with_proof,
            verified_target_li,
            epoch_change_li,
        )?;

        self.update_ledger()
    }

    /// Verifies the transactions based on the provided proofs and ledger info. If the transactions
    /// are valid, executes them and make state checkpoint, so that a later chunk of transaction can
    /// be applied on top of it. This stage calculates the state checkpoint, but not the top level
    /// transaction accumulator.
    fn enqueue_chunk_by_execution(
        &self,
        txn_list_with_proof: TransactionListWithProof,
        // Target LI that has been verified independently: the proofs are relative to this version.
        verified_target_li: &LedgerInfoWithSignatures,
        epoch_change_li: Option<&LedgerInfoWithSignatures>,
    ) -> Result<()>;

    /// Similar to `enqueue_chunk_by_execution`, but instead of executing transactions, apply the
    /// transaction outputs directly to get the executed result.
    fn enqueue_chunk_by_transaction_outputs(
        &self,
        txn_output_list_with_proof: TransactionOutputListWithProof,
        // Target LI that has been verified independently: the proofs are relative to this version.
        verified_target_li: &LedgerInfoWithSignatures,
        epoch_change_li: Option<&LedgerInfoWithSignatures>,
    ) -> Result<()>;

    /// As a separate stage, calculate the transaction accumulator changes, prepare for db commission.
    fn update_ledger(&self) -> Result<()>;

    /// Commit a previously executed chunk. Returns a chunk commit notification.
    fn commit_chunk(&self) -> Result<ChunkCommitNotification>;

    /// Resets the chunk executor by synchronizing state with storage.
    fn reset(&self) -> Result<()>;

    /// Finishes the chunk executor by releasing memory held by inner data structures(SMT).
    fn finish(&self);
}

pub struct StateSnapshotDelta {
    pub version: Version,
    pub smt: SparseMerkleTree,
    pub jmt_updates: Vec<(HashValue, (HashValue, StateKey))>,
}

pub trait BlockExecutorTrait: Send + Sync {
    /// Get the latest committed block id
    fn committed_block_id(&self) -> HashValue;

    /// Reset the internal state including cache with newly fetched latest committed block from storage.
    fn reset(&self) -> Result<()>;

    /// Reset with a virtual genesis block ID (used for consensus recovery)
    fn reset_with_virtual_genesis(
        &self,
        _virtual_genesis_block_id: Option<HashValue>,
    ) -> Result<()> {
        // Default implementation just calls reset() for backward compatibility
        self.reset()
    }

    /// Executes a block - TBD, this API will be removed in favor of `execute_and_state_checkpoint`, followed
    /// by `ledger_update` once we have ledger update as a separate pipeline phase.
    #[cfg(any(test, feature = "fuzzing"))]
    fn execute_block(
        &self,
        block: ExecutableBlock,
        parent_block_id: HashValue,
        onchain_config: BlockExecutorConfigFromOnchain,
    ) -> ExecutorResult<StateComputeResult> {
        let block_id = block.block_id;
        self.execute_and_update_state(block, parent_block_id, onchain_config)?;
        self.ledger_update(block_id, parent_block_id)
    }

    /// Executes a block and returns the state checkpoint output.
    fn execute_and_update_state(
        &self,
        block: ExecutableBlock,
        parent_block_id: HashValue,
        onchain_config: BlockExecutorConfigFromOnchain,
    ) -> ExecutorResult<()>;

    fn ledger_update(
        &self,
        block_id: HashValue,
        parent_block_id: HashValue,
    ) -> ExecutorResult<StateComputeResult>;

    #[cfg(any(test, feature = "fuzzing"))]
    fn commit_blocks(
        &self,
        block_ids: Vec<HashValue>,
        ledger_info_with_sigs: LedgerInfoWithSignatures,
    ) -> ExecutorResult<()> {
        for block_id in block_ids {
            self.pre_commit_block(block_id)?;
        }
        self.commit_ledger(ledger_info_with_sigs)
    }

    fn pre_commit_block(&self, block_id: HashValue) -> ExecutorResult<()>;

    fn commit_ledger(&self, ledger_info_with_sigs: LedgerInfoWithSignatures) -> ExecutorResult<()>;

    /// Finishes the block executor by releasing memory held by inner data structures(SMT).
    fn finish(&self);
}

#[derive(Clone)]
pub enum VerifyExecutionMode {
    NoVerify,
    Verify {
        txns_to_skip: Arc<BTreeSet<Version>>,
        lazy_quit: bool,
        seen_error: Arc<AtomicBool>,
    },
}

impl VerifyExecutionMode {
    pub fn verify_all() -> Self {
        Self::Verify {
            txns_to_skip: Arc::new(BTreeSet::new()),
            lazy_quit: false,
            seen_error: Arc::new(AtomicBool::new(false)),
        }
    }

    pub fn verify_except(txns_to_skip: Vec<Version>) -> Self {
        Self::Verify {
            txns_to_skip: Arc::new(txns_to_skip.into_iter().collect()),
            lazy_quit: false,
            seen_error: Arc::new(AtomicBool::new(false)),
        }
    }

    pub fn txns_to_skip(&self) -> Arc<BTreeSet<Version>> {
        match self {
            VerifyExecutionMode::NoVerify => Arc::new(BTreeSet::new()),
            VerifyExecutionMode::Verify { txns_to_skip, .. } => txns_to_skip.clone(),
        }
    }

    pub fn set_lazy_quit(mut self, is_lazy_quit: bool) -> Self {
        if let Self::Verify {
            ref mut lazy_quit, ..
        } = self
        {
            *lazy_quit = is_lazy_quit
        }
        self
    }

    pub fn is_lazy_quit(&self) -> bool {
        match self {
            VerifyExecutionMode::NoVerify => false,
            VerifyExecutionMode::Verify { lazy_quit, .. } => *lazy_quit,
        }
    }

    pub fn mark_seen_error(&self) {
        match self {
            VerifyExecutionMode::NoVerify => unreachable!("Should not call in no-verify mode."),
            VerifyExecutionMode::Verify { seen_error, .. } => {
                seen_error.store(true, Ordering::Relaxed)
            },
        }
    }

    pub fn should_verify(&self) -> bool {
        !matches!(self, Self::NoVerify)
    }

    pub fn seen_error(&self) -> bool {
        match self {
            VerifyExecutionMode::NoVerify => false,
            VerifyExecutionMode::Verify { seen_error, .. } => seen_error.load(Ordering::Relaxed),
        }
    }
}

pub trait TransactionReplayer: Send {
    fn enqueue_chunks(
        &self,
        transactions: Vec<Transaction>,
        transaction_infos: Vec<TransactionInfo>,
        write_sets: Vec<WriteSet>,
        event_vecs: Vec<Vec<ContractEvent>>,
        verify_execution_mode: &VerifyExecutionMode,
    ) -> Result<usize>;

    fn commit(&self) -> Result<Version>;
}

/// A structure that holds relevant information about a chunk that was committed.
#[derive(Clone)]
pub struct ChunkCommitNotification {
    pub subscribable_events: Vec<ContractEvent>,
    pub committed_transactions: Vec<Transaction>,
    pub reconfiguration_occurred: bool,
}

/// Used in both state sync and consensus to filter the txn events that should be subscribable by node components.
pub fn should_forward_to_subscription_service(event: &ContractEvent) -> bool {
    let type_tag = event.type_tag();
    type_tag == OBSERVED_JWK_UPDATED_MOVE_TYPE_TAG.deref()
        || type_tag == DKG_START_EVENT_MOVE_TYPE_TAG.deref()
        || type_tag == NEW_EPOCH_EVENT_MOVE_TYPE_TAG.deref()
        || type_tag == NEW_EPOCH_EVENT_V2_MOVE_TYPE_TAG.deref()
}

#[cfg(feature = "bench")]
pub fn should_forward_to_subscription_service_old(event: &ContractEvent) -> bool {
    matches!(
        event.type_tag().to_string().as_str(),
        "0x1::reconfiguration::NewEpochEvent"
            | "0x1::dkg::DKGStartEvent"
            | "\
            0x1::jwks::ObservedJWKsUpdated"
    )
}
