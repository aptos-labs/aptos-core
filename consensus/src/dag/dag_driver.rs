// Copyright (c) Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use super::state_machine::{Actions, Command, OutgoingMessage, StateMachine, StateMachineEvent};
use crate::{
    dag::{
        anchor_election::RoundRobinAnchorElection,
        bullshark::Bullshark,
        dag::Dag,
        reliable_broadcast::{ReliableBroadcast, ReliableBroadcastCommand},
        timer::TickingTimer,
    },
    network::{DagSender, NetworkSender},
    network_interface::ConsensusMsg,
    payload_manager::PayloadManager,
    round_manager::VerifiedEvent,
    state_replication::{PayloadClient, StateComputer},
    util::time_service::TimeService,
};
use anyhow::Result;
use aptos_channels::aptos_channel::{self, Receiver};
use aptos_config::config::DagConfig;
use aptos_consensus_types::{
    common::{Author, Payload, PayloadFilter, Round},
    node::{CertifiedNode, CertifiedNodeAck, CertifiedNodeRequest, Node, NodeMetaData},
};
use aptos_crypto::HashValue;
use aptos_logger::{error, spawn_named};
use aptos_types::{
    validator_signer::ValidatorSigner, validator_verifier::ValidatorVerifier, PeerId,
};
use async_trait::async_trait;
use claims::assert_some;
use futures::{FutureExt, StreamExt};
use futures_channel::oneshot;
use std::{collections::HashSet, mem, ops::Deref, sync::Arc, time::Duration};
use tokio::{
    sync::{mpsc::Sender, Mutex},
    time,
};

pub struct DagDriver {
    epoch: u64,
    round: Round,
    author: Author,
    config: DagConfig,
    // payload_client: Arc<dyn PayloadClient>,
    timeout: bool,
    // network_sender: NetworkSender,
    // TODO: Should we clean more often than once an epoch?
    dag: Dag,
    bullshark: Arc<Mutex<Bullshark>>,
    // rb_tx: Sender<ReliableBroadcastCommand>,
    // rb_close_tx: oneshot::Sender<oneshot::Sender<()>>,
    // network_msg_rx: Receiver<PeerId, VerifiedEvent>,
    time_service: Arc<dyn TimeService>,

    timers: Vec<TickingTimer>,
    messages: Vec<OutgoingMessage>,

    // TODO(ibalajiarun): Make this better by combining the three.
    awaiting_proposal: bool,
    next_round_payload_filter: Option<PayloadFilter>,
    next_round_parents: Option<HashSet<NodeMetaData>>,

    rb_command: Option<ReliableBroadcastCommand>,

    interval_timer: TickingTimer,
    remote_fetch_timer: TickingTimer,
}

impl DagDriver {
    pub fn new(
        epoch: u64,
        author: Author,
        config: DagConfig,
        // payload_client: Arc<dyn PayloadClient>,
        // network_sender: NetworkSender,
        verifier: ValidatorVerifier,
        validator_signer: Arc<ValidatorSigner>,
        payload_manager: Arc<PayloadManager>,
        state_computer: Arc<dyn StateComputer>,
        time_service: Arc<dyn TimeService>,
        genesis_block_id: HashValue,
    ) -> Self {
        let proposer_election = Arc::new(RoundRobinAnchorElection::new(&verifier));
        let bullshark = Arc::new(Mutex::new(Bullshark::new(
            epoch,
            author,
            state_computer,
            proposer_election.clone(),
            verifier.clone(),
            genesis_block_id,
        )));

        // let (rb_close_tx, close_rx) = oneshot::channel();

        // spawn_named!(
        //     "reliable_broadcast",
        //     rb.start(rb_network_msg_rx, rb_rx, close_rx)
        // );
        // spawn_named!("bullshark", bullshark.start(dag_bullshark_rx));

        let mut dag_driver = Self {
            epoch,
            round: 0,
            author,
            config,
            // payload_client,
            timeout: false,
            // network_sender,
            dag: Dag::new(
                author,
                epoch,
                bullshark.clone(),
                verifier.clone(),
                proposer_election,
                payload_manager,
            ),
            bullshark,
            // rb_tx,
            // rb_close_tx,
            // network_msg_rx,
            time_service,
            timers: vec![],
            messages: vec![],

            awaiting_proposal: false,
            next_round_payload_filter: None,
            next_round_parents: None,

            interval_timer: TickingTimer::new(100),
            remote_fetch_timer: TickingTimer::new(50),
            rb_command: None,
        };
        dag_driver.init();

        dag_driver
    }

    async fn remote_fetch_missing_nodes(&mut self) {
        for (node_meta_data, nodes_to_request) in self.dag.missing_nodes_metadata() {
            let request = CertifiedNodeRequest::new(node_meta_data, self.author);
            self.send_certified_node_request(request, nodes_to_request)
                .await;
        }
    }

    async fn handle_node_request(&mut self, node_request: CertifiedNodeRequest) {
        if let Some(certified_node) = self.dag.get_node(&node_request) {
            self.send_certified_node(
                certified_node.clone(),
                Some(vec![node_request.requester()]),
                false,
            )
            .await
        }
    }

