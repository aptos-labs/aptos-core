// Copyright © Aptos Foundation
// Parts of the project are originally copyright © Meta Platforms, Inc.
// SPDX-License-Identifier: Apache-2.0

use crate::{
    error::StateSyncError,
    network::{IncomingCommitRequest, IncomingRandGenRequest},
    payload_manager::{DirectMempoolPayloadManager, TPayloadManager},
    pipeline::{
        buffer_manager::OrderedBlocks, execution_client::TExecutionClient,
        pipeline_builder::PipelineBuilder, signing_phase::CommitSignerProvider,
    },
    rand::rand_gen::types::RandConfig,
    test_utils::mock_storage::MockStorage,
};
use anyhow::{anyhow, format_err, Result};
use aptos_channels::aptos_channel;
use aptos_consensus_types::{
    common::{Payload, Round},
    pipelined_block::PipelinedBlock,
    vote_data::VoteData,
    wrapped_ledger_info::WrappedLedgerInfo,
};
use aptos_crypto::{bls12381::PrivateKey, HashValue};
use aptos_executor_types::ExecutorResult;
use aptos_infallible::Mutex;
use aptos_logger::prelude::*;
use aptos_types::{
    epoch_state::EpochState,
    ledger_info::LedgerInfoWithSignatures,
    on_chain_config::{OnChainConsensusConfig, OnChainExecutionConfig, OnChainRandomnessConfig},
    transaction::SignedTransaction,
    validator_signer::ValidatorSigner,
};
use futures::{channel::mpsc, SinkExt};
use futures_channel::mpsc::UnboundedSender;
use move_core_types::account_address::AccountAddress;
use std::{collections::HashMap, sync::Arc, time::Duration};

pub struct MockExecutionClient {
    state_sync_client: mpsc::UnboundedSender<Vec<SignedTransaction>>,
    executor_channel: UnboundedSender<OrderedBlocks>,
    consensus_db: Arc<MockStorage>,
    block_cache: Mutex<HashMap<HashValue, Payload>>,
    payload_manager: Arc<dyn TPayloadManager>,
    block_store_callback:
        Mutex<Option<Box<dyn Fn(HashValue, Round, WrappedLedgerInfo) + Send + Sync>>>,
}

impl MockExecutionClient {
    pub fn new(
        state_sync_client: mpsc::UnboundedSender<Vec<SignedTransaction>>,
        executor_channel: UnboundedSender<OrderedBlocks>,
        consensus_db: Arc<MockStorage>,
    ) -> Self {
        MockExecutionClient {
            state_sync_client,
            executor_channel,
            consensus_db,
            block_cache: Mutex::new(HashMap::new()),
            payload_manager: Arc::from(DirectMempoolPayloadManager::new()),
            block_store_callback: Mutex::new(None),
        }
    }

    pub fn set_callback(
        &self,
        callback: Box<dyn Fn(HashValue, Round, WrappedLedgerInfo) + Send + Sync>,
    ) {
        *self.block_store_callback.lock() = Some(callback);
    }

    pub async fn commit_to_storage(&self, blocks: OrderedBlocks) -> ExecutorResult<()> {
        let OrderedBlocks {
            ordered_blocks,
            ordered_proof,
        } = blocks;

        self.consensus_db
            .commit_to_storage(ordered_proof.ledger_info().clone());
        if let Some(callback) = self.block_store_callback.lock().as_ref() {
            for block in &ordered_blocks {
                callback(
                    block.id(),
                    block.round(),
                    WrappedLedgerInfo::new(VoteData::dummy(), ordered_proof.clone()),
                );
            }
        }
        // mock sending commit notif to state sync
        let mut txns = vec![];
        for block in &ordered_blocks {
            self.block_cache
                .lock()
                .remove(&block.id())
                .ok_or_else(|| format_err!("Cannot find block"))?;
            let (mut payload_txns, _max_txns_from_block_to_execute, _block_gas_limit) = self
                .payload_manager
                .get_transactions(block.block(), None)
                .await?;
            txns.append(&mut payload_txns);
        }
        // they may fail during shutdown
        let _ = self.state_sync_client.unbounded_send(txns);

        Ok(())
    }
}

#[async_trait::async_trait]
impl TExecutionClient for MockExecutionClient {
    async fn start_epoch(
        &self,
        _maybe_consensus_key: Arc<PrivateKey>,
        _epoch_state: Arc<EpochState>,
        _commit_signer_provider: Arc<dyn CommitSignerProvider>,
        _payload_manager: Arc<dyn TPayloadManager>,
        _onchain_consensus_config: &OnChainConsensusConfig,
        _onchain_execution_config: &OnChainExecutionConfig,
        _onchain_randomness_config: &OnChainRandomnessConfig,
        _rand_config: Option<RandConfig>,
        _fast_rand_config: Option<RandConfig>,
        _rand_msg_rx: aptos_channel::Receiver<AccountAddress, IncomingRandGenRequest>,
        _highest_committed_round: Round,
    ) {
    }

    fn get_execution_channel(&self) -> Option<UnboundedSender<OrderedBlocks>> {
        Some(self.executor_channel.clone())
    }

    async fn finalize_order(
        &self,
        blocks: Vec<Arc<PipelinedBlock>>,
        finality_proof: WrappedLedgerInfo,
    ) -> ExecutorResult<()> {
        assert!(!blocks.is_empty());
        info!(
            "MockStateComputer commit put on queue {:?}",
            blocks.iter().map(|v| v.round()).collect::<Vec<_>>()
        );

        for block in &blocks {
            self.block_cache.lock().insert(
                block.id(),
                block
                    .payload()
                    .unwrap_or(&Payload::empty(false, true))
                    .clone(),
            );
        }

        if self
            .executor_channel
            .clone()
            .send(OrderedBlocks {
                ordered_blocks: blocks,
                ordered_proof: finality_proof.ledger_info().clone(),
            })
            .await
            .is_err()
        {
            debug!("Failed to send to buffer manager, maybe epoch ends");
        }

        Ok(())
    }

    fn send_commit_msg(
        &self,
        _peer_id: AccountAddress,
        _commit_msg: IncomingCommitRequest,
    ) -> Result<()> {
        Ok(())
    }

    async fn sync_for_duration(
        &self,
        _duration: Duration,
    ) -> Result<LedgerInfoWithSignatures, StateSyncError> {
        Err(StateSyncError::from(anyhow!(
            "sync_for_duration() is not supported by the MockExecutionClient!"
        )))
    }

    async fn sync_to_target(&self, commit: LedgerInfoWithSignatures) -> Result<(), StateSyncError> {
        debug!(
            "Fake sync to block id {}",
            commit.ledger_info().consensus_block_id()
        );
        self.consensus_db
            .commit_to_storage(commit.ledger_info().clone());
        Ok(())
    }

    async fn reset(&self, _target: &LedgerInfoWithSignatures) -> Result<()> {
        Ok(())
    }

    async fn end_epoch(&self) {}

    fn pipeline_builder(&self, _signer: Arc<ValidatorSigner>) -> PipelineBuilder {
        unimplemented!()
    }
}
