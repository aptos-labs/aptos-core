// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{
    consensus_observer::{
        common::logging::{LogEntry, LogSchema},
        observer::payload_store::BlockPayloadStatus,
        publisher::consensus_publisher::ConsensusPublisher,
    },
    payload_manager::{
        ConsensusObserverPayloadManager, DirectMempoolPayloadManager, TPayloadManager,
    },
};
use aptos_config::config::NodeConfig;
use aptos_event_notifications::{DbBackedOnChainConfig, ReconfigNotificationListener};
use aptos_infallible::Mutex;
use aptos_logger::{error, info, warn};
use aptos_types::{
    epoch_state::EpochState,
    on_chain_config::{
        OnChainConsensusConfig, OnChainExecutionConfig, OnChainRandomnessConfig,
        RandomnessConfigMoveStruct, RandomnessConfigSeqNum, ValidatorSet,
    },
};
use futures::StreamExt;
use std::{collections::BTreeMap, sync::Arc};

/// The epoch state used by the consensus observer
pub struct ObserverEpochState {
    // The configuration of the node
    node_config: NodeConfig,

    // The consensus publisher
    consensus_publisher: Option<Arc<ConsensusPublisher>>,

    // The current epoch state
    epoch_state: Option<Arc<EpochState>>,

    // Execution pool window size (if none, execution pool is disabled)
    execution_pool_window_size: Option<u64>,

    // The latest on-chain configs (from the most recent reconfiguration)
    on_chain_configs: Option<(
        OnChainConsensusConfig,
        OnChainExecutionConfig,
        OnChainRandomnessConfig,
    )>,

    // The payload manager used to manage the transaction payloads
    payload_manager: Option<Arc<dyn TPayloadManager>>,

    // Whether quorum store is enabled for the current epoch
    quorum_store_enabled: bool,

    // The reconfiguration event listener to refresh on-chain configs
    reconfig_events: ReconfigNotificationListener<DbBackedOnChainConfig>,
}

impl ObserverEpochState {
    pub fn new(
        node_config: NodeConfig,
        reconfig_events: ReconfigNotificationListener<DbBackedOnChainConfig>,
        consensus_publisher: Option<Arc<ConsensusPublisher>>,
    ) -> Self {
        Self {
            node_config,
            consensus_publisher,
            epoch_state: None, // This is updated on each epoch change
            execution_pool_window_size: None, // This is updated on each epoch change
            on_chain_configs: None, // This is updated on each epoch change
            payload_manager: None, // This is updated on each epoch change
            quorum_store_enabled: false, // This is updated on each epoch change
            reconfig_events,
        }
    }

    /// Returns the current epoch state
    pub fn epoch_state(&self) -> Arc<EpochState> {
        self.epoch_state
            .clone()
            .expect("The epoch state is not set! This should never happen!")
    }

    /// Returns the execution pool window size
    pub fn execution_pool_window_size(&self) -> Option<u64> {
        self.execution_pool_window_size
    }

    /// Returns true iff the quorum store is enabled for the current epoch
    pub fn is_quorum_store_enabled(&self) -> bool {
        self.quorum_store_enabled
    }

    /// Returns the latest on-chain configs
    pub fn on_chain_configs(
        &self,
    ) -> (
        OnChainConsensusConfig,
        OnChainExecutionConfig,
        OnChainRandomnessConfig,
    ) {
        self.on_chain_configs
            .clone()
            .expect("The on-chain configs are not set! This should never happen!")
    }

    /// Returns the payload manager
    pub fn payload_manager(&self) -> Arc<dyn TPayloadManager> {
        self.payload_manager
            .clone()
            .expect("The payload manager is not set! This should never happen!")
    }