    async fn create_node(&mut self, parents: HashSet<NodeMetaData>, payload: Payload) -> Node {
        let timestamp = self.time_service.get_current_timestamp().as_micros() as u64;

        Node::new(
            self.epoch,
            self.round,
            self.author,
            payload,
            parents,
            timestamp,
        )
    }

    async fn try_advance_round(&mut self) -> bool {
        if let Some(parents) = self.dag.try_advance_round(self.timeout) {
            self.round += 1;
            self.awaiting_proposal = true;
            self.next_round_parents = Some(parents);
            self.next_round_payload_filter = Some(self.bullshark.lock().await.pending_payload());
            true
        } else {
            false
        }
    }

    async fn handle_certified_node(&mut self, certified_node: CertifiedNode, ack_required: bool) {
        let digest = certified_node.digest();
        let source = certified_node.source();
        self.dag.try_add_node(certified_node).await;

        if ack_required {
            let ack = CertifiedNodeAck::new(self.epoch, digest, self.author);
            self.send_certified_node_ack(ack, vec![source]).await
        }
    }

    async fn tick(&mut self) {
        for timer in self.timers.iter_mut() {
            timer.tick();
        }
    }

    async fn process_message(&mut self, msg: VerifiedEvent) {
        match msg {
            VerifiedEvent::CertifiedNodeRequestMsg(node_request) => {
                self.handle_node_request(*node_request).await
            },
            VerifiedEvent::CertifiedNodeMsg(certified_node, ack_required) => {
                self.handle_certified_node(*certified_node, ack_required)
                    .await
            },
            _ => {
                error!("DAG: unexpected message type: {:?}", msg);
            },
        }
    }

    async fn advance_round(&mut self, payload: Payload) {
        // TODO(ibalajiarun): Fix the unwrap
        let parents = self.next_round_parents.take().unwrap();
        let node = self.create_node(parents, payload).await;
        self.rb_command = Some(ReliableBroadcastCommand::BroadcastRequest(node));
        self.timeout = false;
        self.interval_timer.reset();
    }

    async fn process_command(&mut self, cmd: Command) {
        assert!(self.awaiting_proposal);

        self.awaiting_proposal = false;
        if let Command::DagNodeProposal(payload) = cmd {
            self.advance_round(payload).await
        }
    }

    pub fn init(&mut self) {
        self.awaiting_proposal = true;
        self.next_round_parents = Some(HashSet::new());
        self.next_round_payload_filter = Some(PayloadFilter::Empty);
    }

    async fn on_remote_fetch_timer(&mut self) {
        self.remote_fetch_missing_nodes();
        self.remote_fetch_timer.reset();
    }

    async fn on_interval_timer(&mut self) {
        if self.timeout == false {
            self.timeout = true;
            if self.try_advance_round().await {
                // TODO(ibalajiarun): stop the timer here
                self.interval_timer.stop();
            }
        }
    }

    async fn send_certified_node(
        &mut self,
        certified_node: CertifiedNode,
        maybe_recipients: Option<Vec<Author>>,
        ack_required: bool,
    ) {
        self.messages.push(OutgoingMessage {
            message: ConsensusMsg::CertifiedNodeMsg(Box::new(certified_node), ack_required),
            maybe_recipients,
        });
    }

    async fn send_certified_node_ack(&mut self, ack: CertifiedNodeAck, recipients: Vec<Author>) {
        self.messages.push(OutgoingMessage {
            message: ConsensusMsg::CertifiedNodeAckMsg(Box::new(ack)),
            maybe_recipients: Some(recipients),
        });
    }

    async fn send_certified_node_request(
        &mut self,
        req: CertifiedNodeRequest,
        recipients: Vec<Author>,
    ) {
        self.messages.push(OutgoingMessage {
            message: ConsensusMsg::CertifiedNodeRequestMsg(Box::new(req)),
            maybe_recipients: Some(recipients),
        });
    }
}

#[async_trait]
impl StateMachine for DagDriver {
    async fn tick(&mut self) {
        if self.interval_timer.tick() {
            self.on_interval_timer().await;
        }
        if self.remote_fetch_timer.tick() {
            self.on_remote_fetch_timer().await;
        }
    }

    async fn step(&mut self, input: StateMachineEvent) -> Result<()> {
        match input {
            StateMachineEvent::VerifiedEvent(event) => self.process_message(event).await,
            StateMachineEvent::Command(command) => self.process_command(command).await,
        };
        Ok(())
    }

    fn has_ready(&self) -> bool {
        self.awaiting_proposal || !self.messages.is_empty() || self.rb_command.is_some()
    }

    async fn ready(&mut self) -> Option<Actions> {
        if !self.has_ready() {
            return None;
        }
        let mut actions = Actions::default();

        if self.awaiting_proposal {
            actions.generate_proposal = self.next_round_payload_filter.take();
        }

        if !self.messages.is_empty() {
            actions.messages = mem::take(&mut self.messages);
        }

        if self.rb_command.is_some() {
            actions.command = self
                .rb_command
                .take()
                .map(|cmd| Command::ReliableBroadcastCommand(cmd))
        }

        Some(actions)
    }
}
