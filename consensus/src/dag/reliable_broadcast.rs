// Copyright (c) Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{
    dag::{
        state_machine::{Actions, Command, OutgoingMessage, StateMachine, StateMachineEvent},
        timer::TickingTimer,
        types::{AckSet, IncrementalNodeCertificateState},
    },
    network::NetworkSender,
    network_interface::ConsensusMsg,
    round_manager::VerifiedEvent,
};
use aptos_channels::aptos_channel;
use aptos_consensus_types::{
    common::{Author, Round},
    node::{CertifiedNode, CertifiedNodeAck, Node, SignedNodeDigest},
};
use aptos_logger::info;
use aptos_types::{
    validator_signer::ValidatorSigner, validator_verifier::ValidatorVerifier, PeerId,
};
use async_trait::async_trait;
use futures::{FutureExt, StreamExt};
use futures_channel::oneshot;
use std::{collections::BTreeMap, mem, sync::Arc, time::Duration};
use tokio::{sync::mpsc::Receiver, time};

#[allow(dead_code)]
#[derive(Debug)]
pub(crate) enum ReliableBroadcastCommand {
    BroadcastRequest(Node),
}

enum Status {
    NothingToSend,
    SendingNode(Node, IncrementalNodeCertificateState),
    SendingCertificate(CertifiedNode, AckSet),
}

// TODO: should we use the same message for node and certifade node -> create two verified events.

pub struct ReliableBroadcast {
    my_id: PeerId,
    epoch: u64,
    status: Status,
    peer_round_signatures: BTreeMap<(Round, PeerId), SignedNodeDigest>,
    // vs BTreeMap<Round, BTreeMap<PeerId, ConsensusMsg>> vs Hashset?
    // network_sender: NetworkSender,
    validator_verifier: ValidatorVerifier,
    validator_signer: Arc<ValidatorSigner>,

    messages: Vec<OutgoingMessage>,
    broadcast_timer: TickingTimer,
}

impl ReliableBroadcast {
    pub fn new(
        my_id: PeerId,
        epoch: u64,
        validator_verifier: ValidatorVerifier,
        validator_signer: Arc<ValidatorSigner>,
    ) -> Self {
        Self {
            my_id,
            epoch,
            status: Status::NothingToSend, // TODO status should be persisted.
            // TODO: we need to persist the map and rebuild after crash
            // TODO: Do we need to clean memory inside an epoc? We need to DB between epochs.
            peer_round_signatures: BTreeMap::new(),
            // network_sender,
            validator_verifier,
            validator_signer,

            messages: Vec::new(),
            broadcast_timer: TickingTimer::new(100),
        }
    }

    async fn handle_broadcast_request(&mut self, node: Node) {
        // It is live to stop broadcasting the previous node at this point.
        self.status = Status::SendingNode(
            node.clone(),
            IncrementalNodeCertificateState::new(node.digest()),
        ); // TODO: should we persist?
        self.send_node(node, None)
    }

