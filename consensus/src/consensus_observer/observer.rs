// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{
    consensus_observer::{
        logging::{LogEntry, LogSchema},
        network_message::{
            BlockPayload, ConsensusObserverDirectSend, ConsensusObserverMessage, OrderedBlock,
        },
        publisher::ConsensusPublisher,
    },
    dag::DagCommitSigner,
    network::{IncomingCommitRequest, IncomingRandGenRequest},
    network_interface::CommitMessage,
    payload_manager::PayloadManager,
    pipeline::execution_client::TExecutionClient,
    state_replication::StateComputerCommitCallBackType,
};
use aptos_channels::{aptos_channel, message_queues::QueueStyle};
use aptos_consensus_types::{
    pipeline::commit_decision::CommitDecision, pipelined_block::PipelinedBlock,
};
use aptos_crypto::{bls12381, Genesis, HashValue};
use aptos_event_notifications::{DbBackedOnChainConfig, ReconfigNotificationListener};
use aptos_infallible::Mutex;
use aptos_logger::{error, info};
use aptos_network::protocols::{network::Event, wire::handshake::v1::ProtocolId};
use aptos_reliable_broadcast::DropGuard;
use aptos_types::{
    block_info::{BlockInfo, Round},
    epoch_state::EpochState,
    ledger_info::LedgerInfoWithSignatures,
    on_chain_config::{
        OnChainConsensusConfig, OnChainExecutionConfig, OnChainRandomnessConfig,
        RandomnessConfigMoveStruct, ValidatorSet,
    },
    transaction::SignedTransaction,
    validator_signer::ValidatorSigner,
};
use futures::{
    future::{AbortHandle, Abortable},
    Stream, StreamExt,
};
use futures_channel::oneshot;
use move_core_types::account_address::AccountAddress;
use std::{
    collections::{hash_map::Entry, BTreeMap, HashMap},
    mem,
    sync::Arc,
};
use tokio::sync::{mpsc::UnboundedSender, oneshot as tokio_oneshot};

/// The transaction payload of each block
#[derive(Debug, Clone)]
pub struct BlockTransactionPayload {
    pub transactions: Vec<SignedTransaction>,
    pub limit: Option<usize>,
}

impl BlockTransactionPayload {
    pub fn new(transactions: Vec<SignedTransaction>, limit: Option<usize>) -> Self {
        Self {
            transactions,
            limit,
        }
    }
}

/// The status of consensus observer data
pub enum ObserverDataStatus {
    Requested(tokio_oneshot::Sender<BlockTransactionPayload>),
    Available(BlockTransactionPayload),
}

/// The consensus observer receives consensus updates and propagates them to the execution pipeline
pub struct ConsensusObserver {
    // The current epoch
    epoch: u64,
    // The latest ledger info (updated via a callback)
    root: Arc<Mutex<LedgerInfoWithSignatures>>,
    // The pending execute/commit blocks (also buffers when in sync mode)
    pending_blocks: Arc<Mutex<BTreeMap<Round, (OrderedBlock, Option<CommitDecision>)>>>,
    // The execution client to the buffer manager
    execution_client: Arc<dyn TExecutionClient>,
    // If the sync handle is set it indicates that we're in state sync mode
    sync_handle: Option<DropGuard>,
    // The sender to notify the consensus observer that state sync to the (epoch, round) is done
    sync_notification_sender: tokio::sync::mpsc::UnboundedSender<(u64, Round)>,
    // The reconfiguration event listener to refresh on-chain configs
    reconfig_events: ReconfigNotificationListener<DbBackedOnChainConfig>,
    // The payload store maps block id's to transaction payloads (the same as payload manager returns)
    payload_store: Arc<Mutex<HashMap<HashValue, ObserverDataStatus>>>,
    // The consensus publisher to forward payload messages
    consensus_publisher: Option<ConsensusPublisher>,
}

