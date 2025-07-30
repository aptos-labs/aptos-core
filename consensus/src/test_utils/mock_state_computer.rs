// Copyright © Aptos Foundation
// Parts of the project are originally copyright © Meta Platforms, Inc.
// SPDX-License-Identifier: Apache-2.0

use crate::{
    error::StateSyncError, payload_manager::TPayloadManager, state_replication::StateComputer,
    transaction_deduper::TransactionDeduper, transaction_shuffler::TransactionShuffler,
};
use anyhow::{anyhow, Result};
use aptos_crypto::HashValue;
use aptos_types::{
    block_executor::config::BlockExecutorConfigFromOnchain, epoch_state::EpochState,
    ledger_info::LedgerInfoWithSignatures,
};
use std::{sync::Arc, time::Duration};

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
    async fn sync_for_duration(
        &self,
        _duration: Duration,
    ) -> Result<LedgerInfoWithSignatures, StateSyncError> {
        Err(StateSyncError::from(anyhow!(
            "sync_for_duration() is not supported by the RandomComputeResultStateComputer!"
        )))
    }

    async fn sync_to_target(
        &self,
        _target: LedgerInfoWithSignatures,
    ) -> Result<(), StateSyncError> {
        Ok(())
    }

    fn new_epoch(
        &self,
        _: &EpochState,
        _: Arc<dyn TPayloadManager>,
        _: Arc<dyn TransactionShuffler>,
        _: BlockExecutorConfigFromOnchain,
        _: Arc<dyn TransactionDeduper>,
        _: bool,
        _: bool,
        _: u8,
    ) {
    }

    fn end_epoch(&self) {}
}
