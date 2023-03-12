// Copyright (c) Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::dag::types::{AckSet, IncrementalNodeCertificateState};
use crate::network::{DagSender, NetworkSender};
use crate::round_manager::VerifiedEvent;
use aptos_channels::aptos_channel;
use aptos_consensus_types::common::Round;
use aptos_consensus_types::node::{CertifiedNode, CertifiedNodeAck, Node, SignedNodeDigest};
use aptos_logger::info;
use aptos_types::validator_signer::ValidatorSigner;
use aptos_types::validator_verifier::ValidatorVerifier;
use aptos_types::PeerId;
use futures::StreamExt;
use std::collections::BTreeMap;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::mpsc::Receiver;
use tokio::time;

#[allow(dead_code)]
#[derive(Debug)]
pub(crate) enum ReliableBroadcastCommand {
    BroadcastRequest(Node),
}

// TODO: traits?
enum Status {
    NothingToSend,
    SendingNode(Node, IncrementalNodeCertificateState),
    SendingCertificate(CertifiedNode, AckSet),
}

// TODO: should we use the same message for node and certifade node -> create two verified events.

#[allow(dead_code)]
pub struct ReliableBroadcast {
    my_id: PeerId,
    status: Status,
    peer_round_signatures: BTreeMap<(Round, PeerId), SignedNodeDigest>,
    // vs BTreeMap<Round, BTreeMap<PeerId, ConsensusMsg>> vs Hashset?
    network_sender: NetworkSender,
    validator_verifier: ValidatorVerifier,
    validator_signer: Arc<ValidatorSigner>,
}

#[allow(dead_code)]
impl ReliableBroadcast {
    pub fn new(
        my_id: PeerId,
        network_sender: NetworkSender,
        validator_verifier: ValidatorVerifier,
        validator_signer: Arc<ValidatorSigner>,
    ) -> Self {
        Self {
            my_id,
            status: Status::NothingToSend, // TODO status should be persisted.
            // TODO: we need to persist the map and rebuild after crash
            // TODO: Do we need to clean memory inside an epoc? We need to DB between epochs.
            peer_round_signatures: BTreeMap::new(),
            network_sender,
            validator_verifier,
            validator_signer,
        }
    }

    async fn handle_broadcast_request(&mut self, node: Node) {
        // It is live to stop broadcasting the previous node at this point.
        self.status = Status::SendingNode(
            node.clone(),
            IncrementalNodeCertificateState::new(node.digest()),
        ); // TODO: should we persist?
        self.network_sender.send_node(node, None).await
    }

    // TODO: verify earlier that digest matches the node and epoch is right.
    // TODO: verify node has n-f parents(?).
    async fn handle_node_message(&mut self, node: Node) {
        match self
            .peer_round_signatures
            .get(&(node.round(), *node.source()))
        {
            Some(signed_node_digest) => {
                self.network_sender
                    .send_signed_node_digest(signed_node_digest.clone(), vec![*node.source()])
                    .await
            },
            None => {
                let signed_node_digest =
                    SignedNodeDigest::new(node.digest(), self.validator_signer.clone()).unwrap();
                self.peer_round_signatures
                    .insert((node.round(), *node.source()), signed_node_digest.clone());
                // TODO: persist
                self.network_sender
                    .send_signed_node_digest(signed_node_digest, vec![*node.source()])
                    .await;
            },
        }
    }

    fn handle_signed_digest(
        &mut self,
        signed_node_digest: SignedNodeDigest,
    ) -> Option<CertifiedNode> {
        let mut certificate_done = false;

        if let Status::SendingNode(_, incremental_node_certificate_state) = &mut self.status {
            if let Err(e) = incremental_node_certificate_state.add_signature(signed_node_digest) {
                info!("DAG: could not add signature, err = {:?}", e);
            } else if incremental_node_certificate_state.ready(&self.validator_verifier) {
                certificate_done = true;
            }
        }

        if certificate_done {
            match std::mem::replace(&mut self.status, Status::NothingToSend) {
                Status::SendingNode(node, incremental_node_certificate_state) => {
                    let node_certificate =
                        incremental_node_certificate_state.take(&self.validator_verifier);
                    let certified_node = CertifiedNode::new(node, node_certificate);
                    let ack_set = AckSet::new(certified_node.node().digest());

                    self.status = Status::SendingCertificate(certified_node.clone(), ack_set); // TODO: should we persist status? probably yes.
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
        match &self.status {
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
        match &mut self.status {
            Status::SendingCertificate(certified_node, ack_set) => {
                // TODO: check ack is up to date!
                if ack.digest() == certified_node.digest() {
                    ack_set.add(ack);
                    if ack_set.missing_peers(&self.validator_verifier).is_empty() {
                        self.status = Status::NothingToSend;
                    }
                }
            },
            _ => {},
        }
    }

    pub(crate) async fn start(
        mut self,
        mut network_msg_rx: aptos_channel::Receiver<PeerId, VerifiedEvent>,
        mut command_rx: Receiver<ReliableBroadcastCommand>,
    ) {
        // TODO: think about tick readability and races.
        let mut interval = time::interval(Duration::from_millis(500)); // TODO: time out should be slightly more than one network round trip.

        loop {
            // TODO: shutdown
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
            }
        }
    }
}