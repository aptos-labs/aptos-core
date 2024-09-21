// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{
    consensus_observer::{
        common::{
            logging::{LogEntry, LogSchema},
            metrics,
        },
        network::{
            network_handler::ConsensusObserverNetworkMessage,
            observer_client::ConsensusObserverClient,
            observer_message::{
                BlockPayload, CommitDecision, ConsensusObserverDirectSend,
                ConsensusObserverMessage, OrderedBlock,
            },
        },
        observer::{
            active_state::ActiveObserverState, ordered_blocks::OrderedBlockStore,
            payload_store::BlockPayloadStore, pending_blocks::PendingBlockStore,
            subscription_manager::SubscriptionManager,
        },
        publisher::consensus_publisher::ConsensusPublisher,
    },
    dag::DagCommitSigner,
    network::{IncomingCommitRequest, IncomingRandGenRequest},
    network_interface::CommitMessage,
    pipeline::execution_client::TExecutionClient,
};
use aptos_channels::{aptos_channel, aptos_channel::Receiver, message_queues::QueueStyle};
use aptos_config::{
    config::{ConsensusObserverConfig, NodeConfig},
    network_id::PeerNetworkId,
};
use aptos_consensus_types::{pipeline, pipelined_block::PipelinedBlock};
use aptos_crypto::{bls12381, Genesis};
use aptos_event_notifications::{DbBackedOnChainConfig, ReconfigNotificationListener};
use aptos_infallible::Mutex;
use aptos_logger::{debug, error, info, warn};
use aptos_network::{
    application::interface::NetworkClient, protocols::wire::handshake::v1::ProtocolId,
};
use aptos_reliable_broadcast::DropGuard;
use aptos_storage_interface::DbReader;
use aptos_time_service::TimeService;
use aptos_types::{
    block_info::{BlockInfo, Round},
    epoch_state::EpochState,
    validator_signer::ValidatorSigner,
};
use futures::{
    future::{AbortHandle, Abortable},
    StreamExt,
};
use futures_channel::oneshot;
use move_core_types::account_address::AccountAddress;
use std::{sync::Arc, time::Duration};
use tokio::{sync::mpsc::UnboundedSender, time::interval};
use tokio_stream::wrappers::IntervalStream;

// Whether to log messages at the info level (useful for debugging)
const LOG_MESSAGES_AT_INFO_LEVEL: bool = true;

/// The consensus observer receives consensus updates and propagates them to the execution pipeline
pub struct ConsensusObserver {
    // The currently active observer state (e.g., epoch and root)
    active_observer_state: ActiveObserverState,

    // The block payload store (containing the block transaction payloads)
    block_payload_store: Arc<Mutex<BlockPayloadStore>>,

    // The ordered block store (containing ordered blocks that are ready for execution)
    ordered_block_store: Arc<Mutex<OrderedBlockStore>>,

    // The pending block store (containing pending blocks that are without payloads)
    pending_block_store: Arc<Mutex<PendingBlockStore>>,

    // The execution client to the buffer manager
    execution_client: Arc<dyn TExecutionClient>,

    // The sender to notify the observer that state syncing to the (epoch, round) has completed
    sync_notification_sender: UnboundedSender<(u64, Round)>,

    // If the sync handle is set it indicates that we're in state sync mode.
    // The flag indicates if we're waiting to transition to a new epoch.
    sync_handle: Option<(DropGuard, bool)>,

    // The consensus observer subscription manager
    subscription_manager: SubscriptionManager,
}

