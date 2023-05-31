// Copyright © Aptos Foundation
// Parts of the project are originally copyright © Meta Platforms, Inc.
// SPDX-License-Identifier: Apache-2.0

use crate::{
    error::{QuorumStoreError, StateSyncError},
    payload_manager::PayloadManager,
    transaction_deduper::TransactionDeduper,
    transaction_shuffler::TransactionShuffler,
};
use anyhow::Result;
use aptos_consensus_types::{
    block::Block,
    common::{Payload, PayloadFilter},
    executed_block::ExecutedBlock,
};
use aptos_crypto::HashValue;
use aptos_executor_types::{Error as ExecutionError, StateComputeResult};
use aptos_types::{epoch_state::EpochState, ledger_info::LedgerInfoWithSignatures};
use futures::future::BoxFuture;
use std::{sync::Arc, time::Duration};

pub type StateComputerCommitCallBackType =
    Box<dyn FnOnce(&[Arc<ExecutedBlock>], LedgerInfoWithSignatures) + Send + Sync>;

/// Clients can pull information about transactions from the mempool and return
/// the retrieved information as a `Payload`.
#[async_trait::async_trait]
pub trait PayloadClient: Send + Sync {
    async fn pull_payload(
        &self,
        max_poll_time: Duration,
        max_items: u64,
        max_bytes: u64,
        exclude: PayloadFilter,
        wait_callback: BoxFuture<'static, ()>,
        pending_ordering: bool,
        pending_uncommitted_blocks: usize,
        recent_max_fill_fraction: f32,
    ) -> Result<Payload, QuorumStoreError>;

    fn trace_payloads(&self) {}
}

/// While Consensus is managing proposed blocks, `StateComputer` is managing the results of the
/// (speculative) execution of their payload.
/// StateComputer is using proposed block ids for identifying the transactions.
#[async_trait::async_trait]
pub trait StateComputer: Send + Sync {
    /// How to execute a sequence of transactions and obtain the next state. While some of the
    /// transactions succeed, some of them can fail.
    /// In case all the transactions are failed, new_state_id is equal to the previous state id.
    async fn compute(
        &self,
        // The block that will be computed.
        block: &Block,
        // The parent block root hash.
        parent_block_id: HashValue,
    ) -> Result<StateComputeResult, ExecutionError>;

    /// Send a successful commit. A future is fulfilled when the state is finalized.
    async fn commit(
        &self,
        blocks: &[Arc<ExecutedBlock>],
        finality_proof: LedgerInfoWithSignatures,
        callback: StateComputerCommitCallBackType,
    ) -> Result<(), ExecutionError>;

    /// Best effort state synchronization to the given target LedgerInfo.
    /// In case of success (`Result::Ok`) the LI of storage is at the given target.
    /// In case of failure (`Result::Error`) the LI of storage remains unchanged, and the validator
    /// can assume there were no modifications to the storage made.
    async fn sync_to(&self, target: LedgerInfoWithSignatures) -> Result<(), StateSyncError>;

    // Reconfigure to execute transactions for a new epoch.
    fn new_epoch(
        &self,
        epoch_state: &EpochState,
        payload_manager: Arc<PayloadManager>,
        transaction_shuffler: Arc<dyn TransactionShuffler>,
        block_gas_limit: Option<u64>,
        transaction_deduper: Arc<dyn TransactionDeduper>,
    );

    // Reconfigure to clear epoch state at end of epoch.
    fn end_epoch(&self);
}
