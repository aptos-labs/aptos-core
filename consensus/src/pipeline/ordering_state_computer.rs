// Copyright © Aptos Foundation
// Parts of the project are originally copyright © Meta Platforms, Inc.
// SPDX-License-Identifier: Apache-2.0

use crate::{
    error::StateSyncError,
    payload_manager::PayloadManager,
    pipeline::{
        buffer_manager::{OrderedBlocks, ResetAck, ResetRequest, ResetSignal},
        errors::Error,
    },
    state_computer::PipelineExecutionResult,
    state_replication::{StateComputer, StateComputerCommitCallBackType},
    transaction_deduper::TransactionDeduper,
    transaction_shuffler::TransactionShuffler,
};
use anyhow::Result;
use aptos_consensus_types::{block::Block, executed_block::ExecutedBlock};
use aptos_crypto::HashValue;
use aptos_executor_types::ExecutorResult;
use aptos_logger::prelude::*;
use aptos_types::{
    block_executor::config::BlockExecutorConfigFromOnchain, epoch_state::EpochState,
    ledger_info::LedgerInfoWithSignatures,
};
use async_trait::async_trait;
use fail::fail_point;
use futures::{
    channel::{mpsc::UnboundedSender, oneshot},
    SinkExt,
};
use futures_channel::mpsc::unbounded;
use std::sync::Arc;

/// Ordering-only execution proxy
/// implements StateComputer traits.
/// Used only when node_config.validator.consensus.decoupled = true.
pub struct OrderingStateComputer {
    // the channel to pour vectors of blocks into
    // the real execution phase (will be handled in ExecutionPhase).
    executor_channel: UnboundedSender<OrderedBlocks>,
    state_computer_for_sync: Arc<dyn StateComputer>,
    reset_event_channel_tx: UnboundedSender<ResetRequest>,
}

impl OrderingStateComputer {
    pub fn new(
        executor_channel: UnboundedSender<OrderedBlocks>,
        state_computer_for_sync: Arc<dyn StateComputer>,
        reset_event_channel_tx: UnboundedSender<ResetRequest>,
    ) -> Self {
        Self {
            executor_channel,
            state_computer_for_sync,
            reset_event_channel_tx,
        }
    }
}

#[async_trait::async_trait]
impl StateComputer for OrderingStateComputer {
    async fn compute(
        &self,
        // The block to be executed.
        _block: &Block,
        // The parent block id.
        _parent_block_id: HashValue,
    ) -> ExecutorResult<PipelineExecutionResult> {
        // Return dummy block and bypass the execution phase.
        // This will break the e2e smoke test (for now because
        // no one is actually handling the next phase) if the
        // decoupled execution feature is turned on.
        Ok(PipelineExecutionResult::new_dummy())
    }

    /// Send ordered blocks to the real execution phase through the channel.
    /// A future is fulfilled right away when the blocks are sent into the channel.
    async fn commit(
        &self,
        blocks: &[Arc<ExecutedBlock>],
        finality_proof: LedgerInfoWithSignatures,
        callback: StateComputerCommitCallBackType,
    ) -> ExecutorResult<()> {
        assert!(!blocks.is_empty());

        for block in blocks {
            block.set_insertion_time();
        }

        if self
            .executor_channel
            .clone()
            .send(OrderedBlocks {
                ordered_blocks: blocks
                    .iter()
                    .map(|b| (**b).clone())
                    .collect::<Vec<ExecutedBlock>>(),
                ordered_proof: finality_proof,
                callback,
            })
            .await
            .is_err()
        {
            debug!("Failed to send to buffer manager, maybe epoch ends");
        }

        Ok(())
    }

    /// Synchronize to a commit that not present locally.
    async fn sync_to(&self, target: LedgerInfoWithSignatures) -> Result<(), StateSyncError> {
        fail_point!("consensus::sync_to", |_| {
            Err(anyhow::anyhow!("Injected error in sync_to").into())
        });

        // reset execution phase and commit phase
        let (tx, rx) = oneshot::channel::<ResetAck>();
        self.reset_event_channel_tx
            .clone()
            .send(ResetRequest {
                tx,
                signal: ResetSignal::TargetRound(target.commit_info().round()),
            })
            .await
            .map_err(|_| Error::ResetDropped)?;
        rx.await.map_err(|_| Error::ResetDropped)?;

        // TODO: handle the sync error, should re-push the ordered blocks to buffer manager
        // when it's reset but sync fails.
        self.state_computer_for_sync.sync_to(target).await?;
        Ok(())
    }

    fn new_epoch(
        &self,
        _: &EpochState,
        _payload_manager: Arc<PayloadManager>,
        _: Arc<dyn TransactionShuffler>,
        _: BlockExecutorConfigFromOnchain,
        _: Arc<dyn TransactionDeduper>,
    ) {
    }

    fn end_epoch(&self) {}
}

// TODO: stop using state computer for DAG state sync
pub struct DagStateSyncComputer {
    ordering_state_computer: OrderingStateComputer,
}

impl DagStateSyncComputer {
    #[allow(dead_code)]
    pub fn new(
        state_computer_for_sync: Arc<dyn StateComputer>,
        reset_event_channel_tx: UnboundedSender<ResetRequest>,
    ) -> Self {
        // note: this channel is unused
        let (sender_tx, _) = unbounded();
        Self {
            ordering_state_computer: OrderingStateComputer {
                executor_channel: sender_tx,
                state_computer_for_sync,
                reset_event_channel_tx,
            },
        }
    }
}

#[async_trait]
impl StateComputer for DagStateSyncComputer {
    async fn compute(
        &self,
        // The block that will be computed.
        _block: &Block,
        // The parent block root hash.
        _parent_block_id: HashValue,
    ) -> ExecutorResult<PipelineExecutionResult> {
        unimplemented!("method not supported")
    }

    /// Send a successful commit. A future is fulfilled when the state is finalized.
    async fn commit(
        &self,
        _blocks: &[Arc<ExecutedBlock>],
        _finality_proof: LedgerInfoWithSignatures,
        _callback: StateComputerCommitCallBackType,
    ) -> ExecutorResult<()> {
        unimplemented!("method not supported")
    }

    /// Best effort state synchronization to the given target LedgerInfo.
    /// In case of success (`Result::Ok`) the LI of storage is at the given target.
    /// In case of failure (`Result::Error`) the LI of storage remains unchanged, and the validator
    /// can assume there were no modifications to the storage made.
    async fn sync_to(&self, target: LedgerInfoWithSignatures) -> Result<(), StateSyncError> {
        self.ordering_state_computer.sync_to(target).await
    }

    // Reconfigure to execute transactions for a new epoch.
    fn new_epoch(
        &self,
        _epoch_state: &EpochState,
        _payload_manager: Arc<PayloadManager>,
        _transaction_shuffler: Arc<dyn TransactionShuffler>,
        _block_executor_onchain_config: BlockExecutorConfigFromOnchain,
        _transaction_deduper: Arc<dyn TransactionDeduper>,
    ) {
        unimplemented!("method not supported");
    }

    // Reconfigure to clear epoch state at end of epoch.
    fn end_epoch(&self) {
        unimplemented!("method not supported")
    }
}