impl ConsensusObserver {
    pub fn new(
        node_config: NodeConfig,
        consensus_observer_client: Arc<
            ConsensusObserverClient<NetworkClient<ConsensusObserverMessage>>,
        >,
        db_reader: Arc<dyn DbReader>,
        execution_client: Arc<dyn TExecutionClient>,
        sync_notification_sender: UnboundedSender<(u64, Round)>,
        reconfig_events: Option<ReconfigNotificationListener<DbBackedOnChainConfig>>,
        consensus_publisher: Option<Arc<ConsensusPublisher>>,
        time_service: TimeService,
    ) -> Self {
        // Get the consensus observer config
        let consensus_observer_config = node_config.consensus_observer;

        // Create the subscription manager
        let subscription_manager = SubscriptionManager::new(
            consensus_observer_client,
            consensus_observer_config,
            consensus_publisher.clone(),
            db_reader.clone(),
            time_service.clone(),
        );

        // Create the active observer state
        let reconfig_events =
            reconfig_events.expect("Reconfig events should exist for the consensus observer!");
        let active_observer_state =
            ActiveObserverState::new(node_config, db_reader, reconfig_events, consensus_publisher);

        // Create the block and payload stores
        let ordered_block_store = OrderedBlockStore::new(consensus_observer_config);
        let block_payload_store = BlockPayloadStore::new(consensus_observer_config);
        let pending_block_store = PendingBlockStore::new(consensus_observer_config);

        // Create the consensus observer
        Self {
            active_observer_state,
            ordered_block_store: Arc::new(Mutex::new(ordered_block_store)),
            block_payload_store: Arc::new(Mutex::new(block_payload_store)),
            pending_block_store: Arc::new(Mutex::new(pending_block_store)),
            execution_client,
            sync_notification_sender,
            sync_handle: None,
            subscription_manager,
        }
    }

    /// Returns true iff all payloads exist for the given blocks
    fn all_payloads_exist(&self, blocks: &[Arc<PipelinedBlock>]) -> bool {
        // If quorum store is disabled, all payloads exist (they're already in the blocks)
        if !self.active_observer_state.is_quorum_store_enabled() {
            return true;
        }

        // Otherwise, check if all the payloads exist in the payload store
        self.block_payload_store.lock().all_payloads_exist(blocks)
    }

    /// Checks the progress of the consensus observer
    async fn check_progress(&mut self) {
        debug!(LogSchema::new(LogEntry::ConsensusObserver)
            .message("Checking consensus observer progress!"));

        // If we're in state sync mode, we should wait for state sync to complete
        if self.in_state_sync_mode() {
            info!(
                LogSchema::new(LogEntry::ConsensusObserver).message(&format!(
                    "Waiting for state sync to reach target: {:?}!",
                    self.active_observer_state.root().commit_info()
                ))
            );
            return;
        }

        // Otherwise, check the health of the active subscriptions
        if let Err(error) = self
            .subscription_manager
            .check_and_manage_subscriptions()
            .await
        {
            // Log the failure and clear the pending block state
            warn!(LogSchema::new(LogEntry::ConsensusObserver)
                .message(&format!("Subscription checks failed! Error: {:?}", error)));
            self.clear_pending_block_state().await;
        }
    }

    /// Clears the pending block state (this is useful for changing
    /// subscriptions, where we want to wipe all state and restart).
    async fn clear_pending_block_state(&self) {
        // Clear the payload store
        self.block_payload_store.lock().clear_all_payloads();

        // Clear the pending blocks
        self.pending_block_store.lock().clear_missing_blocks();

        // Clear the ordered blocks
        self.ordered_block_store.lock().clear_all_ordered_blocks();

        // Reset the execution pipeline for the root
        let root = self.active_observer_state.root();
        if let Err(error) = self.execution_client.reset(&root).await {
            error!(
                LogSchema::new(LogEntry::ConsensusObserver).message(&format!(
                    "Failed to reset the execution pipeline for the root! Error: {:?}",
                    error
                ))
            );
        }

        // Increment the cleared block state counter
        metrics::increment_counter_without_labels(&metrics::OBSERVER_CLEARED_BLOCK_STATE);
    }

    /// Finalizes the ordered block by sending it to the execution pipeline
    async fn finalize_ordered_block(&mut self, ordered_block: OrderedBlock) {
        info!(
            LogSchema::new(LogEntry::ConsensusObserver).message(&format!(
                "Forwarding ordered blocks to the execution pipeline: {}",
                ordered_block.proof_block_info()
            ))
        );

        // Create the commit callback (to be called after the execution pipeline)
        let commit_callback = self.active_observer_state.create_commit_callback(
            self.ordered_block_store.clone(),
            self.block_payload_store.clone(),
        );

        // Send the ordered block to the execution pipeline
        if let Err(error) = self
            .execution_client
            .finalize_order(
                ordered_block.blocks(),
                ordered_block.ordered_proof().clone(),
                commit_callback,
            )
            .await
        {
            error!(
                LogSchema::new(LogEntry::ConsensusObserver).message(&format!(
                    "Failed to finalize ordered block! Error: {:?}",
                    error
                ))
            );
        }
    }

