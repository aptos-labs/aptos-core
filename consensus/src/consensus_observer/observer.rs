// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{
    consensus_observer::{
        network::{ObserverMessage, OrderedBlock},
        publisher::Publisher,
    },
    dag::DagCommitSigner,
    network::{IncomingCommitRequest, IncomingRandGenRequest},
    network_interface::CommitMessage,
    payload_manager::PayloadManager,
    pipeline::execution_client::TExecutionClient,
    state_replication::StateComputerCommitCallBackType,
};
use aptos_channels::{aptos_channel, message_queues::QueueStyle};
use aptos_consensus_types::pipeline::commit_decision::CommitDecision;
use aptos_crypto::{bls12381, Genesis, HashValue};
use aptos_event_notifications::{DbBackedOnChainConfig, ReconfigNotificationListener};
use aptos_infallible::Mutex;
use aptos_logger::{debug, error, info};
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
use tokio::sync::oneshot as tokio_oneshot;

pub enum ObserverDataStatus {
    Requested(tokio_oneshot::Sender<(Vec<SignedTransaction>, Option<usize>)>),
    Available((Vec<SignedTransaction>, Option<usize>)),
}

/// Consensus observer, get update from upstreams and propagate to execution pipeline.
pub struct Observer {
    epoch: u64,
    // latest ledger info, updated with callback
    root: Arc<Mutex<LedgerInfoWithSignatures>>,
    // pending execute/commit blocks, also buffers when in sync mode
    pending_blocks: Arc<Mutex<BTreeMap<Round, (OrderedBlock, Option<CommitDecision>)>>>,
    // execution client to buffer manager
    execution_client: Arc<dyn TExecutionClient>,
    // Indicate if it's in state sync mode, hold the task handle.
    sync_handle: Option<DropGuard>,
    // Sender to notify the observer state sync to `(Epoch, Round)` is done.
    sync_notifier: tokio::sync::mpsc::UnboundedSender<(u64, Round)>,
    // Reconfig event listener to reload on-chain configs.
    reconfig_events: ReconfigNotificationListener<DbBackedOnChainConfig>,
    // Maps between block id and its payload, the same as payload manager returns.
    payload_store: Arc<Mutex<HashMap<HashValue, ObserverDataStatus>>>,
    // Publisher to forward payload message.
    publisher: Option<Publisher>,
}

impl Observer {
    pub fn new(
        root: LedgerInfoWithSignatures,
        execution_client: Arc<dyn TExecutionClient>,
        sync_notifier: tokio::sync::mpsc::UnboundedSender<(u64, Round)>,
        reconfig_events: ReconfigNotificationListener<DbBackedOnChainConfig>,
        publisher: Option<Publisher>,
    ) -> Self {
        Self {
            epoch: root.commit_info().epoch(),
            root: Arc::new(Mutex::new(root)),
            pending_blocks: Arc::new(Mutex::new(BTreeMap::new())),
            execution_client,
            sync_handle: None,
            sync_notifier,
            reconfig_events,
            payload_store: Arc::new(Mutex::new(HashMap::new())),
            publisher,
        }
    }

    fn last_block(&self) -> BlockInfo {
        self.pending_blocks
            .lock()
            .last_key_value()
            .as_ref()
            .map_or_else(
                || self.root.lock().commit_info().clone(),
                |(_, (last_blocks, _))| last_blocks.blocks.last().unwrap().block_info(),
            )
    }

    fn commit_callback(&self) -> StateComputerCommitCallBackType {
        let root = self.root.clone();
        let pending_blocks = self.pending_blocks.clone();
        let payload_store = self.payload_store.clone();
        Box::new(move |blocks, ledger_info: LedgerInfoWithSignatures| {
            {
                let mut payload_store = payload_store.lock();
                for block in blocks.iter() {
                    payload_store.remove(&block.id());
                }
            }
            {
                let mut pending_blocks = pending_blocks.lock();
                *pending_blocks =
                    pending_blocks.split_off(&(ledger_info.commit_info().round() + 1));
            }
            *root.lock() = ledger_info;
        })
    }

    fn forward_decision(&self, decision: CommitDecision) {
        let (tx, _rx) = oneshot::channel();
        self.execution_client
            // it's just a dummy rpc message
            .send_commit_msg(AccountAddress::ONE, IncomingCommitRequest {
                req: CommitMessage::Decision(decision),
                protocol: ProtocolId::ConsensusDirectSendCompressed,
                response_sender: tx,
            })
            .unwrap()
    }

