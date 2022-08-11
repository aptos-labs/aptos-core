// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use crate::error::{QuorumStoreError, StateSyncError};
use anyhow::Result;
use aptos_crypto::HashValue;
use aptos_types::{epoch_state::EpochState, ledger_info::LedgerInfoWithSignatures};
use consensus_types::{
    block::Block,
    common::{Payload, PayloadFilter},
    executed_block::ExecutedBlock,
};
use executor_types::{Error as ExecutionError, StateComputeResult};
use futures::future::BoxFuture;
use std::sync::Arc;

pub type StateComputerCommitCallBackType =
    Box<dyn FnOnce(&[Arc<ExecutedBlock>], LedgerInfoWithSignatures) + Send + Sync>;

#[async_trait::async_trait]
pub trait PayloadManager: Send + Sync {
    async fn pull_payload(
        &self,
        max_items: u64,
        max_bytes: u64,
        exclude: PayloadFilter,
        wait_callback: BoxFuture<'static, ()>,
        pending_ordering: bool,
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
    fn new_epoch(&self, epoch_state: &EpochState);
}