    /// Forwards the commit decision to the execution pipeline
    fn forward_commit_decision(&self, commit_decision: CommitDecision) {
        // Create a dummy RPC message
        let (response_sender, _response_receiver) = oneshot::channel();
        let commit_request = IncomingCommitRequest {
            req: CommitMessage::Decision(pipeline::commit_decision::CommitDecision::new(
                commit_decision.commit_proof().clone(),
            )),
            protocol: ProtocolId::ConsensusDirectSendCompressed,
            response_sender,
        };

        // Send the message to the execution client
        if let Err(error) = self
            .execution_client
            .send_commit_msg(AccountAddress::ONE, commit_request)
        {
            error!(
                LogSchema::new(LogEntry::ConsensusObserver).message(&format!(
                    "Failed to send commit decision to the execution pipeline! Error: {:?}",
                    error
                ))
            )
        };
    }

    /// Returns the current epoch state, and panics if it is not set
    fn get_epoch_state(&self) -> Arc<EpochState> {
        self.active_observer_state.epoch_state()
    }

    /// Returns the highest committed block epoch and round
    fn get_highest_committed_epoch_round(&self) -> (u64, Round) {
        if let Some(epoch_round) = self
            .ordered_block_store
            .lock()
            .get_highest_committed_epoch_round()
        {
            epoch_round
        } else {
            // Return the root epoch and round
            let root_block_info = self.active_observer_state.root().commit_info().clone();
            (root_block_info.epoch(), root_block_info.round())
        }
    }

    /// Returns the last ordered block
    fn get_last_ordered_block(&self) -> BlockInfo {
        if let Some(last_ordered_block) = self.ordered_block_store.lock().get_last_ordered_block() {
            last_ordered_block
        } else {
            // Return the root ledger info
            self.active_observer_state.root().commit_info().clone()
        }
    }

    /// Returns true iff we are waiting for state sync to complete an epoch change
    fn in_state_sync_epoch_change(&self) -> bool {
        matches!(self.sync_handle, Some((_, true)))
    }

    /// Returns true iff we are waiting for state sync to complete
    fn in_state_sync_mode(&self) -> bool {
        self.sync_handle.is_some()
    }

    /// Orders any ready pending blocks for the given epoch and round
    async fn order_ready_pending_block(&mut self, block_epoch: u64, block_round: Round) {
        // Get any ready ordered block
        let ready_ordered_block = self.pending_block_store.lock().remove_ready_block(
            block_epoch,
            block_round,
            self.block_payload_store.clone(),
        );

        // Process the ready ordered block (if it exists)
        if let Some(ready_ordered_block) = ready_ordered_block {
            self.process_ordered_block(ready_ordered_block).await;
        }
    }

    /// Processes the block payload message
    async fn process_block_payload_message(
        &mut self,
        peer_network_id: PeerNetworkId,
        block_payload: BlockPayload,
    ) {
        // Get the epoch and round for the block
        let block_epoch = block_payload.epoch();
        let block_round = block_payload.round();

        // Determine if the payload is behind the last ordered block, or if it already exists
        let last_ordered_block = self.get_last_ordered_block();
        let payload_out_of_date =
            (block_epoch, block_round) <= (last_ordered_block.epoch(), last_ordered_block.round());
        let payload_exists = self
            .block_payload_store
            .lock()
            .existing_payload_entry(&block_payload);

        // If the payload is out of date or already exists, ignore it
        if payload_out_of_date || payload_exists {
            // Update the metrics for the dropped block payload
            update_metrics_for_dropped_block_payload_message(peer_network_id, &block_payload);
            return;
        }

        // Update the metrics for the received block payload
        update_metrics_for_block_payload_message(peer_network_id, &block_payload);

        // Verify the block payload digests
        if let Err(error) = block_payload.verify_payload_digests() {
            error!(
                LogSchema::new(LogEntry::ConsensusObserver).message(&format!(
                    "Failed to verify block payload digests! Ignoring block: {:?}. Error: {:?}",
                    block_payload.block(),
                    error
                ))
            );
            return;
        }

        // If the payload is for the current epoch, verify the proof signatures
        let epoch_state = self.get_epoch_state();
        let verified_payload = if block_epoch == epoch_state.epoch {
            // Verify the block proof signatures
            if let Err(error) = block_payload.verify_payload_signatures(&epoch_state) {
                error!(
                    LogSchema::new(LogEntry::ConsensusObserver).message(&format!(
                        "Failed to verify block payload signatures! Ignoring block: {:?}. Error: {:?}",
                        block_payload.block(), error
                    ))
                );
                return;
            }

            true // We have successfully verified the signatures
        } else {
            false // We can't verify the signatures yet
        };

        // Update the payload store with the payload
        self.block_payload_store
            .lock()
            .insert_block_payload(block_payload, verified_payload);

        // Check if there are blocks that were missing payloads but are
        // now ready because of the new payload. Note: this should only
        // be done if the payload has been verified correctly.
        if verified_payload {
            self.order_ready_pending_block(block_epoch, block_round)
                .await;
        }
    }

