// Copyright © Aptos Foundation

// Copyright (c) Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

pub mod storage;

use crate::{
    dag::types::{AckSet, IncrementalNodeCertificateState},
    network::{DagSender, NetworkSender},
    round_manager::VerifiedEvent,
};
use aptos_channels::aptos_channel;
use aptos_consensus_types::{
    common::Round,
    node::{CertifiedNode, CertifiedNodeAck, Node, SignedNodeDigest},
};
use aptos_logger::info;
use aptos_types::{
    PeerId, validator_signer::ValidatorSigner, validator_verifier::ValidatorVerifier,
};
use futures::{FutureExt, StreamExt};
use futures_channel::oneshot;
use std::{collections::BTreeMap, sync::Arc, time::Duration};
use std::path::Path;
use tokio::{sync::mpsc::Receiver, time};
use storage::ReliableBroadcastStorage;
use serde::{Serialize, Deserialize};

#[allow(dead_code)]
#[derive(Debug)]
pub(crate) enum ReliableBroadcastCommand {
    BroadcastRequest(Node),
}

#[derive(Debug, PartialEq, Serialize, Deserialize)]
enum Status {
    NothingToSend,
    SendingNode(Node, IncrementalNodeCertificateState),
    SendingCertificate(CertifiedNode, AckSet),
}

/// The in-mem copy of a ReliableBroadcast state.
#[derive(Debug, PartialEq, Serialize, Deserialize)]
pub struct ReliableBroadcastInMem {
    my_id: PeerId,
    epoch: u64,
    status: Status,
    peer_round_signatures: BTreeMap<(Round, PeerId), SignedNodeDigest>,
    // vs BTreeMap<Round, BTreeMap<PeerId, ConsensusMsg>> vs Hashset?
}

// TODO: should we use the same message for node and certifade node -> create two verified events.

pub struct ReliableBroadcast {
    in_mem: ReliableBroadcastInMem,
    storage: Arc<dyn ReliableBroadcastStorage>,
    network_sender: NetworkSender,
    validator_verifier: ValidatorVerifier,
    validator_signer: Arc<ValidatorSigner>,
}

impl ReliableBroadcast {
    pub fn new(
        my_id: PeerId,
        epoch: u64,
        storage: Arc<dyn ReliableBroadcastStorage>,
        network_sender: NetworkSender,
        validator_verifier: ValidatorVerifier,
        validator_signer: Arc<ValidatorSigner>,
    ) -> Self {
        //TODO: is this a good time to clean up?
        let in_mem = if let Some(in_mem) = storage.load_all(my_id, epoch) {
            in_mem
        } else {
            let in_mem = ReliableBroadcastInMem {
                my_id,
                epoch,
                status: Status::NothingToSend,
                peer_round_signatures: BTreeMap::new(),
            };
            storage.save_all(my_id, epoch, &in_mem);
            in_mem
        };
        Self {
            in_mem,
            storage,
            // TODO: we need to persist the map and rebuild after crash
            // TODO: Do we need to clean memory inside an epoc? We need to DB between epochs.
            network_sender,
            validator_verifier,
            validator_signer,
        }
    }

    fn persist_state(&mut self) {
        self.storage.save_all(self.in_mem.my_id, self.in_mem.epoch, &self.in_mem);
    }

    async fn handle_broadcast_request(&mut self, node: Node) {
        // It is live to stop broadcasting the previous node at this point.
        self.in_mem.status = Status::SendingNode(
            node.clone(),
            IncrementalNodeCertificateState::new(node.digest()),
        );
        self.persist_state();
        self.network_sender.send_node(node, None).await
    }

    // TODO: verify earlier that digest matches the node and epoch is right.
    // TODO: verify node has n-f parents(?).
    async fn handle_node_message(&mut self, node: Node) {
        match self
            .in_mem.peer_round_signatures
            .get(&(node.round(), node.source()))
        {
            Some(signed_node_digest) => {
                self.network_sender
                    .send_signed_node_digest(signed_node_digest.clone(), vec![node.source()])
                    .await
            },
            None => {
                let signed_node_digest =
                    SignedNodeDigest::new(self.in_mem.epoch, node.digest(), self.validator_signer.clone())
                        .unwrap();
                self.in_mem.peer_round_signatures
                    .insert((node.round(), node.source()), signed_node_digest.clone());
                self.persist_state(); //TODO: only write the diff.
                self.network_sender
                    .send_signed_node_digest(signed_node_digest, vec![node.source()])
                    .await;
            },
        }
    }