impl ConsensusObserver {
    pub fn new(
        root: LedgerInfoWithSignatures,
        execution_client: Arc<dyn TExecutionClient>,
        sync_notification_sender: tokio::sync::mpsc::UnboundedSender<(u64, Round)>,
        reconfig_events: ReconfigNotificationListener<DbBackedOnChainConfig>,
        consensus_publisher: Option<ConsensusPublisher>,
    ) -> Self {
        Self {
            epoch: root.commit_info().epoch(),
            root: Arc::new(Mutex::new(root)),
            pending_blocks: Arc::new(Mutex::new(BTreeMap::new())),
            execution_client,
            sync_handle: None,
            sync_notification_sender,
            reconfig_events,
            payload_store: Arc::new(Mutex::new(HashMap::new())),
            consensus_publisher,
        }
    }

    /// Returns the last known block
    fn get_last_block(&self) -> BlockInfo {
        if let Some((_, (last_blocks, _))) = self.pending_blocks.lock().last_key_value() {
            // Return the last block in the pending blocks
            last_blocks.blocks.last().unwrap().block_info()
        } else {
            // Return the root ledger info
            self.root.lock().commit_info().clone()
        }
    }

    /// Creates and returns a commit callback (to be called after the execution pipeline)
    fn create_commit_callback(&self) -> StateComputerCommitCallBackType {
        // Clone the root, pending blocks and payload store
        let root = self.root.clone();
        let pending_blocks = self.pending_blocks.clone();
        let payload_store = self.payload_store.clone();

        // Create the commit callback
        Box::new(move |blocks, ledger_info: LedgerInfoWithSignatures| {
            // Remove the committed blocks from the payload store
            remove_payload_blocks(payload_store, blocks);

            // Remove the committed blocks from the pending blocks
            remove_pending_blocks(pending_blocks, &ledger_info);

            // Update the root ledger info
            *root.lock() = ledger_info;
        })
    }

    /// Forwards the commit decision to the execution pipeline
    fn forward_commit_decision(&self, decision: CommitDecision) {
        // Create a dummy RPC message
        let (response_sender, _response_receiver) = oneshot::channel();
        let commit_request = IncomingCommitRequest {
            req: CommitMessage::Decision(decision),
            protocol: ProtocolId::ConsensusDirectSendCompressed,
            response_sender,
        };

        // Send the message to the execution client
        self.execution_client
            .send_commit_msg(AccountAddress::ONE, commit_request)
            .unwrap()
    }

    /// Processes the block payload
    fn process_block_payload(&mut self, block_payload: BlockPayload) {
        // Unpack the block payload
        let block = block_payload.block;
        let transactions = block_payload.transactions;
        let limit = block_payload.limit;

        // Update the payload store with the transaction payload
        let transaction_payload = BlockTransactionPayload::new(transactions, limit);
        match self.payload_store.lock().entry(block.id()) {
            Entry::Occupied(mut entry) => {
                // Get the current status of the block payload
                let mut status = ObserverDataStatus::Available(transaction_payload.clone());

                // Replace the status with the new block payload
                mem::swap(entry.get_mut(), &mut status);

                // If the status was originally requested, send the payload to the listener
                if let ObserverDataStatus::Requested(payload_sender) = status {
                    if let Err(error) = payload_sender.send(transaction_payload) {
                        error!(LogSchema::new(LogEntry::ConsensusObserver).log(&format!(
                            "Failed to send block payload to listener! Error: {:?}",
                            error
                        )));
                    }
                }
            },
            Entry::Vacant(entry) => {
                // Insert the block payload into the payload store
                entry.insert(ObserverDataStatus::Available(transaction_payload));
            },
        }
    }