    /// Processes the commit decision message
    fn process_commit_decision_message(
        &mut self,
        peer_network_id: PeerNetworkId,
        commit_decision: CommitDecision,
    ) {
        // Get the commit decision epoch and round
        let commit_epoch = commit_decision.epoch();
        let commit_round = commit_decision.round();

        // If the commit message is behind our highest committed block, ignore it
        if (commit_epoch, commit_round) <= self.get_highest_committed_epoch_round() {
            // Update the metrics for the dropped commit decision
            update_metrics_for_dropped_commit_decision_message(peer_network_id, &commit_decision);
            return;
        }

        // Update the metrics for the received commit decision
        update_metrics_for_commit_decision_message(peer_network_id, &commit_decision);

        // If the commit decision is for the current epoch, verify and process it
        let epoch_state = self.get_epoch_state();
        if commit_epoch == epoch_state.epoch {
            // Verify the commit decision
            if let Err(error) = commit_decision.verify_commit_proof(&epoch_state) {
                error!(
                    LogSchema::new(LogEntry::ConsensusObserver).message(&format!(
                        "Failed to verify commit decision! Ignoring: {:?}, Error: {:?}",
                        commit_decision.proof_block_info(),
                        error
                    ))
                );
                return;
            }

            // Update the pending blocks with the commit decision
            if self.process_commit_decision_for_pending_block(&commit_decision) {
                return; // The commit decision was successfully processed
            }
        }

        // TODO: identify the best way to handle an invalid commit decision
        // for a future epoch. In such cases, we currently rely on state sync.

        // Otherwise, we failed to process the commit decision. If the commit
        // is for a future epoch or round, we need to state sync.
        let last_block = self.get_last_ordered_block();
        let epoch_changed = commit_epoch > last_block.epoch();
        if epoch_changed || commit_round > last_block.round() {
            // If we're waiting for state sync to transition into a new epoch,
            // we should just wait and not issue a new state sync request.
            if self.in_state_sync_epoch_change() {
                info!(
                    LogSchema::new(LogEntry::ConsensusObserver).message(&format!(
                        "Already waiting for state sync to reach new epoch: {:?}. Dropping commit decision: {:?}!",
                        self.active_observer_state.root().commit_info(),
                        commit_decision.proof_block_info()
                    ))
                );
                return;
            }

            // Otherwise, we should start the state sync process
            info!(
                LogSchema::new(LogEntry::ConsensusObserver).message(&format!(
                    "Started syncing to {}!",
                    commit_decision.proof_block_info()
                ))
            );

            // Update the root and clear the pending blocks (up to the commit)
            self.active_observer_state
                .update_root(commit_decision.commit_proof().clone());
            self.block_payload_store
                .lock()
                .remove_blocks_for_epoch_round(commit_epoch, commit_round);
            self.ordered_block_store
                .lock()
                .remove_blocks_for_commit(commit_decision.commit_proof());

            // Start the state sync process
            let abort_handle = sync_to_commit_decision(
                commit_decision,
                commit_epoch,
                commit_round,
                self.execution_client.clone(),
                self.sync_notification_sender.clone(),
            );
            self.sync_handle = Some((DropGuard::new(abort_handle), epoch_changed));
        }
    }

