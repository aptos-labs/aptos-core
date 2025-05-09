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
use aptos_consensus_types::wrapped_ledger_info::WrappedLedgerInfo;
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

    // Whether quorum store is enabled for the current epoch
    quorum_store_enabled: bool,

    // The reconfiguration event listener to refresh on-chain configs
    reconfig_events: ReconfigNotificationListener<DbBackedOnChainConfig>,

    // The latest ledger info
    root: Arc<Mutex<LedgerInfoWithSignatures>>,
}

impl ObserverEpochState {
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

        // Create the observer epoch state
        ObserverEpochState::new_with_root(node_config, reconfig_events, consensus_publisher, root)
    }

    /// Creates a returns a new observer epoch state with the given root ledger info
    fn new_with_root(
        node_config: NodeConfig,
        reconfig_events: ReconfigNotificationListener<DbBackedOnChainConfig>,
        consensus_publisher: Option<Arc<ConsensusPublisher>>,
        root: LedgerInfoWithSignatures,
    ) -> Self {
        Self {
            node_config,
            consensus_publisher,
            epoch_state: None,                // This is updated on epoch change
            execution_pool_window_size: None, // This is updated by the on-chain configs
            quorum_store_enabled: false,      // This is updated by the on-chain configs
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
        pending_ordered_blocks: Arc<Mutex<OrderedBlockStore>>,
        block_payload_store: Arc<Mutex<BlockPayloadStore>>,
    ) -> Box<dyn FnOnce(WrappedLedgerInfo, LedgerInfoWithSignatures) + Send + Sync> {
        // Clone the root pointer
        let root = self.root.clone();

        // Create the commit callback
        Box::new(move |_, ledger_info: LedgerInfoWithSignatures| {
            handle_committed_blocks(
                pending_ordered_blocks,
                block_payload_store,
                root,
                ledger_info,
            );
        })
    }

    /// Creates and returns the commit callback used by old pipeline.
    pub fn create_commit_callback_deprecated(
        &self,
        pending_ordered_blocks: Arc<Mutex<OrderedBlockStore>>,
        block_payload_store: Arc<Mutex<BlockPayloadStore>>,
    ) -> StateComputerCommitCallBackType {
        let root = self.root.clone();
        Box::new(move |_, ledger_info| {
            handle_committed_blocks(
                pending_ordered_blocks,
                block_payload_store,
                root,
                ledger_info,
            );
        })
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
        self.execution_pool_window_size = consensus_config.window_size();
        self.quorum_store_enabled = consensus_config.quorum_store_enabled();
        info!(
            LogSchema::new(LogEntry::ConsensusObserver).message(&format!(
                "New epoch started: {:?}. Execution pool window: {:?}. Quorum store enabled: {:?}",
                epoch_state.epoch, self.execution_pool_window_size, self.quorum_store_enabled,
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

    /// Returns whether the pipeline is enabled
    pub fn pipeline_enabled(&self) -> bool {
        self.node_config.consensus_observer.enable_pipeline
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

/// A simple helper function that handles the committed blocks
/// (as part of the commit callback).
fn handle_committed_blocks(
    pending_ordered_blocks: Arc<Mutex<OrderedBlockStore>>,
    block_payload_store: Arc<Mutex<BlockPayloadStore>>,
    root: Arc<Mutex<LedgerInfoWithSignatures>>,
    ledger_info: LedgerInfoWithSignatures,
) {
    // grab the lock for whole section to avoid inconsistent views
    let mut root = root.lock();
    // Remove the committed blocks from the payload and pending stores
    block_payload_store.lock().remove_blocks_for_epoch_round(
        ledger_info.commit_info().epoch(),
        ledger_info.commit_info().round(),
    );
    pending_ordered_blocks
        .lock()
        .remove_blocks_for_commit(&ledger_info);

    // Verify the ledger info is for the same epoch
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
        info!(
            LogSchema::new(LogEntry::ConsensusObserver).message(&format!(
                "Updating the root ledger info! Old root: (epoch: {:?}, round: {:?}). New root: (epoch: {:?}, round: {:?})",
                root.commit_info().epoch(),
                root.commit_info().round(),
                ledger_info.commit_info().epoch(),
                ledger_info.commit_info().round(),
            ))
        );
        *root = ledger_info;
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::consensus_observer::{
        network::observer_message::{BlockPayload, BlockTransactionPayload, OrderedBlock},
        observer::execution_pool::ObservedOrderedBlock,
    };
    use aptos_channels::{aptos_channel, message_queues::QueueStyle};
    use aptos_consensus_types::{
        block::Block,
        block_data::{BlockData, BlockType},
        pipelined_block::{OrderedBlockWindow, PipelinedBlock},
        quorum_cert::QuorumCert,
    };
    use aptos_crypto::HashValue;
    use aptos_event_notifications::ReconfigNotification;
    use aptos_types::{
        aggregate_signature::AggregateSignature, block_info::BlockInfo, ledger_info::LedgerInfo,
        transaction::Version,
    };

    #[test]
    fn test_check_root_epoch_and_round() {
        // Create a root ledger info
        let epoch = 10;
        let round = 5;
        let root = create_ledger_info(epoch, round);

        // Create the observer epoch state
        let (_, reconfig_events) = create_reconfig_notifier_and_listener();
        let observer_state =
            ObserverEpochState::new_with_root(NodeConfig::default(), reconfig_events, None, root);

        // Check the root epoch and round
        assert!(observer_state.check_root_epoch_and_round(epoch, round));
        assert!(!observer_state.check_root_epoch_and_round(epoch, round + 1));
        assert!(!observer_state.check_root_epoch_and_round(epoch + 1, round));

        // Update the root ledger info
        let new_epoch = epoch + 10;
        let new_round = round + 100;
        let new_root = create_ledger_info(new_epoch, new_round);
        observer_state.update_root(new_root.clone());

        // Check the updated root epoch and round
        assert!(!observer_state.check_root_epoch_and_round(epoch, round));
        assert!(observer_state.check_root_epoch_and_round(new_epoch, new_round));
    }

    #[test]
    fn test_get_and_update_root() {
        // Create a root ledger info
        let epoch = 100;
        let round = 50;
        let root = create_ledger_info(epoch, round);

        // Create the observer epoch state
        let (_, reconfig_events) = create_reconfig_notifier_and_listener();
        let observer_state = ObserverEpochState::new_with_root(
            NodeConfig::default(),
            reconfig_events,
            None,
            root.clone(),
        );

        // Check the root ledger info
        assert_eq!(observer_state.root(), root);

        // Update the root ledger info
        let new_root = create_ledger_info(epoch, round + 1000);
        observer_state.update_root(new_root.clone());

        // Check the updated root ledger info
        assert_eq!(observer_state.root(), new_root);
    }

    #[test]
    fn test_handle_committed_blocks() {
        // Create a node config
        let node_config = NodeConfig::default();

        // Create the root ledger info
        let epoch = 1000;
        let round = 100;
        let root = Arc::new(Mutex::new(create_ledger_info(epoch, round)));

        // Create the ordered block store and block payload store
        let ordered_block_store = Arc::new(Mutex::new(OrderedBlockStore::new(
            node_config.consensus_observer,
        )));
        let block_payload_store = Arc::new(Mutex::new(BlockPayloadStore::new(
            node_config.consensus_observer,
        )));

        // Handle the committed blocks at the wrong epoch and verify the root is not updated
        handle_committed_blocks(
            ordered_block_store.clone(),
            block_payload_store.clone(),
            root.clone(),
            create_ledger_info(epoch + 1, round + 1),
        );
        assert_eq!(root.lock().commit_info().epoch(), epoch);

        // Handle the committed blocks at the wrong round and verify the root is not updated
        handle_committed_blocks(
            ordered_block_store.clone(),
            block_payload_store.clone(),
            root.clone(),
            create_ledger_info(epoch, round - 1),
        );
        assert_eq!(root.lock().commit_info().round(), round);

        // Add pending ordered blocks
        let num_ordered_blocks = 10;
        let ordered_blocks = create_and_add_ordered_blocks(
            ordered_block_store.clone(),
            num_ordered_blocks,
            epoch,
            round,
        );

        // Add block payloads for the ordered blocks
        for ordered_block in &ordered_blocks {
            create_and_add_payloads_for_ordered_block(block_payload_store.clone(), ordered_block);
        }

        // Create the commit ledger info (for the second to last block)
        let commit_round = round + (num_ordered_blocks as Round) - 2;
        let committed_ledger_info = create_ledger_info(epoch, commit_round);

        // Create the committed blocks and ledger info
        let mut committed_blocks = vec![];
        for ordered_block in ordered_blocks.iter().take(num_ordered_blocks - 1) {
            let pipelined_block = create_pipelined_block(ordered_block.blocks()[0].block_info());
            committed_blocks.push(pipelined_block);
        }

        // Handle the committed blocks
        handle_committed_blocks(
            ordered_block_store.clone(),
            block_payload_store.clone(),
            root.clone(),
            committed_ledger_info.clone(),
        );

        // Verify the committed blocks are removed from the stores
        assert_eq!(ordered_block_store.lock().get_all_ordered_blocks().len(), 1);
        assert_eq!(
            block_payload_store.lock().get_block_payloads().lock().len(),
            1
        );

        // Verify the root is updated
        assert_eq!(root.lock().clone(), committed_ledger_info);
    }

    #[test]
    fn test_simple_epoch_state() {
        // Create a root ledger info
        let epoch = 10;
        let round = 5;
        let root = create_ledger_info(epoch, round);

        // Create the observer epoch state
        let (_, reconfig_events) = create_reconfig_notifier_and_listener();
        let mut observer_state =
            ObserverEpochState::new_with_root(NodeConfig::default(), reconfig_events, None, root);

        // Verify that the execution pool window size is not set
        assert!(observer_state.execution_pool_window_size().is_none());

        // Verify that quorum store is not enabled
        assert!(!observer_state.is_quorum_store_enabled());

        // Manually update the epoch state, execution pool window, and quorum store flag
        let epoch_state = Arc::new(EpochState::empty());
        observer_state.epoch_state = Some(epoch_state.clone());
        observer_state.execution_pool_window_size = Some(1);
        observer_state.quorum_store_enabled = true;

        // Verify the epoch state and quorum store flag are updated
        assert_eq!(observer_state.epoch_state(), epoch_state);
        assert_eq!(observer_state.execution_pool_window_size(), Some(1));
        assert!(observer_state.is_quorum_store_enabled());
    }

    /// Creates and adds the specified number of ordered blocks to the ordered blocks
    fn create_and_add_ordered_blocks(
        ordered_block_store: Arc<Mutex<OrderedBlockStore>>,
        num_ordered_blocks: usize,
        epoch: u64,
        starting_round: Round,
    ) -> Vec<OrderedBlock> {
        let mut ordered_blocks = vec![];
        for i in 0..num_ordered_blocks {
            // Create a new block info
            let round = starting_round + (i as Round);
            let block_info = BlockInfo::new(
                epoch,
                round,
                HashValue::random(),
                HashValue::random(),
                i as Version,
                i as u64,
                None,
            );

            // Create a pipelined block
            let block_data = BlockData::new_for_testing(
                block_info.epoch(),
                block_info.round(),
                block_info.timestamp_usecs(),
                QuorumCert::dummy(),
                BlockType::Genesis,
            );
            let block = Block::new_for_testing(block_info.id(), block_data, None);
            let pipelined_block = Arc::new(PipelinedBlock::new_ordered(
                block,
                OrderedBlockWindow::empty(),
            ));

            // Create an ordered block
            let blocks = vec![pipelined_block];
            let ordered_proof =
                create_ledger_info(epoch, i as aptos_consensus_types::common::Round);
            let ordered_block = OrderedBlock::new(blocks, ordered_proof);

            // Create an observed ordered block
            let observed_ordered_block =
                ObservedOrderedBlock::new_for_testing(ordered_block.clone());

            // Insert the block into the ordered block store
            ordered_block_store
                .lock()
                .insert_ordered_block(observed_ordered_block.clone());

            // Add the block to the ordered blocks
            ordered_blocks.push(ordered_block);
        }

        ordered_blocks
    }

    /// Creates and adds payloads for the ordered block
    fn create_and_add_payloads_for_ordered_block(
        block_payload_store: Arc<Mutex<BlockPayloadStore>>,
        ordered_block: &OrderedBlock,
    ) {
        for block in ordered_block.blocks() {
            let block_payload =
                BlockPayload::new(block.block_info(), BlockTransactionPayload::empty());
            block_payload_store
                .lock()
                .insert_block_payload(block_payload, true);
        }
    }

    /// Creates and returns a new ledger info with the specified epoch and round
    fn create_ledger_info(
        epoch: u64,
        round: aptos_consensus_types::common::Round,
    ) -> LedgerInfoWithSignatures {
        LedgerInfoWithSignatures::new(
            LedgerInfo::new(
                BlockInfo::random_with_epoch(epoch, round),
                HashValue::random(),
            ),
            AggregateSignature::empty(),
        )
    }

    /// Creates and returns a new pipelined block with the given block info
    fn create_pipelined_block(block_info: BlockInfo) -> Arc<PipelinedBlock> {
        let block_data = BlockData::new_for_testing(
            block_info.epoch(),
            block_info.round(),
            block_info.timestamp_usecs(),
            QuorumCert::dummy(),
            BlockType::Genesis,
        );
        let block = Block::new_for_testing(block_info.id(), block_data, None);
        Arc::new(PipelinedBlock::new_ordered(
            block,
            OrderedBlockWindow::empty(),
        ))
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