    /// Waits for a new epoch to start (signaled by the reconfig events) and
    /// returns the new payload manager and on-chain configs (for the epoch).
    pub async fn wait_for_epoch_start(
        &mut self,
        block_payloads: Arc<
            Mutex<BTreeMap<(u64, aptos_consensus_types::common::Round), BlockPayloadStatus>>,
        >,
    ) {
        // Extract the latest epoch state and on-chain configs
        let (epoch_state, consensus_config, execution_config, randomness_config) =
            extract_latest_on_chain_configs(&self.node_config, &mut self.reconfig_events).await;

        // Update the local epoch state and on-chain configs
        self.epoch_state = Some(epoch_state.clone());
        self.on_chain_configs = Some((
            consensus_config.clone(),
            execution_config.clone(),
            randomness_config.clone(),
        ));

        // Update the on-chain flags
        self.execution_pool_window_size = consensus_config.window_size();
        self.quorum_store_enabled = consensus_config.quorum_store_enabled();
        info!(
            LogSchema::new(LogEntry::ConsensusObserver).message(&format!(
                "New epoch started: {:?}. Execution pool window: {:?}. Quorum store enabled: {:?}",
                epoch_state.epoch, self.execution_pool_window_size, self.quorum_store_enabled,
            ))
        );

        // Create and update the payload manager
        let payload_manager: Arc<dyn TPayloadManager> = if self.quorum_store_enabled {
            Arc::new(ConsensusObserverPayloadManager::new(
                block_payloads,
                self.consensus_publisher.clone(),
            ))
        } else {
            Arc::new(DirectMempoolPayloadManager {})
        };
        self.payload_manager = Some(payload_manager);
    }

    /// Returns whether the pipeline is enabled
    pub fn pipeline_enabled(&self) -> bool {
        self.node_config.consensus_observer.enable_pipeline
    }
}

/// A helper function that extracts the latest on-chain configs from the reconfig events
async fn extract_latest_on_chain_configs(
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
    let epoch_state = Arc::new(EpochState::new(
        on_chain_configs.epoch(),
        (&validator_set).into(),
    ));

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
        warn!(
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

#[cfg(test)]
mod test {
    use super::*;
    use aptos_channels::{aptos_channel, message_queues::QueueStyle};
    use aptos_event_notifications::ReconfigNotification;

    #[test]
    fn test_simple_state_accessors() {
        // Create the observer epoch state
        let (_, reconfig_events) = create_reconfig_notifier_and_listener();
        let mut observer_epoch_state =
            ObserverEpochState::new(NodeConfig::default(), reconfig_events, None);

        // Verify the initial states
        assert!(observer_epoch_state.epoch_state.is_none());
        assert!(observer_epoch_state.execution_pool_window_size.is_none());
        assert!(observer_epoch_state.on_chain_configs.is_none());
        assert!(observer_epoch_state.payload_manager.is_none());
        assert!(!observer_epoch_state.quorum_store_enabled);

        // Manually update the epoch state, on-chain configs and internal states
        let epoch_state = Arc::new(EpochState::empty());
        observer_epoch_state.epoch_state = Some(epoch_state.clone());
        observer_epoch_state.execution_pool_window_size = Some(1);
        observer_epoch_state.on_chain_configs = Some((
            OnChainConsensusConfig::default(),
            OnChainExecutionConfig::Missing,
            OnChainRandomnessConfig::Off,
        ));
        observer_epoch_state.payload_manager = Some(Arc::new(DirectMempoolPayloadManager::new()));
        observer_epoch_state.quorum_store_enabled = true;

        // Verify the updated states through the accessors
        assert_eq!(observer_epoch_state.epoch_state(), epoch_state);
        assert_eq!(observer_epoch_state.execution_pool_window_size(), Some(1));
        observer_epoch_state.on_chain_configs(); // Note: the accessor will panic if this is not set
        observer_epoch_state.payload_manager(); // Note: the accessor will panic if this is not set
        assert!(observer_epoch_state.is_quorum_store_enabled());
    }

    /// Creates and returns a reconfig notifier and listener
    fn create_reconfig_notifier_and_listener() -> (
        aptos_channel::Sender<(), ReconfigNotification<DbBackedOnChainConfig>>,
        ReconfigNotificationListener<DbBackedOnChainConfig>,
    ) {
        let (notification_sender, notification_receiver) =
            aptos_channel::new(QueueStyle::LIFO, 1, None);
        let reconfig_notification_listener = ReconfigNotificationListener {
            notification_receiver,
        };

        (notification_sender, reconfig_notification_listener)
    }
}