    fn handle_signed_digest(
        &mut self,
        signed_node_digest: SignedNodeDigest,
    ) -> Option<CertifiedNode> {
        let mut certificate_done = false;

        if let Status::SendingNode(_, incremental_node_certificate_state) = &mut self.in_mem.status {
            if let Err(e) = incremental_node_certificate_state.add_signature(signed_node_digest) {
                info!("DAG: could not add signature, err = {:?}", e);
            } else {
                if incremental_node_certificate_state.ready(&self.validator_verifier) {
                    certificate_done = true;
                }
                self.persist_state();//TODO: only write the diff.
            }
        }

        if certificate_done {
            match std::mem::replace(&mut self.in_mem.status, Status::NothingToSend) {
                Status::SendingNode(node, incremental_node_certificate_state) => {
                    let node_certificate =
                        incremental_node_certificate_state.take(&self.validator_verifier);
                    let certified_node = CertifiedNode::new(node, node_certificate);
                    let ack_set = AckSet::new(certified_node.node().digest());

                    self.in_mem.status = Status::SendingCertificate(certified_node.clone(), ack_set);
                    self.persist_state(); //TODO: only write the diff.
                    Some(certified_node)
                },
                _ => unreachable!("dag: status has to be SendingNode"),
            }
        } else {
            None
        }
    }

    // TODO: consider marge node and certified node and use a trait to resend message.
    async fn handle_tick(&mut self) {
        match &self.in_mem.status {
            // Status::NothingToSend => info!("DAG: reliable broadcast has nothing to resend on tick peer_id {},", self.my_id),
            Status::NothingToSend => info!("DAG: reliable broadcast has nothing to resend on tick"),
            Status::SendingNode(node, incremental_node_certificate_state) => {
                self.network_sender
                    .send_node(
                        node.clone(),
                        Some(
                            incremental_node_certificate_state
                                .missing_peers_signatures(&self.validator_verifier),
                        ),
                    )
                    .await;
            },
            Status::SendingCertificate(certified_node, ack_set) => {
                self.network_sender
                    .send_certified_node(
                        certified_node.clone(),
                        Some(ack_set.missing_peers(&self.validator_verifier)),
                        true,
                    )
                    .await;
            },
        };
    }

    fn handle_certified_node_ack_msg(&mut self, ack: CertifiedNodeAck) {
        match &mut self.in_mem.status {
            Status::SendingCertificate(certified_node, ack_set) => {
                // TODO: check ack is up to date!
                if ack.digest() == certified_node.digest() {
                    ack_set.add(ack);
                    if ack_set.missing_peers(&self.validator_verifier).is_empty() {
                        self.in_mem.status = Status::NothingToSend;
                    }
                    self.persist_state(); //TODO: only write the diff.
                }
            },
            _ => {},
        }
    }

    pub(crate) async fn start(
        mut self,
        mut network_msg_rx: aptos_channel::Receiver<PeerId, VerifiedEvent>,
        mut command_rx: Receiver<ReliableBroadcastCommand>,
        close_rx: oneshot::Receiver<oneshot::Sender<()>>,
    ) {
        // TODO: think about tick readability and races.
        let mut interval = time::interval(Duration::from_millis(500)); // TODO: time out should be slightly more than one network round trip.
        let mut close_rx = close_rx.into_stream();

        loop {
            tokio::select! {
                biased;

                _ = interval.tick() => {
                    self.handle_tick().await;
                },

                Some(command) = command_rx.recv() => {
                    match command {
                        ReliableBroadcastCommand::BroadcastRequest(node) => {
                            self.handle_broadcast_request(node).await;
                            interval.reset();
                        }
                    }
                },

                Some(msg) = network_msg_rx.next() => {
                    match msg {
                        VerifiedEvent::NodeMsg(node) => {
                            self.handle_node_message(*node).await
                        },

                        VerifiedEvent::SignedNodeDigestMsg(signed_node_digest) => {
                            if let Some(certified_node) = self.handle_signed_digest(*signed_node_digest) {
                                self.network_sender.send_certified_node(certified_node, None, true).await;
                                interval.reset();
                            }

                        },


                        VerifiedEvent::CertifiedNodeAckMsg(ack) => {
                            self.handle_certified_node_ack_msg(*ack);
                        },

                        _ => unreachable!("reliable broadcast got wrong messsgae"),
                    }

                },

                close_req = close_rx.select_next_some() => {
                    if let Ok(ack_sender) = close_req {
                        ack_sender.send(()).expect("[ReliableBroadcast] Fail to ack shutdown");
                    }
                    break;
                }

            }
        }
    }
}