    async fn process_ordered_block(&mut self, ordered_block: OrderedBlock) {
        let OrderedBlock {
            blocks,
            ordered_proof,
        } = ordered_block.clone();
        let last_block_id = self.last_block().id();
        // if the block is a child of the last block we have, we can insert it.
        if last_block_id == blocks.first().unwrap().parent_id() {
            info!(
                "[Observer] Add blocks to pending {}",
                ordered_proof.commit_info()
            );
            self.pending_blocks
                .lock()
                .insert(blocks.last().unwrap().round(), (ordered_block, None));
            if self.sync_handle.is_none() {
                info!("[Observer] Forward blocks {}", ordered_proof.commit_info());
                self.execution_client
                    .finalize_order(&blocks, ordered_proof, self.commit_callback())
                    .await
                    .unwrap();
            }
        } else {
            info!(
                "[Observer] Do not have parent block, Ignore {}.",
                ordered_proof.commit_info()
            );
        }
    }

    fn process_commit_decision(&mut self, decision: CommitDecision) {
        let mut pending_blocks = self.pending_blocks.lock();
        let decision_epoch = decision.ledger_info().commit_info().epoch();
        let decision_round = decision.round();
        if let Some((ordered_blocks, maybe_decision)) = pending_blocks.get_mut(&decision_round) {
            let payload_exist = {
                let payload = self.payload_store.lock();
                ordered_blocks.blocks.iter().all(|block| {
                    matches!(
                        payload.get(&block.id()),
                        Some(ObserverDataStatus::Available(_))
                    )
                })
            };
            if payload_exist {
                info!(
                    "[Observer] Add decision to pending {}",
                    decision.ledger_info().commit_info()
                );
                *maybe_decision = Some(decision.clone());
                if self.sync_handle.is_none() {
                    info!(
                        "[Observer] Forward decision to pending {}.",
                        decision.ledger_info().commit_info()
                    );
                    self.forward_decision(decision);
                }
                return;
            }
        }
        // need to drop the lock otherwise it deadlocks last_block
        drop(pending_blocks);
        // we don't advance to next epoch via commit, so it has to sync from here to enter new epoch
        if decision_epoch > self.last_block().epoch() || decision_round > self.last_block().round()
        {
            info!(
                "[Observer] Start sync to {}.",
                decision.ledger_info().commit_info()
            );
            // enter sync mode if we are missing blocks
            *self.root.lock() = decision.ledger_info().clone();
            self.pending_blocks.lock().clear();
            let execution_client = self.execution_client.clone();
            let notify_tx = self.sync_notifier.clone();
            let (abort_handle, abort_registration) = AbortHandle::new_pair();
            tokio::spawn(Abortable::new(
                async move {
                    execution_client
                        .clone()
                        .sync_to(decision.ledger_info().clone())
                        .await
                        .unwrap(); // todo: handle error
                    notify_tx.send((decision_epoch, decision_round)).unwrap();
                },
                abort_registration,
            ));
            self.sync_handle = Some(DropGuard::new(abort_handle));
        }
    }

    async fn process_sync_notify(&mut self, epoch: u64, round: Round) {
        info!(
            "[Observer] Received sync notify to epoch {}, round: {}",
            epoch, round
        );
        {
            let lock = self.root.lock();
            let expected = (lock.commit_info().epoch(), lock.commit_info().round());
            if expected != (epoch, round) {
                return;
            }
            info!("[Observer] Finish sync to {}.", lock.commit_info());
        }
        if epoch > self.epoch {
            self.execution_client.end_epoch().await;
            self.await_new_epoch().await;
        }
        self.sync_handle = None;
        let pending = self.pending_blocks.lock().clone();
        for (_, (ordered_block, maybe_decision)) in pending.into_iter() {
            let OrderedBlock {
                blocks,
                ordered_proof,
            } = ordered_block;
            self.execution_client
                .finalize_order(&blocks, ordered_proof.clone(), self.commit_callback())
                .await
                .unwrap();
            if let Some(decision) = maybe_decision {
                self.forward_decision(decision.clone());
            }
        }
    }

