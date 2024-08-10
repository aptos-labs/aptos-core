// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{
    consensus_observer::{
        common::logging::{LogEntry, LogSchema},
        observer::{
            ordered_blocks::OrderedBlockStore,
            payload_store::{BlockPayloadStatus, BlockPayloadStore},
        },
        publisher::consensus_publisher::ConsensusPublisher,
    },
    payload_manager::{
        ConsensusObserverPayloadManager, DirectMempoolPayloadManager, TPayloadManager,
    },
    state_replication::StateComputerCommitCallBackType,
};
use aptos_config::config::NodeConfig;
use aptos_event_notifications::{DbBackedOnChainConfig, ReconfigNotificationListener};
use aptos_infallible::Mutex;
use aptos_logger::{error, info, warn};
use aptos_storage_interface::DbReader;
use aptos_types::{
    block_info::Round,
    epoch_state::EpochState,
    ledger_info::LedgerInfoWithSignatures,
    on_chain_config::{
        OnChainConsensusConfig, OnChainExecutionConfig, OnChainRandomnessConfig,
        RandomnessConfigMoveStruct, RandomnessConfigSeqNum, ValidatorSet,
    },
};
use futures::StreamExt;
use std::{collections::BTreeMap, sync::Arc};

pub struct ActiveObserverState {
    // The configuration of the node
    node_config: NodeConfig,

    // The consensus publisher
    consensus_publisher: Option<Arc<ConsensusPublisher>>,

    // The current epoch state
    epoch_state: Option<Arc<EpochState>>,

    // Whether quorum store is enabled for the current epoch
    quorum_store_enabled: bool,

    // The reconfiguration event listener to refresh on-chain configs
    reconfig_events: ReconfigNotificationListener<DbBackedOnChainConfig>,

    // The latest ledger info
    root: Arc<Mutex<LedgerInfoWithSignatures>>,
}

impl ActiveObserverState {
    pub fn new(
        node_config: NodeConfig,
        db_reader: Arc<dyn DbReader>,
        reconfig_events: ReconfigNotificationListener<DbBackedOnChainConfig>,
        consensus_publisher: Option<Arc<ConsensusPublisher>>,
    ) -> Self {
        // Get the latest ledger info from storage
        let root = db_reader
            .get_latest_ledger_info()
            .expect("Failed to read latest ledger info from storage!");

        // Create the active observer state
        Self {
            node_config,
            consensus_publisher,
            epoch_state: None,
            quorum_store_enabled: false,
            reconfig_events,
            root: Arc::new(Mutex::new(root)),
        }
    }

    /// Returns true iff the root epoch and round match the given values
    pub fn check_root_epoch_and_round(&self, epoch: u64, round: Round) -> bool {
        // Get the expected epoch and round
        let root = self.root();
        let expected_epoch = root.commit_info().epoch();
        let expected_round = root.commit_info().round();

        // Check if the expected epoch and round match the given values
        expected_epoch == epoch && expected_round == round
    }

    /// Creates and returns a commit callback. This will update the
    /// root ledger info and remove the blocks from the given stores.
    pub fn create_commit_callback(
        &self,
        pending_ordered_blocks: OrderedBlockStore,
        block_payload_store: BlockPayloadStore,
    ) -> StateComputerCommitCallBackType {
        // Clone the root pointer
        let root = self.root.clone();

        // Create the commit callback
        Box::new(move |blocks, ledger_info: LedgerInfoWithSignatures| {
            // Remove the committed blocks from the payload and pending stores
            block_payload_store.remove_committed_blocks(blocks);
            pending_ordered_blocks.remove_blocks_for_commit(&ledger_info);

            // Verify the ledger info is for the same epoch
            let mut root = root.lock();
            if ledger_info.commit_info().epoch() != root.commit_info().epoch() {
                warn!(
                    LogSchema::new(LogEntry::ConsensusObserver).message(&format!(
                        "Received commit callback for a different epoch! Ledger info: {:?}, Root: {:?}",
                        ledger_info.commit_info(),
                        root.commit_info()
                    ))
                );
                return;
            }

            // Update the root ledger info. Note: we only want to do this if
            // the new ledger info round is greater than the current root
            // round. Otherwise, this can race with the state sync process.
            if ledger_info.commit_info().round() > root.commit_info().round() {
                *root = ledger_info;
            }
        })
    }

    /// Returns the current epoch state
    pub fn epoch_state(&self) -> Arc<EpochState> {
        self.epoch_state
            .clone()
            .expect("The epoch state is not set! This should never happen!")
    }

    /// Returns true iff the quorum store is enabled for the current epoch
    pub fn is_quorum_store_enabled(&self) -> bool {
        self.quorum_store_enabled
    }

    /// Returns a clone of the current root ledger info
    pub fn root(&self) -> LedgerInfoWithSignatures {
        self.root.lock().clone()
    }

    /// Updates the root ledger info
    pub fn update_root(&self, new_root: LedgerInfoWithSignatures) {
        *self.root.lock() = new_root;
    }

