// Copyright © Aptos Foundation
// Parts of the project are originally copyright © Meta Platforms, Inc.
// SPDX-License-Identifier: Apache-2.0

use crate::{
    error::StateSyncError,
    experimental::buffer_manager::OrderedBlocks,
    payload_manager::PayloadManager,
    state_replication::{StateComputer, StateComputerCommitCallBackType},
    test_utils::mock_storage::MockStorage,
    transaction_deduper::TransactionDeduper,
    transaction_shuffler::TransactionShuffler,
};
use anyhow::{format_err, Result};
use aptos_consensus_types::{block::Block, common::Payload, executed_block::ExecutedBlock};
use aptos_crypto::HashValue;
use aptos_executor_types::{Error, StateComputeResult};
use aptos_infallible::Mutex;
use aptos_logger::prelude::*;
use aptos_types::{
    epoch_state::EpochState, ledger_info::LedgerInfoWithSignatures, transaction::SignedTransaction,
};
use futures::{channel::mpsc, SinkExt};
use futures_channel::mpsc::UnboundedSender;
use std::{collections::HashMap, sync::Arc};

pub struct MockStateComputer {
    state_sync_client: mpsc::UnboundedSender<Vec<SignedTransaction>>,
    executor_channel: UnboundedSender<OrderedBlocks>,
    consensus_db: Arc<MockStorage>,
    block_cache: Mutex<HashMap<HashValue, Payload>>,
    payload_manager: Arc<PayloadManager>,
}

impl MockStateComputer {
    pub fn new(
        state_sync_client: mpsc::UnboundedSender<Vec<SignedTransaction>>,
        executor_channel: UnboundedSender<OrderedBlocks>,
        consensus_db: Arc<MockStorage>,
    ) -> Self {
        MockStateComputer {
            state_sync_client,
            executor_channel,
            consensus_db,
            block_cache: Mutex::new(HashMap::new()),
            payload_manager: Arc::from(PayloadManager::DirectMempool),
        }
    }

    pub async fn commit_to_storage(&self, blocks: OrderedBlocks) -> Result<(), Error> {
        let OrderedBlocks {
            ordered_blocks,
            ordered_proof,
            callback,
        } = blocks;

        self.consensus_db
            .commit_to_storage(ordered_proof.ledger_info().clone());
        // mock sending commit notif to state sync
        let mut txns = vec![];
        for block in &ordered_blocks {
            self.block_cache
                .lock()
                .remove(&block.id())
                .ok_or_else(|| format_err!("Cannot find block"))?;
            let mut payload_txns = self.payload_manager.get_transactions(block.block()).await?;
            txns.append(&mut payload_txns);
        }
        // they may fail during shutdown
        let _ = self.state_sync_client.unbounded_send(txns);

        callback(
            &ordered_blocks.into_iter().map(Arc::new).collect::<Vec<_>>(),
            ordered_proof,
        );

        Ok(())
    }
}

#[async_trait::async_trait]
impl StateComputer for MockStateComputer {
    async fn compute(
        &self,
        block: &Block,
        _parent_block_id: HashValue,
    ) -> Result<StateComputeResult, Error> {
        self.block_cache.lock().insert(
            block.id(),
            block.payload().unwrap_or(&Payload::empty(false)).clone(),
        );
        let result = StateComputeResult::new_dummy();
        Ok(result)
    }

    async fn commit(
        &self,
        blocks: &[Arc<ExecutedBlock>],
        finality_proof: LedgerInfoWithSignatures,
        callback: StateComputerCommitCallBackType,
    ) -> Result<(), Error> {
        assert!(!blocks.is_empty());
        info!(
            "MockStateComputer commit put on queue {:?}",
            blocks.iter().map(|v| v.round()).collect::<Vec<_>>()
        );
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

    async fn sync_to(&self, commit: LedgerInfoWithSignatures) -> Result<(), StateSyncError> {
        debug!(
            "Fake sync to block id {}",
            commit.ledger_info().consensus_block_id()
        );
        self.consensus_db
            .commit_to_storage(commit.ledger_info().clone());
        Ok(())
    }

    fn new_epoch(
        &self,
        _: &EpochState,
        _: Arc<PayloadManager>,
        _: Arc<dyn TransactionShuffler>,
        _: Option<u64>,
        _: Arc<dyn TransactionDeduper>,
    ) {
    }

    fn end_epoch(&self) {}
}

pub struct EmptyStateComputer;

#[async_trait::async_trait]
impl StateComputer for EmptyStateComputer {
    async fn compute(
        &self,
        _block: &Block,
        _parent_block_id: HashValue,
    ) -> Result<StateComputeResult, Error> {
        Ok(StateComputeResult::new_dummy())
    }

    async fn commit(
        &self,
        _blocks: &[Arc<ExecutedBlock>],
        _commit: LedgerInfoWithSignatures,
        _call_back: StateComputerCommitCallBackType,
    ) -> Result<(), Error> {
        Ok(())
    }

    async fn sync_to(&self, _commit: LedgerInfoWithSignatures) -> Result<(), StateSyncError> {
        Ok(())
    }

    fn new_epoch(
        &self,
        _: &EpochState,
        _: Arc<PayloadManager>,
        _: Arc<dyn TransactionShuffler>,
        _: Option<u64>,
        _: Arc<dyn TransactionDeduper>,
    ) {
    }

    fn end_epoch(&self) {}
}

/// Random Compute Result State Computer
/// When compute(), if parent id is random_compute_result_root_hash, it returns Err(Error::BlockNotFound(parent_block_id))
/// Otherwise, it returns a dummy StateComputeResult with root hash as random_compute_result_root_hash.
pub struct RandomComputeResultStateComputer {
    random_compute_result_root_hash: HashValue,
}

impl RandomComputeResultStateComputer {
    pub fn new() -> Self {
        Self {
            random_compute_result_root_hash: HashValue::random(),
        }
    }

    pub fn get_root_hash(&self) -> HashValue {
        self.random_compute_result_root_hash
    }
}

#[async_trait::async_trait]
impl StateComputer for RandomComputeResultStateComputer {
    async fn compute(
        &self,
        _block: &Block,
        parent_block_id: HashValue,
    ) -> Result<StateComputeResult, Error> {
        // trapdoor for Execution Error
        if parent_block_id == self.random_compute_result_root_hash {
            Err(Error::BlockNotFound(parent_block_id))
        } else {
            Ok(StateComputeResult::new_dummy_with_root_hash(
                self.random_compute_result_root_hash,
            ))
        }
    }

    async fn commit(
        &self,
        _blocks: &[Arc<ExecutedBlock>],
        _commit: LedgerInfoWithSignatures,
        _call_back: StateComputerCommitCallBackType,
    ) -> Result<(), Error> {
        Ok(())
    }

    async fn sync_to(&self, _commit: LedgerInfoWithSignatures) -> Result<(), StateSyncError> {
        Ok(())
    }

    fn new_epoch(
        &self,
        _: &EpochState,
        _: Arc<PayloadManager>,
        _: Arc<dyn TransactionShuffler>,
        _: Option<u64>,
        _: Arc<dyn TransactionDeduper>,
    ) {
    }

    fn end_epoch(&self) {}
}