    // TODO: verify earlier that digest matches the node and epoch is right.
    // TODO: verify node has n-f parents(?).
    async fn handle_node_message(&mut self, node: Node) {
        match self
            .peer_round_signatures
            .get(&(node.round(), node.source()))
        {
            Some(signed_node_digest) => {
                self.send_signed_node_digest(signed_node_digest.clone(), vec![node.source()])
            },
            None => {
                let signed_node_digest =
                    SignedNodeDigest::new(self.epoch, node.digest(), self.validator_signer.clone())
                        .unwrap();
                self.peer_round_signatures
                    .insert((node.round(), node.source()), signed_node_digest.clone());
                // TODO: persist
                self.send_signed_node_digest(signed_node_digest, vec![node.source()]);
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
    fn handle_tick(&mut self) {
        match &self.status {
            // Status::NothingToSend => info!("DAG: reliable broadcast has nothing to resend on tick peer_id {},", self.my_id),
            Status::NothingToSend => info!("DAG: reliable broadcast has nothing to resend on tick"),
            Status::SendingNode(node, incremental_node_certificate_state) => {
                self.send_node(
                    node.clone(),
                    Some(
                        incremental_node_certificate_state
                            .missing_peers_signatures(&self.validator_verifier),
                    ),
                );
            },
            Status::SendingCertificate(certified_node, ack_set) => {
                self.send_certified_node(
                    certified_node.clone(),
                    Some(ack_set.missing_peers(&self.validator_verifier)),
                    true,
                );
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
        close_rx: oneshot::Receiver<oneshot::Sender<()>>,
    ) {
        // TODO: think about tick readability and races.
        let mut interval = time::interval(Duration::from_millis(500)); // TODO: time out should be slightly more than one network round trip.
        let mut close_rx = close_rx.into_stream();

        loop {
            tokio::select! {
                biased;

                _ = interval.tick() => {
                    self.handle_tick();
                },

                Some(command) = command_rx.recv() => {
                    self.process_command(command).await;
                },

                Some(msg) = network_msg_rx.next() => {
                    self.process_message(msg).await;
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

    async fn process_message(&mut self, msg: VerifiedEvent) {
        info!("RB: process_message {:?}", msg);
        match msg {
            VerifiedEvent::NodeMsg(node) => self.handle_node_message(*node).await,

            VerifiedEvent::SignedNodeDigestMsg(signed_node_digest) => {
                if let Some(certified_node) = self.handle_signed_digest(*signed_node_digest) {
                    self.send_certified_node(certified_node, None, true);
                    self.broadcast_timer.reset();
                }
            },

            VerifiedEvent::CertifiedNodeAckMsg(ack) => {
                self.handle_certified_node_ack_msg(*ack);
            },

            _ => unreachable!("reliable broadcast got wrong messsgae"),
        }
    }

    async fn process_command(&mut self, command: ReliableBroadcastCommand) {
        info!("RB: process_command {:?}", command);
        match command {
            ReliableBroadcastCommand::BroadcastRequest(node) => {
                self.handle_broadcast_request(node).await;
                self.broadcast_timer.reset();
            },
        }
    }

    fn send_node(&mut self, node: Node, maybe_recipients: Option<Vec<Author>>) {
        self.messages.push(OutgoingMessage {
            message: ConsensusMsg::NodeMsg(Box::new(node)),
            maybe_recipients,
        });
    }

    fn send_signed_node_digest(
        &mut self,
        signed_node_digest: SignedNodeDigest,
        recipients: Vec<Author>,
    ) {
        self.messages.push(OutgoingMessage {
            message: ConsensusMsg::SignedNodeDigestMsg(Box::new(signed_node_digest)),
            maybe_recipients: Some(recipients),
        });
    }

    fn send_certified_node(
        &mut self,
        node: CertifiedNode,
        maybe_recipients: Option<Vec<Author>>,
        expects_ack: bool,
    ) {
        self.messages.push(OutgoingMessage {
            message: ConsensusMsg::CertifiedNodeMsg(Box::new(node), expects_ack),
            maybe_recipients,
        });
    }
}

#[async_trait]
impl StateMachine for ReliableBroadcast {
    async fn tick(&mut self) {
        if self.broadcast_timer.tick() {
            self.handle_tick();
            self.broadcast_timer.reset();
        }
    }

    async fn step(&mut self, input: StateMachineEvent) -> anyhow::Result<()> {
        match input {
            StateMachineEvent::VerifiedEvent(event) => {
                self.process_message(event).await;
            },
            StateMachineEvent::Command(command) => {
                if let Command::ReliableBroadcastCommand(command) = command {
                    self.process_command(command).await;
                } else {
                    unreachable!("reliable broadcast got wrong command");
                }
            },
        }
        Ok(())
    }

    async fn has_ready(&self) -> bool {
        !self.messages.is_empty()
    }

    async fn ready(&mut self) -> Option<Actions> {
        if !self.has_ready().await {
            return None;
        }

        info!("preparing ready {:?}", self.messages);

        Some(Actions {
            messages: mem::take(&mut self.messages),
            command: None,
            generate_proposal: None,
            ordered_blocks: None,
            state_sync: None,
        })
    }
}