    /// Processes the commit decision for the pending block and returns true iff
    /// the commit decision was successfully processed. Note: this function
    /// assumes the commit decision has already been verified.
    fn process_commit_decision_for_pending_block(&self, commit_decision: &CommitDecision) -> bool {
        // Get the pending block for the commit decision
        let pending_block = self
            .ordered_block_store
            .lock()
            .get_ordered_block(commit_decision.epoch(), commit_decision.round());

        // Process the pending block
        if let Some(pending_block) = pending_block {
            // If all payloads exist, add the commit decision to the pending blocks
            if self.all_payloads_exist(pending_block.blocks()) {
                debug!(
                    LogSchema::new(LogEntry::ConsensusObserver).message(&format!(
                        "Adding decision to pending block: {}",
                        commit_decision.proof_block_info()
                    ))
                );
                self.ordered_block_store
                    .lock()
                    .update_commit_decision(commit_decision);

                // If we are not in sync mode, forward the commit decision to the execution pipeline
                if !self.in_state_sync_mode() {
                    info!(
                        LogSchema::new(LogEntry::ConsensusObserver).message(&format!(
                            "Forwarding commit decision to the execution pipeline: {}",
                            commit_decision.proof_block_info()
                        ))
                    );
                    self.forward_commit_decision(commit_decision.clone());
                }

                return true; // The commit decision was successfully processed
            }
        }

        false // The commit decision was not processed
    }

    /// Processes a network message received by the consensus observer
    async fn process_network_message(&mut self, network_message: ConsensusObserverNetworkMessage) {
        // Unpack the network message
        let (peer_network_id, message) = network_message.into_parts();

        // Verify the message is from the peers we've subscribed to
        if let Err(error) = self
            .subscription_manager
            .verify_message_for_subscription(peer_network_id)
        {
            // Increment the rejected message counter
            metrics::increment_counter(
                &metrics::OBSERVER_REJECTED_MESSAGES,
                message.get_label(),
                &peer_network_id,
            );

            // Log the error and return
            warn!(
                LogSchema::new(LogEntry::ConsensusObserver).message(&format!(
                    "Received message that was not from an active subscription! Error: {:?}",
                    error,
                ))
            );
            return;
        }

        // Increment the received message counter
        metrics::increment_counter(
            &metrics::OBSERVER_RECEIVED_MESSAGES,
            message.get_label(),
            &peer_network_id,
        );

        // Process the message based on the type
        match message {
            ConsensusObserverDirectSend::OrderedBlock(ordered_block) => {
                self.process_ordered_block_message(peer_network_id, ordered_block)
                    .await;
            },
            ConsensusObserverDirectSend::CommitDecision(commit_decision) => {
                self.process_commit_decision_message(peer_network_id, commit_decision);
            },
            ConsensusObserverDirectSend::BlockPayload(block_payload) => {
                self.process_block_payload_message(peer_network_id, block_payload)
                    .await;
            },
        }

        // Update the metrics for the processed blocks
        self.update_processed_blocks_metrics();
    }

    /// Processes the ordered block
    async fn process_ordered_block_message(
        &mut self,
        peer_network_id: PeerNetworkId,
        ordered_block: OrderedBlock,
    ) {
        // Verify the ordered blocks before processing
        if let Err(error) = ordered_block.verify_ordered_blocks() {
            error!(
                LogSchema::new(LogEntry::ConsensusObserver).message(&format!(
                    "Failed to verify ordered blocks! Ignoring: {:?}, Error: {:?}",
                    ordered_block.proof_block_info(),
                    error
                ))
            );
            return;
        };

        // Get the epoch and round of the first block
        let first_block = ordered_block.first_block();
        let first_block_epoch_round = (first_block.epoch(), first_block.round());

        // Determine if the block is behind the last ordered block, or if it is already pending
        let last_ordered_block = self.get_last_ordered_block();
        let block_out_of_date =
            first_block_epoch_round <= (last_ordered_block.epoch(), last_ordered_block.round());
        let block_pending = self
            .pending_block_store
            .lock()
            .existing_pending_block(&ordered_block);

        // If the block is out of date or already pending, ignore it
        if block_out_of_date || block_pending {
            // Update the metrics for the dropped ordered block
            update_metrics_for_dropped_ordered_block_message(peer_network_id, &ordered_block);
            return;
        }

        // Update the metrics for the received ordered block
        update_metrics_for_ordered_block_message(peer_network_id, &ordered_block);

        // If all payloads exist, process the block. Otherwise, store it
        // in the pending block store and wait for the payloads to arrive.
        if self.all_payloads_exist(ordered_block.blocks()) {
            self.process_ordered_block(ordered_block).await;
        } else {
            self.pending_block_store
                .lock()
                .insert_pending_block(ordered_block);
        }
    }