    async fn await_new_epoch(&mut self) {
        let reconfig_notification = self
            .reconfig_events
            .next()
            .await
            .expect("Reconfig sender dropped, unable to start new epoch");
        let payload = reconfig_notification.on_chain_configs;
        let validator_set: ValidatorSet = payload
            .get()
            .expect("failed to get ValidatorSet from payload");
        let epoch_state = Arc::new(EpochState {
            epoch: payload.epoch(),
            verifier: (&validator_set).into(),
        });
        self.epoch = payload.epoch();
        info!("[Observer] enter epoch: {}", self.epoch);
        let onchain_consensus_config: anyhow::Result<OnChainConsensusConfig> = payload.get();
        let onchain_execution_config: anyhow::Result<OnChainExecutionConfig> = payload.get();
        let onchain_randomness_config: anyhow::Result<RandomnessConfigMoveStruct> = payload.get();

        if let Err(error) = &onchain_consensus_config {
            error!("Failed to read on-chain consensus config {}", error);
        }

        if let Err(error) = &onchain_execution_config {
            error!("Failed to read on-chain execution config {}", error);
        }

        if let Err(error) = &onchain_randomness_config {
            error!("Failed to read on-chain randomness config {}", error);
        }

        let consensus_config = onchain_consensus_config.unwrap_or_default();
        let execution_config = onchain_execution_config
            .unwrap_or_else(|_| OnChainExecutionConfig::default_if_missing());
        let onchain_randomness_config = onchain_randomness_config
            .and_then(OnChainRandomnessConfig::try_from)
            .unwrap_or_else(|_| OnChainRandomnessConfig::default_if_missing());
        let signer = Arc::new(ValidatorSigner::new(
            AccountAddress::ZERO,
            bls12381::PrivateKey::genesis(),
        ));
        let dummy_signer = Arc::new(DagCommitSigner::new(signer.clone()));
        let (_, rand_msg_rx) =
            aptos_channel::new::<AccountAddress, IncomingRandGenRequest>(QueueStyle::FIFO, 1, None);
        let payload_manager = if consensus_config.quorum_store_enabled() {
            PayloadManager::Observer(self.payload_store.clone(), self.publisher.clone())
        } else {
            PayloadManager::DirectMempool
        };
        self.execution_client
            .start_epoch(
                epoch_state.clone(),
                dummy_signer,
                Arc::new(payload_manager),
                &consensus_config,
                &execution_config,
                &onchain_randomness_config,
                None,
                None,
                rand_msg_rx,
                0,
            )
            .await;
    }

    pub async fn start(
        mut self,
        mut network_events: Box<dyn Stream<Item = Event<ObserverMessage>> + Send + Unpin>,
        mut notifier_rx: tokio::sync::mpsc::UnboundedReceiver<(u64, Round)>,
    ) {
        info!("[Observer] starts.");
        self.await_new_epoch().await;
        loop {
            tokio::select! {
                Some(event) = network_events.next() => {
                    if let Event::Message(peer, msg) = event {
                         // todo: verify messages
                        match msg {
                            ObserverMessage::OrderedBlock(ordered_block) => {
                                 info!(
                                     "[Observer] received ordered block {} from {}.",
                                     ordered_block.ordered_proof.commit_info(),
                                     peer,
                                 );
                                 self.process_ordered_block(ordered_block).await;
                            }
                            ObserverMessage::CommitDecision(msg) => {
                                 info!(
                                     "[Observer] received commit decision {} from {}.",
                                     msg.ledger_info().commit_info(),
                                     peer,
                                 );
                                 self.process_commit_decision(msg);
                            }
                             ObserverMessage::Payload((block, payload)) => {
                                 info!("[Observer] received payload {} from {}", block, peer);
                                 match self.payload_store.lock().entry(block.id()) {
                                     Entry::Occupied(mut entry) => {
                                         let mut status = ObserverDataStatus::Available(payload.clone());
                                         mem::swap(entry.get_mut(), &mut status);
                                         if let ObserverDataStatus::Requested(tx) = status {
                                             tx.send(payload).unwrap();
                                         }
                                     }
                                     Entry::Vacant(entry) => {
                                         entry.insert(ObserverDataStatus::Available(payload));
                                     }
                                 }
                            }
                        }
                    } else {
                        debug!("[Observer] Received untracked event: {:?}", event);
                    }
                },
                Some((epoch, round)) = notifier_rx.recv() => {
                    self.process_sync_notify(epoch, round).await;
                },
                else => break,
            }
        }
        info!("[Observer] shuts down.");
    }
}