    /// Waits for a new epoch to start (signaled by the reconfig events) and
    /// returns the new payload manager and on-chain configs (for the epoch).
    pub async fn wait_for_epoch_start(
        &mut self,
        block_payloads: Arc<
            Mutex<BTreeMap<(u64, aptos_consensus_types::common::Round), BlockPayloadStatus>>,
        >,
    ) -> (
        Arc<dyn TPayloadManager>,
        OnChainConsensusConfig,
        OnChainExecutionConfig,
        OnChainRandomnessConfig,
    ) {
        // Extract the epoch state and on-chain configs
        let (epoch_state, consensus_config, execution_config, randomness_config) =
            extract_on_chain_configs(&self.node_config, &mut self.reconfig_events).await;

        // Update the local epoch state and quorum store config
        self.epoch_state = Some(epoch_state.clone());
        self.quorum_store_enabled = consensus_config.quorum_store_enabled();
        info!(
            LogSchema::new(LogEntry::ConsensusObserver).message(&format!(
                "New epoch started: {:?}. Updated the epoch state! Quorum store enabled: {:?}",
                epoch_state.epoch, self.quorum_store_enabled,
            ))
        );

        // Create the payload manager
        let payload_manager: Arc<dyn TPayloadManager> = if self.quorum_store_enabled {
            Arc::new(ConsensusObserverPayloadManager::new(
                block_payloads,
                self.consensus_publisher.clone(),
            ))
        } else {
            Arc::new(DirectMempoolPayloadManager {})
        };

        // Return the payload manager and on-chain configs
        (
            payload_manager,
            consensus_config,
            execution_config,
            randomness_config,
        )
    }
}

/// A simple helper function that extracts the on-chain configs from the reconfig events
async fn extract_on_chain_configs(
    node_config: &NodeConfig,
    reconfig_events: &mut ReconfigNotificationListener<DbBackedOnChainConfig>,
) -> (
    Arc<EpochState>,
    OnChainConsensusConfig,
    OnChainExecutionConfig,
    OnChainRandomnessConfig,
) {
    // Fetch the next reconfiguration notification
    let reconfig_notification = reconfig_events
        .next()
        .await
        .expect("Failed to get reconfig notification!");

    // Extract the epoch state from the reconfiguration notification
    let on_chain_configs = reconfig_notification.on_chain_configs;
    let validator_set: ValidatorSet = on_chain_configs
        .get()
        .expect("Failed to get the validator set from the on-chain configs!");
    let epoch_state = Arc::new(EpochState {
        epoch: on_chain_configs.epoch(),
        verifier: (&validator_set).into(),
    });

    // Extract the consensus config (or use the default if it's missing)
    let onchain_consensus_config: anyhow::Result<OnChainConsensusConfig> = on_chain_configs.get();
    if let Err(error) = &onchain_consensus_config {
        error!(
            LogSchema::new(LogEntry::ConsensusObserver).message(&format!(
                "Failed to read on-chain consensus config! Error: {:?}",
                error
            ))
        );
    }
    let consensus_config = onchain_consensus_config.unwrap_or_default();

    // Extract the execution config (or use the default if it's missing)
    let onchain_execution_config: anyhow::Result<OnChainExecutionConfig> = on_chain_configs.get();
    if let Err(error) = &onchain_execution_config {
        error!(
            LogSchema::new(LogEntry::ConsensusObserver).message(&format!(
                "Failed to read on-chain execution config! Error: {:?}",
                error
            ))
        );
    }
    let execution_config =
        onchain_execution_config.unwrap_or_else(|_| OnChainExecutionConfig::default_if_missing());

    // Extract the randomness config sequence number (or use the default if it's missing)
    let onchain_randomness_config_seq_num: anyhow::Result<RandomnessConfigSeqNum> =
        on_chain_configs.get();
    if let Err(error) = &onchain_randomness_config_seq_num {
        error!(
            LogSchema::new(LogEntry::ConsensusObserver).message(&format!(
                "Failed to read on-chain randomness config seq num! Error: {:?}",
                error
            ))
        );
    }
    let onchain_randomness_config_seq_num = onchain_randomness_config_seq_num
        .unwrap_or_else(|_| RandomnessConfigSeqNum::default_if_missing());

    // Extract the randomness config
    let onchain_randomness_config: anyhow::Result<RandomnessConfigMoveStruct> =
        on_chain_configs.get();
    if let Err(error) = &onchain_randomness_config {
        error!(
            LogSchema::new(LogEntry::ConsensusObserver).message(&format!(
                "Failed to read on-chain randomness config! Error: {:?}",
                error
            ))
        );
    }
    let onchain_randomness_config = OnChainRandomnessConfig::from_configs(
        node_config.randomness_override_seq_num,
        onchain_randomness_config_seq_num.seq_num,
        onchain_randomness_config.ok(),
    );

    // Return the extracted epoch state and on-chain configs
    (
        epoch_state,
        consensus_config,
        execution_config,
        onchain_randomness_config,
    )
}