    /// Processes the ordered block. This assumes the ordered block
    /// has been sanity checked and that all payloads exist.
    async fn process_ordered_block(&mut self, ordered_block: OrderedBlock) {
        // Verify the ordered block proof
        let epoch_state = self.get_epoch_state();
        if ordered_block.proof_block_info().epoch() == epoch_state.epoch {
            if let Err(error) = ordered_block.verify_ordered_proof(&epoch_state) {
                warn!(
                    LogSchema::new(LogEntry::ConsensusObserver).message(&format!(
                        "Failed to verify ordered proof! Ignoring: {:?}, Error: {:?}",
                        ordered_block.proof_block_info(),
                        error
                    ))
                );
                return;
            }
        } else {
            // Drop the block and log an error (the block should always be for the current epoch)
            error!(
                LogSchema::new(LogEntry::ConsensusObserver).message(&format!(
                    "Received ordered block for a different epoch! Ignoring: {:?}",
                    ordered_block.proof_block_info()
                ))
            );
            return;
        };

        // Verify the block payloads against the ordered block
        if let Err(error) = self
            .block_payload_store
            .lock()
            .verify_payloads_against_ordered_block(&ordered_block)
        {
            error!(
                LogSchema::new(LogEntry::ConsensusObserver).message(&format!(
                    "Failed to verify block payloads against ordered block! Ignoring: {:?}, Error: {:?}",
                    ordered_block.proof_block_info(),
                    error
                ))
            );
            return;
        }

        // The block was verified correctly. If the block is a child of our
        // last block, we can insert it into the ordered block store.
        if self.get_last_ordered_block().id() == ordered_block.first_block().parent_id() {
            // Insert the ordered block into the pending blocks
            self.ordered_block_store
                .lock()
                .insert_ordered_block(ordered_block.clone());

            // If we're not in sync mode, finalize the ordered blocks
            if !self.in_state_sync_mode() {
                self.finalize_ordered_block(ordered_block).await;
            }
        } else {
            warn!(
                LogSchema::new(LogEntry::ConsensusObserver).message(&format!(
                    "Parent block for ordered block is missing! Ignoring: {:?}",
                    ordered_block.proof_block_info()
                ))
            );
        }
    }

    /// Processes the sync complete notification for the given epoch and round
    async fn process_sync_notification(&mut self, epoch: u64, round: Round) {
        // Log the sync notification
        info!(
            LogSchema::new(LogEntry::ConsensusObserver).message(&format!(
                "Received sync complete notification for epoch {}, round: {}",
                epoch, round
            ))
        );

        // Verify that the sync notification is for the current epoch and round
        if !self
            .active_observer_state
            .check_root_epoch_and_round(epoch, round)
        {
            info!(
                LogSchema::new(LogEntry::ConsensusObserver).message(&format!(
                    "Received invalid sync notification for epoch: {}, round: {}! Current root: {:?}",
                    epoch, round, self.active_observer_state.root()
                ))
            );
            return;
        }

        // If the epoch has changed, end the current epoch and start the new one
        let current_epoch_state = self.get_epoch_state();
        if epoch > current_epoch_state.epoch {
            // Wait for the next epoch to start
            self.execution_client.end_epoch().await;
            self.wait_for_epoch_start().await;

            // Verify the block payloads for the new epoch
            let new_epoch_state = self.get_epoch_state();
            let verified_payload_rounds = self
                .block_payload_store
                .lock()
                .verify_payload_signatures(&new_epoch_state);

            // Order all the pending blocks that are now ready (these were buffered during state sync)
            for payload_round in verified_payload_rounds {
                self.order_ready_pending_block(new_epoch_state.epoch, payload_round)
                    .await;
            }
        };

        // Reset and drop the sync handle
        self.sync_handle = None;

        // Process all the newly ordered blocks
        let all_ordered_blocks = self.ordered_block_store.lock().get_all_ordered_blocks();
        for (_, (ordered_block, commit_decision)) in all_ordered_blocks {
            // Finalize the ordered block
            self.finalize_ordered_block(ordered_block).await;

            // If a commit decision is available, forward it to the execution pipeline
            if let Some(commit_decision) = commit_decision {
                self.forward_commit_decision(commit_decision.clone());
            }
        }
    }

    /// Updates the metrics for the processed blocks
    fn update_processed_blocks_metrics(&self) {
        // Update the payload store metrics
        self.block_payload_store
            .lock()
            .update_payload_store_metrics();

        // Update the pending block metrics
        self.pending_block_store
            .lock()
            .update_pending_blocks_metrics();

        // Update the pending block metrics
        self.ordered_block_store
            .lock()
            .update_ordered_blocks_metrics();
    }