    /// Processes the commit decision
    fn process_commit_decision(&mut self, commit_decision: CommitDecision) {
        // Update the pending blocks with the commit decision
        if self.process_commit_decision_for_pending_block(&commit_decision) {
            return; // The commit decision was successfully processed
        }

        // Otherwise, check if we need to state sync (i.e., the
        // commit decision is for a future epoch or round).
        let decision_epoch = commit_decision.ledger_info().commit_info().epoch();
        let decision_round = commit_decision.round();
        let last_block = self.get_last_block();
        if decision_epoch > last_block.epoch() || decision_round > last_block.round() {
            info!(LogSchema::new(LogEntry::ConsensusObserver).log(&format!(
                "Started syncing to {}!",
                commit_decision.ledger_info().commit_info()
            )));

            // Update the root and clear the pending blocks
            *self.root.lock() = commit_decision.ledger_info().clone();
            self.pending_blocks.lock().clear();

            // Start the state sync process
            let abort_handle = sync_to_commit_decision(
                commit_decision,
                decision_epoch,
                decision_round,
                self.execution_client.clone(),
                self.sync_notification_sender.clone(),
            );
            self.sync_handle = Some(DropGuard::new(abort_handle));
        }
    }

    /// Processes the commit decision for the pending block and returns iff
    /// the commit decision was successfully processed.
    fn process_commit_decision_for_pending_block(&self, commit_decision: &CommitDecision) -> bool {
        let mut pending_blocks = self.pending_blocks.lock();
        if let Some((ordered_blocks, pending_commit_decision)) =
            pending_blocks.get_mut(&commit_decision.round())
        {
            // Check if the payload already exists
            let payload_exists = {
                let payload_store = self.payload_store.lock();
                ordered_blocks.blocks.iter().all(|block| {
                    matches!(
                        payload_store.get(&block.id()),
                        Some(ObserverDataStatus::Available(_))
                    )
                })
            };

            // If the payload exists, add the commit decision to the pending blocks
            if payload_exists {
                info!(LogSchema::new(LogEntry::ConsensusObserver).log(&format!(
                    "Adding decision to pending block: {}",
                    commit_decision.ledger_info().commit_info()
                )));
                *pending_commit_decision = Some(commit_decision.clone());

                // If we are not in sync mode, forward the commit decision to the execution pipeline
                if self.sync_handle.is_none() {
                    info!(LogSchema::new(LogEntry::ConsensusObserver).log(&format!(
                        "Forwarding commit decision to the execution pipeline: {}",
                        commit_decision.ledger_info().commit_info()
                    )));
                    self.forward_commit_decision(commit_decision.clone());
                }

                return true; // The commit decision was successfully processed
            }
        }

        false // The commit decision was not processed
    }

    /// Processes the ordered block
    async fn process_ordered_block(&mut self, ordered_block: OrderedBlock) {
        // Unpack the ordered block
        let OrderedBlock {
            blocks,
            ordered_proof,
        } = ordered_block.clone();

        // If the block is a child of our last block, we can insert it
        if self.get_last_block().id() == blocks.first().unwrap().parent_id() {
            info!(LogSchema::new(LogEntry::ConsensusObserver).log(&format!(
                "Adding ordered block to the pending blocks: {}",
                ordered_proof.commit_info()
            )));

            // Insert the ordered block into the pending blocks
            self.pending_blocks
                .lock()
                .insert(blocks.last().unwrap().round(), (ordered_block, None));

            // If we are not in sync mode, forward the blocks to the execution pipeline
            if self.sync_handle.is_none() {
                info!(LogSchema::new(LogEntry::ConsensusObserver).log(&format!(
                    "Forwarding blocks to the execution pipeline: {}",
                    ordered_proof.commit_info()
                )));
                self.execution_client
                    .finalize_order(&blocks, ordered_proof, self.create_commit_callback())
                    .await
                    .unwrap();
            }
        } else {
            info!(LogSchema::new(LogEntry::ConsensusObserver).log(&format!(
                "Parent block is missing! Ignoring: {:?}",
                ordered_proof.commit_info()
            )));
        }
    }

