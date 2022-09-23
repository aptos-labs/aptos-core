// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use crate::{
    error::StateSyncError,
    experimental::{
        buffer_manager::{OrderedBlocks, ResetAck, ResetRequest},
        errors::Error,
    },
    state_replication::{StateComputer, StateComputerCommitCallBackType},
};
use anyhow::Result;
use aptos_logger::prelude::*;
use aptos_types::ledger_info::LedgerInfoWithSignatures;
use consensus_types::block::Block;
use executor_types::Error as ExecutionError;
use fail::fail_point;
use futures::{
    channel::{mpsc::UnboundedSender, oneshot},
    SinkExt,
};
use std::sync::Arc;

#[async_trait::async_trait]
pub trait OrderingComputer: Send + Sync {
    /// Send ordered blocks to the real execution phase through the channel.
    /// A future is fulfilled after execution is completed
    async fn send_to_execution(
        &self,
        blocks: Vec<Block>,
        finality_proof: LedgerInfoWithSignatures,
        callback: StateComputerCommitCallBackType,
    ) -> Result<(), ExecutionError>;

    /// Best effort state synchronization to the given target LedgerInfo.
    /// In case of success (`Result::Ok`) the LI of storage is at the given target.
    /// In case of failure (`Result::Error`) the LI of storage remains unchanged, and the validator
    /// can assume there were no modifications to the storage made.
    async fn sync_to(&self, target: LedgerInfoWithSignatures) -> Result<(), StateSyncError>;
}

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
impl OrderingComputer for OrderingStateComputer {
    /// Send ordered blocks to the real execution phase through the channel.
    /// A future is fulfilled after execution is completed
    async fn send_to_execution(
        &self,
        blocks: Vec<Block>,
        finality_proof: LedgerInfoWithSignatures,
        callback: StateComputerCommitCallBackType,
    ) -> Result<(), ExecutionError> {
        assert!(!blocks.is_empty());

        if self
            .executor_channel
            .clone()
            .send(OrderedBlocks {
                ordered_blocks: blocks,
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
                stop: false, // epoch manager is responsible for sending stop request
            })
            .await
            .map_err(|_| Error::ResetDropped)?;
        rx.await.map_err(|_| Error::ResetDropped)?;

        // TODO: handle the sync error, should re-push the ordered blocks to buffer manager
        // when it's reset but sync fails.
        self.state_computer_for_sync.sync_to(target).await?;
        Ok(())
    }
}