    /// Waits for a new epoch to start
    async fn wait_for_epoch_start(&mut self) {
        // Wait for the active state epoch to update
        let block_payloads = self.block_payload_store.lock().get_block_payloads();
        let (payload_manager, consensus_config, execution_config, randomness_config) = self
            .active_observer_state
            .wait_for_epoch_start(block_payloads)
            .await;

        // Fetch the new epoch state
        let epoch_state = self.get_epoch_state();

        // Start the new epoch
        let sk = Arc::new(bls12381::PrivateKey::genesis());
        let signer = Arc::new(ValidatorSigner::new(AccountAddress::ZERO, sk.clone()));
        let dummy_signer = Arc::new(DagCommitSigner::new(signer.clone()));
        let (_, rand_msg_rx) =
            aptos_channel::new::<AccountAddress, IncomingRandGenRequest>(QueueStyle::FIFO, 1, None);
        self.execution_client
            .start_epoch(
                Some(sk),
                epoch_state.clone(),
                dummy_signer,
                payload_manager,
                &consensus_config,
                &execution_config,
                &randomness_config,
                None,
                None,
                rand_msg_rx,
                0,
            )
            .await;
    }

    /// Starts the consensus observer loop that processes incoming
    /// messages and ensures the observer is making progress.
    pub async fn start(
        mut self,
        consensus_observer_config: ConsensusObserverConfig,
        mut consensus_observer_message_receiver: Receiver<(), ConsensusObserverNetworkMessage>,
        mut sync_notification_listener: tokio::sync::mpsc::UnboundedReceiver<(u64, Round)>,
    ) {
        // Create a progress check ticker
        let mut progress_check_interval = IntervalStream::new(interval(Duration::from_millis(
            consensus_observer_config.progress_check_interval_ms,
        )))
        .fuse();

        // Wait for the epoch to start
        self.wait_for_epoch_start().await;

        // Start the consensus observer loop
        info!(LogSchema::new(LogEntry::ConsensusObserver)
            .message("Starting the consensus observer loop!"));
        loop {
            tokio::select! {
                Some(network_message) = consensus_observer_message_receiver.next() => {
                    self.process_network_message(network_message).await;
                }
                Some((epoch, round)) = sync_notification_listener.recv() => {
                    self.process_sync_notification(epoch, round).await;
                },
                _ = progress_check_interval.select_next_some() => {
                    self.check_progress().await;
                }
                else => {
                    break; // Exit the consensus observer loop
                }
            }
        }

        // Log the exit of the consensus observer loop
        error!(LogSchema::new(LogEntry::ConsensusObserver)
            .message("The consensus observer loop exited unexpectedly!"));
    }
}

/// Logs the received message using an appropriate log level
fn log_received_message(message: String) {
    // Log the message at the appropriate level
    let log_schema = LogSchema::new(LogEntry::ConsensusObserver).message(&message);
    if LOG_MESSAGES_AT_INFO_LEVEL {
        info!(log_schema);
    } else {
        debug!(log_schema);
    }
}

/// Spawns a task to sync to the given commit decision and notifies
/// the consensus observer. Also, returns an abort handle to cancel the task.
fn sync_to_commit_decision(
    commit_decision: CommitDecision,
    decision_epoch: u64,
    decision_round: Round,
    execution_client: Arc<dyn TExecutionClient>,
    sync_notification_sender: UnboundedSender<(u64, Round)>,
) -> AbortHandle {
    let (abort_handle, abort_registration) = AbortHandle::new_pair();
    tokio::spawn(Abortable::new(
        async move {
            // Sync to the commit decision
            if let Err(error) = execution_client
                .clone()
                .sync_to(commit_decision.commit_proof().clone())
                .await
            {
                warn!(
                    LogSchema::new(LogEntry::ConsensusObserver).message(&format!(
                        "Failed to sync to commit decision: {:?}! Error: {:?}",
                        commit_decision, error
                    ))
                );
            }

            // Notify the consensus observer that the sync is complete
            if let Err(error) = sync_notification_sender.send((decision_epoch, decision_round)) {
                error!(
                    LogSchema::new(LogEntry::ConsensusObserver).message(&format!(
                        "Failed to send sync notification for decision epoch: {:?}, round: {:?}! Error: {:?}",
                        decision_epoch, decision_round, error
                    ))
                );
            }
        },
        abort_registration,
    ));
    abort_handle
}