    /// Processes the sync complete notification for the given epoch and round
    async fn process_sync_notification(&mut self, epoch: u64, round: Round) {
        // Verify that the sync notification is for the current epoch and round
        if !check_root_epoch_and_round(self.root.clone(), epoch, round) {
            info!(LogSchema::new(LogEntry::ConsensusObserver).log(&format!(
                "Received outdated sync notification for epoch: {}, round: {}! Current root: {:?}",
                epoch, round, self.root
            )));
            return;
        }

        // If the epoch has changed, end the current epoch and start the new one
        if epoch > self.epoch {
            self.execution_client.end_epoch().await;
            self.wait_for_epoch_start().await;
        }

        // Reset and drop the sync handle
        self.sync_handle = None;

        // Process the pending blocks
        let pending_blocks = self.pending_blocks.lock().clone();
        for (_, (ordered_block, commit_decision)) in pending_blocks.into_iter() {
            // Unpack the ordered block
            let OrderedBlock {
                blocks,
                ordered_proof,
            } = ordered_block;

            // Finalize the ordered block
            self.execution_client
                .finalize_order(
                    &blocks,
                    ordered_proof.clone(),
                    self.create_commit_callback(),
                )
                .await
                .unwrap();

            // If a commit decision is available, forward it to the execution pipeline
            if let Some(commit_decision) = commit_decision {
                self.forward_commit_decision(commit_decision.clone());
            }
        }
    }

