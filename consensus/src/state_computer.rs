// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use crate::{
    counters,
    error::StateSyncError,
    state_replication::{StateComputer, StateComputerCommitCallBackType, TxnManager},
};
use anyhow::Result;
use aptos_crypto::HashValue;
use aptos_infallible::Mutex;
use aptos_logger::prelude::*;
use aptos_metrics::monitor;
use aptos_types::{
    account_address::AccountAddress, contract_event::ContractEvent, epoch_state::EpochState,
    ledger_info::LedgerInfoWithSignatures, transaction::Transaction,
};
use consensus_notifications::ConsensusNotificationSender;
use consensus_types::{block::Block, executed_block::ExecutedBlock};
use executor_types::{BlockExecutorTrait, Error as ExecutionError, StateComputeResult};
use fail::fail_point;
use futures::{SinkExt, StreamExt};
use std::{boxed::Box, sync::Arc};

type NotificationType = (
    Box<dyn FnOnce() + Send + Sync>,
    Vec<Transaction>,
    Vec<ContractEvent>,
);

/// Basic communication with the Execution module;
/// implements StateComputer traits.
pub struct ExecutionProxy {
    executor: Box<dyn BlockExecutorTrait>,
    mempool_notifier: Arc<dyn TxnManager>,
    state_sync_notifier: Arc<dyn ConsensusNotificationSender>,
    async_state_sync_notifier: channel::Sender<NotificationType>,
    validators: Mutex<Vec<AccountAddress>>,
}

impl ExecutionProxy {
    pub fn new(
        executor: Box<dyn BlockExecutorTrait>,
        mempool_notifier: Arc<dyn TxnManager>,
        state_sync_notifier: Arc<dyn ConsensusNotificationSender>,
        handle: &tokio::runtime::Handle,
    ) -> Self {
        let (tx, mut rx) =
            channel::new::<NotificationType>(10, &counters::PENDING_STATE_SYNC_NOTIFICATION);
        let notifier = state_sync_notifier.clone();
        handle.spawn(async move {
            while let Some((callback, txns, reconfig_events)) = rx.next().await {
                if let Err(e) = monitor!(
                    "notify_state_sync",
                    notifier.notify_new_commit(txns, reconfig_events).await
                ) {
                    error!(error = ?e, "Failed to notify state synchronizer");
                }

                callback();
            }
        });
        Self {
            executor,
            mempool_notifier,
            state_sync_notifier,
            async_state_sync_notifier: tx,
            validators: Mutex::new(vec![]),
        }
    }
}

#[async_trait::async_trait]
impl StateComputer for ExecutionProxy {
    async fn compute(
        &self,
        // The block to be executed.
        block: &Block,
        // The parent block id.
        parent_block_id: HashValue,
    ) -> Result<StateComputeResult, ExecutionError> {
        fail_point!("consensus::compute", |_| {
            Err(ExecutionError::InternalError {
                error: "Injected error in compute".into(),
            })
        });
        debug!(
            block_id = block.id(),
            parent_id = block.parent_id(),
            "Executing block",
        );

        // TODO: figure out error handling for the prologue txn
        let compute_result = monitor!(
            "execute_block",
            self.executor.execute_block(
                (
                    block.id(),
                    block.transactions_to_execute(&self.validators.lock())
                ),
                parent_block_id
            )
        )?;

        // notify mempool about failed transaction
        if let Err(e) = self
            .mempool_notifier
            .notify_failed_txn(block, &compute_result)
            .await
        {
            error!(
                error = ?e, "Failed to notify mempool of rejected txns",
            );
        }
        Ok(compute_result)
    }

    /// Send a successful commit. A future is fulfilled when the state is finalized.
    async fn commit(
        &self,
        blocks: &[Arc<ExecutedBlock>],
        finality_proof: LedgerInfoWithSignatures,
        callback: StateComputerCommitCallBackType,
    ) -> Result<(), ExecutionError> {
        let mut block_ids = Vec::new();
        let mut txns = Vec::new();
        let mut reconfig_events = Vec::new();

        for block in blocks {
            block_ids.push(block.id());
            txns.extend(block.transactions_to_commit(&self.validators.lock()));
            reconfig_events.extend(block.reconfig_event());
        }

        monitor!(
            "commit_block",
            self.executor
                .commit_blocks(block_ids, finality_proof.clone())?
        );

        let blocks = blocks.to_vec();
        let wrapped_callback = move || {
            callback(&blocks, finality_proof);
        };
        self.async_state_sync_notifier
            .clone()
            .send((Box::new(wrapped_callback), txns, reconfig_events))
            .await
            .expect("Failed to send async state sync notification");

        Ok(())
    }

    /// Synchronize to a commit that not present locally.
    async fn sync_to(&self, target: LedgerInfoWithSignatures) -> Result<(), StateSyncError> {
        fail_point!("consensus::sync_to", |_| {
            Err(anyhow::anyhow!("Injected error in sync_to").into())
        });
        // Here to start to do state synchronization where ChunkExecutor inside will
        // process chunks and commit to Storage. However, after block execution and
        // commitments, the the sync state of ChunkExecutor may be not up to date so
        // it is required to reset the cache of ChunkExecutor in State Sync
        // when requested to sync.
        let res = monitor!(
            "sync_to",
            self.state_sync_notifier.sync_to_target(target).await
        );
        // Similarily, after the state synchronization, we have to reset the cache
        // of BlockExecutor to guarantee the latest committed state is up to date.
        self.executor.reset()?;

        res.map_err(|error| {
            let anyhow_error: anyhow::Error = error.into();
            anyhow_error.into()
        })
    }

    fn new_epoch(&self, epoch_state: &EpochState) {
        *self.validators.lock() = epoch_state
            .verifier
            .get_ordered_account_addresses_iter()
            .collect();
    }
}