/// Updates the metrics for the received block payload message
fn update_metrics_for_block_payload_message(
    peer_network_id: PeerNetworkId,
    block_payload: &BlockPayload,
) {
    // Log the received block payload message
    let log_message = format!(
        "Received block payload: {}, from peer: {}!",
        block_payload.block(),
        peer_network_id
    );
    log_received_message(log_message);

    // Update the metrics for the received block payload
    metrics::set_gauge_with_label(
        &metrics::OBSERVER_RECEIVED_MESSAGE_ROUNDS,
        metrics::BLOCK_PAYLOAD_LABEL,
        block_payload.round(),
    );
}

/// Updates the metrics for the received commit decision message
fn update_metrics_for_commit_decision_message(
    peer_network_id: PeerNetworkId,
    commit_decision: &CommitDecision,
) {
    // Log the received commit decision message
    let log_message = format!(
        "Received commit decision: {}, from peer: {}!",
        commit_decision.proof_block_info(),
        peer_network_id
    );
    log_received_message(log_message);

    // Update the metrics for the received commit decision
    metrics::set_gauge_with_label(
        &metrics::OBSERVER_RECEIVED_MESSAGE_ROUNDS,
        metrics::COMMIT_DECISION_LABEL,
        commit_decision.round(),
    );
}

/// Updates the metrics for the dropped block payload message
fn update_metrics_for_dropped_block_payload_message(
    peer_network_id: PeerNetworkId,
    block_payload: &BlockPayload,
) {
    // Increment the dropped message counter
    metrics::increment_counter(
        &metrics::OBSERVER_DROPPED_MESSAGES,
        metrics::BLOCK_PAYLOAD_LABEL,
        &peer_network_id,
    );

    // Log the dropped block payload message
    debug!(
        LogSchema::new(LogEntry::ConsensusObserver).message(&format!(
            "Ignoring block payload message from peer: {:?}! Block epoch and round: ({}, {})",
            peer_network_id,
            block_payload.epoch(),
            block_payload.round()
        ))
    );
}

/// Updates the metrics for the dropped commit decision message
fn update_metrics_for_dropped_commit_decision_message(
    peer_network_id: PeerNetworkId,
    commit_decision: &CommitDecision,
) {
    // Increment the dropped message counter
    metrics::increment_counter(
        &metrics::OBSERVER_DROPPED_MESSAGES,
        metrics::COMMITTED_BLOCKS_LABEL,
        &peer_network_id,
    );

    // Log the dropped commit decision message
    debug!(
        LogSchema::new(LogEntry::ConsensusObserver).message(&format!(
            "Ignoring commit decision message from peer: {:?}! Commit epoch and round: ({}, {})",
            peer_network_id,
            commit_decision.epoch(),
            commit_decision.round()
        ))
    );
}

/// Updates the metrics for the dropped ordered block message
fn update_metrics_for_dropped_ordered_block_message(
    peer_network_id: PeerNetworkId,
    ordered_block: &OrderedBlock,
) {
    // Increment the dropped message counter
    metrics::increment_counter(
        &metrics::OBSERVER_DROPPED_MESSAGES,
        metrics::ORDERED_BLOCK_LABEL,
        &peer_network_id,
    );

    // Log the dropped ordered block message
    debug!(
        LogSchema::new(LogEntry::ConsensusObserver).message(&format!(
            "Ignoring ordered block message from peer: {:?}! Block epoch and round: ({}, {})",
            peer_network_id,
            ordered_block.proof_block_info().epoch(),
            ordered_block.proof_block_info().round()
        ))
    );
}

/// Updates the metrics for the received ordered block message
fn update_metrics_for_ordered_block_message(
    peer_network_id: PeerNetworkId,
    ordered_block: &OrderedBlock,
) {
    // Log the received ordered block message
    let log_message = format!(
        "Received ordered block: {}, from peer: {}!",
        ordered_block.proof_block_info(),
        peer_network_id
    );
    log_received_message(log_message);

    // Update the metrics for the received ordered block
    metrics::set_gauge_with_label(
        &metrics::OBSERVER_RECEIVED_MESSAGE_ROUNDS,
        metrics::ORDERED_BLOCK_LABEL,
        ordered_block.proof_block_info().round(),
    );
}