    /// Waits for a new epoch to start
    async fn wait_for_epoch_start(&mut self) {
        // Extract the epoch state and on-chain configs
        let (epoch_state, consensus_config, execution_config, randomness_config) =
            extract_on_chain_configs(&mut self.reconfig_events).await;

        // Update the local epoch
        self.epoch = epoch_state.epoch;
        info!(LogSchema::new(LogEntry::ConsensusObserver)
            .log(&format!("New epoch started: {}", self.epoch)));

        // Create the payload manager
        let payload_manager = if consensus_config.quorum_store_enabled() {
            PayloadManager::ConsensusObserver(
                self.payload_store.clone(),
                self.consensus_publisher.clone(),
            )
        } else {
            PayloadManager::DirectMempool
        };

        // Start the new epoch
        let signer = Arc::new(ValidatorSigner::new(
            AccountAddress::ZERO,
            bls12381::PrivateKey::genesis(),
        ));
        let dummy_signer = Arc::new(DagCommitSigner::new(signer.clone()));
        let (_, rand_msg_rx) =
            aptos_channel::new::<AccountAddress, IncomingRandGenRequest>(QueueStyle::FIFO, 1, None);
        self.execution_client
            .start_epoch(
                epoch_state.clone(),
                dummy_signer,
                Arc::new(payload_manager),
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

    /// Starts the consensus observer process
    pub async fn start(
        mut self,
        mut network_events: Box<dyn Stream<Item = Event<ConsensusObserverMessage>> + Send + Unpin>,
        mut sync_notification_listener: tokio::sync::mpsc::UnboundedReceiver<(u64, Round)>,
    ) {
        // Wait for the epoch to start
        self.wait_for_epoch_start().await;

        // Start the consensus observer loop
        info!(LogSchema::new(LogEntry::ConsensusObserver)
            .log("Starting the consensus observer loop!"));
        loop {
            tokio::select! {
                Some(event) = network_events.next() => {
                    if let Event::Message(peer, message) = event {
                         // todo: verify messages
                        match message {
                            ConsensusObserverMessage::DirectSend(ConsensusObserverDirectSend::OrderedBlock(ordered_block)) => {
                                info!(LogSchema::new(LogEntry::ConsensusObserver).log(&format!(
                                    "Received ordered block: {}, from peer: {}!",
                                    ordered_block.ordered_proof.commit_info(),
                                    peer
                                )));
                                self.process_ordered_block(ordered_block).await;
                            },
                            ConsensusObserverMessage::DirectSend(ConsensusObserverDirectSend::CommitDecision(commit_decision)) => {
                                info!(LogSchema::new(LogEntry::ConsensusObserver).log(&format!(
                                    "Received commit decision: {}, from peer: {}!",
                                    commit_decision.ledger_info().commit_info(),
                                    peer
                                )));
                                 self.process_commit_decision(commit_decision);
                            },
                            ConsensusObserverMessage::DirectSend(ConsensusObserverDirectSend::BlockPayload(block_payload)) => {
                                info!(LogSchema::new(LogEntry::ConsensusObserver).log(&format!(
                                    "Received block payload: {}, from peer: {}!",
                                    block_payload.block,
                                    peer
                                )));
                                self.process_block_payload(block_payload);
                            }
                            _ => {
                                error!(LogSchema::new(LogEntry::ConsensusObserver)
                                    .log(&format!("Received unexpected message from peer: {}", peer)));
                            }
                        }
                    }
                },
                Some((epoch, round)) = sync_notification_listener.recv() => {
                    info!(LogSchema::new(LogEntry::ConsensusObserver).log(&format!(
                        "Received sync complete notification for epoch {}, round: {}",
                        epoch, round
                    )));
                    self.process_sync_notification(epoch, round).await;
                },
                else => break,
            }
        }

        // Log the exit of the consensus observer loop
        error!(LogSchema::new(LogEntry::ConsensusObserver)
            .log("The consensus observer loop exited unexpectedly!"));
    }
}

/// Checks that the epoch and round match the current root
fn check_root_epoch_and_round(
    root: Arc<Mutex<LedgerInfoWithSignatures>>,
    epoch: u64,
    round: Round,
) -> bool {
    // Get the expected epoch and round
    let root = root.lock();
    let expected_epoch = root.commit_info().epoch();
    let expected_round = root.commit_info().round();

    // Check if the expected epoch and round match
    expected_epoch == epoch && expected_round == round
}

/// A simple helper function that extracts the on-chain configs from the reconfig events
async fn extract_on_chain_configs(
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
        error!(LogSchema::new(LogEntry::ConsensusObserver).log(&format!(
            "Failed to read on-chain consensus config! Error: {:?}",
            error
        )));
    }
    let consensus_config = onchain_consensus_config.unwrap_or_default();

    // Extract the execution config (or use the default if it's missing)
    let onchain_execution_config: anyhow::Result<OnChainExecutionConfig> = on_chain_configs.get();
    if let Err(error) = &onchain_execution_config {
        error!(LogSchema::new(LogEntry::ConsensusObserver).log(&format!(
            "Failed to read on-chain execution config! Error: {:?}",
            error
        )));
    }
    let execution_config =
        onchain_execution_config.unwrap_or_else(|_| OnChainExecutionConfig::default_if_missing());

    // Extract the randomness config (or use the default if it's missing)
    let onchain_randomness_config: anyhow::Result<RandomnessConfigMoveStruct> =
        on_chain_configs.get();
    if let Err(error) = &onchain_randomness_config {
        error!(LogSchema::new(LogEntry::ConsensusObserver).log(&format!(
            "Failed to read on-chain randomness config! Error: {:?}",
            error
        )));
    }
    let onchain_randomness_config = onchain_randomness_config
        .and_then(OnChainRandomnessConfig::try_from)
        .unwrap_or_else(|_| OnChainRandomnessConfig::default_if_missing());

    // Return the extracted epoch state and on-chain configs
    (
        epoch_state,
        consensus_config,
        execution_config,
        onchain_randomness_config,
    )
}

/// Removes the given payload blocks from the payload store
fn remove_payload_blocks(
    payload_store: Arc<Mutex<HashMap<HashValue, ObserverDataStatus>>>,
    blocks: &[Arc<PipelinedBlock>],
) {
    let mut payload_store = payload_store.lock();
    for block in blocks.iter() {
        payload_store.remove(&block.id());
    }
}

/// Removes the pending blocks after the given ledger info
fn remove_pending_blocks(
    pending_blocks: Arc<Mutex<BTreeMap<Round, (OrderedBlock, Option<CommitDecision>)>>>,
    ledger_info: &LedgerInfoWithSignatures,
) {
    let mut pending_blocks = pending_blocks.lock();
    let split_off_round = ledger_info.commit_info().round() + 1;
    *pending_blocks = pending_blocks.split_off(&split_off_round);
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
            execution_client
                .clone()
                .sync_to(commit_decision.ledger_info().clone())
                .await
                .unwrap(); // todo: handle error
            sync_notification_sender
                .send((decision_epoch, decision_round))
                .unwrap();
        },
        abort_registration,
    ));
    abort_handle
}
