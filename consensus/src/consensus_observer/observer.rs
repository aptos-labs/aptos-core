// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{
    consensus_observer::{
        network_messages::{ObserverMessage, OrderedBlock},
        publisher::Publisher,
    },
    network::IncomingCommitRequest,
    network_interface::{CommitMessage, ConsensusMsg},
    pipeline::execution_client::TExecutionClient,
    state_replication::StateComputerCommitCallBackType,
};
use aptos_channels::aptos_channel;
use aptos_consensus_types::{
    common::Author, pipeline::commit_decision::CommitDecision, pipelined_block::PipelinedBlock,
};
use aptos_infallible::Mutex;
use aptos_logger::info;
use aptos_network::protocols::wire::handshake::v1::ProtocolId;
use aptos_reliable_broadcast::DropGuard;
use aptos_types::{
    block_info::{BlockInfo, Round},
    ledger_info::LedgerInfoWithSignatures,
};
use futures::{
    future::{AbortHandle, Abortable},
    StreamExt,
};
use futures_channel::oneshot;
use move_core_types::account_address::AccountAddress;
use std::{collections::BTreeMap, sync::Arc};

/// Consensus observer, get update from upstreams and propagate to execution pipeline.
pub struct Observer {
    // latest ledger info, updated with callback
    root: Arc<Mutex<LedgerInfoWithSignatures>>,
    // pending execute/commit blocks, also buffers when in sync mode
    pending_blocks: Arc<Mutex<BTreeMap<Round, (OrderedBlock, Option<CommitDecision>)>>>,
    // execution client to buffer manager
    execution_client: Arc<dyn TExecutionClient>,
    // Indicate if it's in state sync mode, hold the task handle.
    sync_handle: Option<DropGuard>,
    // Sender to notify the observer state sync to `Round` is done.
    sync_notifier: tokio::sync::mpsc::Sender<Round>,
    // Publisher for downstream observers
    publisher: Publisher,
}

impl Observer {
    pub fn new(
        root: LedgerInfoWithSignatures,
        execution_client: Arc<dyn TExecutionClient>,
        sync_notifier: tokio::sync::mpsc::Sender<Round>,
        publisher: Publisher,
    ) -> Self {
        Self {
            root: Arc::new(Mutex::new(root)),
            pending_blocks: Arc::new(Mutex::new(BTreeMap::new())),
            execution_client,
            sync_handle: None,
            sync_notifier,
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
        Box::new(move |_, ledger_info: LedgerInfoWithSignatures| {
            let mut pending_blocks = pending_blocks.lock();
            *pending_blocks = pending_blocks.split_off(&(ledger_info.commit_info().round() + 1));
            *root.lock() = ledger_info;
        })
    }

    fn forward_decision(&self, decision: CommitDecision) {
        let (tx, _rx) = oneshot::channel();
        self.execution_client
            // it's just a dummy rpc message
            .send_commit_msg(AccountAddress::ONE, IncomingCommitRequest {
                req: CommitMessage::Decision(decision),
                author: AccountAddress::ONE,
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
        let mut lock = self.pending_blocks.lock();
        let last_block_id = self.last_block().id();
        // if the block is a child of the last block we have, we can insert it.
        if last_block_id == blocks.first().unwrap().parent_id() {
            lock.insert(blocks.last().unwrap().round(), (ordered_block, None));
            if self.sync_handle.is_none() {
                self.execution_client
                    .finalize_order(&blocks, ordered_proof, self.commit_callback())
                    .await
                    .unwrap();
            }
        }
    }

    fn process_commit_decision(&mut self, decision: CommitDecision) {
        let mut pending_blocks = self.pending_blocks.lock();
        let decision_round = decision.round();
        if let Some((_, maybe_decision)) = pending_blocks.get_mut(&decision_round) {
            *maybe_decision = Some(decision.clone());
            if self.sync_handle.is_none() {
                self.forward_decision(decision);
            }
        } else {
            if decision_round > self.last_block().round() {
                // enter sync mode if we are missing blocks
                *self.root.lock() = decision.ledger_info().clone();
                self.pending_blocks.lock().clear();
                let round = decision.ledger_info().commit_info().round();
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
                        notify_tx.send(round).await.unwrap();
                    },
                    abort_registration,
                ));
                self.sync_handle = Some(DropGuard::new(abort_handle));
            }
        }
    }

    async fn process_sync_notify(&mut self, round: Round) {
        if self.root.lock().commit_info().round() != round {
            return;
        }
        self.sync_handle = None;
        for (_, (ordered_block, maybe_decision)) in self.pending_blocks.lock().iter() {
            let OrderedBlock {
                blocks,
                ordered_proof,
            } = ordered_block;
            self.execution_client
                .finalize_order(blocks, ordered_proof.clone(), self.commit_callback())
                .await
                .unwrap();
            if let Some(decision) = maybe_decision {
                self.forward_decision(decision.clone());
            }
        }
    }

    pub async fn start(
        mut self,
        mut message_rx: aptos_channel::Receiver<AccountAddress, ConsensusMsg>,
        mut notifier_rx: tokio::sync::mpsc::Receiver<Round>,
        mut close_rx: oneshot::Receiver<oneshot::Sender<()>>,
    ) {
        info!("Consensus Observer started.");
        loop {
            tokio::select! {
                Some(consensus_msg) = message_rx.next() => {
                    let ConsensusMsg::ObserverMessage(msg) = consensus_msg else {
                        continue;
                    };
                    // todo: verify messages
                    self.publisher.publish(msg.clone());
                    match *msg {
                        ObserverMessage::OrderedBlock(ordered_block) => {
                            self.process_ordered_block(ordered_block).await;
                        }
                        ObserverMessage::CommitDecision(msg) => {
                            self.process_commit_decision(msg);
                        }
                    }
                },
                Some(round) = notifier_rx.recv() => {
                    self.process_sync_notify(round).await;
                },
                maybe_tx = &mut close_rx => {
                    if let Ok(tx) = maybe_tx {
                        tx.send(()).unwrap();
                    }
                    break;
                }
            }
        }
        info!("Consensus Observer shuts down.");
    }
}
