// Copyright © Velor Foundation
// Parts of the project are originally copyright © Meta Platforms, Inc.
// SPDX-License-Identifier: Apache-2.0

use crate::{
    error::StateSyncError, payload_manager::TPayloadManager,
    transaction_deduper::TransactionDeduper, transaction_shuffler::TransactionShuffler,
};
use anyhow::Result;
use velor_consensus_types::pipelined_block::PipelinedBlock;
use velor_types::{
    block_executor::config::BlockExecutorConfigFromOnchain, epoch_state::EpochState,
    ledger_info::LedgerInfoWithSignatures,
};
use std::{sync::Arc, time::Duration};

pub type StateComputerCommitCallBackType =
    Box<dyn FnOnce(&[Arc<PipelinedBlock>], LedgerInfoWithSignatures) + Send + Sync>;

/// While Consensus is managing proposed blocks, `StateComputer` is managing the results of the
/// (speculative) execution of their payload.
/// StateComputer is using proposed block ids for identifying the transactions.
#[async_trait::async_trait]
pub trait StateComputer: Send + Sync {
    /// Best effort state synchronization for the specified duration.
    /// This function returns the latest synced ledger info after state syncing.
    /// Note: it is possible that state sync may run longer than the specified
    /// duration (e.g., if the node is very far behind).
    async fn sync_for_duration(
        &self,
        duration: Duration,
    ) -> Result<LedgerInfoWithSignatures, StateSyncError>;

    /// Best effort state synchronization to the given target LedgerInfo.
    /// In case of success (`Result::Ok`) the LI of storage is at the given target.
    /// In case of failure (`Result::Error`) the LI of storage remains unchanged, and the validator
    /// can assume there were no modifications to the storage made.
    async fn sync_to_target(&self, target: LedgerInfoWithSignatures) -> Result<(), StateSyncError>;

    // Reconfigure to execute transactions for a new epoch.
    fn new_epoch(
        &self,
        epoch_state: &EpochState,
        payload_manager: Arc<dyn TPayloadManager>,
        transaction_shuffler: Arc<dyn TransactionShuffler>,
        block_executor_onchain_config: BlockExecutorConfigFromOnchain,
        transaction_deduper: Arc<dyn TransactionDeduper>,
        randomness_enabled: bool,
        order_vote_enabled: bool,
        persisted_auxiliary_info_version: u8,
    );

    // Reconfigure to clear epoch state at end of epoch.
    fn end_epoch(&self);
}
