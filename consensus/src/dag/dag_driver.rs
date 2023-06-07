// Copyright (c) Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use super::state_machine::{Actions, Command, OutgoingMessage, StateMachine, StateMachineEvent};
use crate::{
    dag::{
        anchor_election::RoundRobinAnchorElection,
        bullshark::Bullshark,
        dag::Dag,
        dag_driver::Mode::{Normal, StateSync},
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
use aptos_logger::{debug, error, spawn_named, prelude::{sample, SampleRate}};
use aptos_types::{
    ledger_info::LedgerInfoWithSignatures, validator_signer::ValidatorSigner,
    validator_verifier::ValidatorVerifier, PeerId,
};
use async_trait::async_trait;
use claims::assert_some;
use futures::{FutureExt, StreamExt};
use futures_channel::oneshot;
use serde::{Deserialize, Serialize};
use std::{
    collections::HashSet,
    mem,
    ops::Deref,
    path::{Path, PathBuf},
    sync::Arc,
    time::Duration,
};
use tokio::{
    fs::File,
    io::{AsyncReadExt, AsyncWriteExt},
    sync::{mpsc::Sender, Mutex},
    time,
};

const ROOT_NODE_FILE: &str = "root_node";

#[derive(PartialEq)]
pub enum Mode {
    Normal,
    StateSync(LedgerInfoWithSignatures),
}

pub struct DagDriver {
    epoch: u64,
    round: Round,
    author: Author,
    config: DagConfig,
    // payload_client: Arc<dyn PayloadClient>,
    // timeout: bool,
    // network_sender: NetworkSender,
    // TODO: Should we clean more often than once an epoch?
    dag: Dag,
    // bullshark: Bullshark,
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

    mode: Mode,
    state_sync_in_progress: bool,

    rb_command: Option<ReliableBroadcastCommand>,

    interval_timer: TickingTimer,
    remote_fetch_timer: TickingTimer,

    highest_commit_info: LedgerInfoWithSignatures,

    validator_verifier: ValidatorVerifier,

    root_node_path: PathBuf,
}

impl DagDriver {
    pub async fn new(
        epoch: u64,
        author: Author,
        config: DagConfig,
        // payload_client: Arc<dyn PayloadClient>,
        // network_sender: NetworkSender,
        verifier: ValidatorVerifier,
        validator_signer: Arc<ValidatorSigner>,
        payload_manager: Arc<PayloadManager>,
        // state_computer: Arc<dyn StateComputer>,
        time_service: Arc<dyn TimeService>,
        genesis_block_id: HashValue,
        root_commit_ledger_info: LedgerInfoWithSignatures,
        root_node_path: PathBuf,
    ) -> Self {
        let proposer_election = Arc::new(RoundRobinAnchorElection::new(&verifier));
        let bullshark = Bullshark::new(
            epoch,
            author,
            // state_computer,
            proposer_election.clone(),
            verifier.clone(),
            genesis_block_id,
        );

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
            // timeout: false,
            // network_sender,
            dag: Dag::new(
                author,
                epoch,
                bullshark,
                verifier.clone(),
                proposer_election,
                payload_manager,
            ),
            // bullshark,
            // rb_tx,
            // rb_close_tx,
            // network_msg_rx,
            time_service,
            timers: vec![],
            messages: vec![],

            awaiting_proposal: false,
            next_round_payload_filter: None,
            next_round_parents: None,

            interval_timer: TickingTimer::new(5),
            remote_fetch_timer: TickingTimer::new(1),
            rb_command: None,
            highest_commit_info: root_commit_ledger_info,

            validator_verifier: verifier,

            mode: Mode::Normal,
            state_sync_in_progress: false,
            root_node_path,
        };
        dag_driver.init().await;

        dag_driver
    }

    async fn remote_fetch_missing_nodes(&mut self) {
        sample!(
            SampleRate::Duration(Duration::from_secs(1)),
            debug!("trying remote_fetch_missing_nodes");
        );
        for (node_meta_data, nodes_to_request) in self.dag.missing_nodes_metadata() {
            debug!(
                "requesting missing node: {:?} from {:?}",
                node_meta_data, nodes_to_request
            );
            let request = CertifiedNodeRequest::new(node_meta_data, self.author);
            self.send_certified_node_request(request, nodes_to_request)
                .await;
        }
    }

    async fn handle_node_request(&mut self, node_request: CertifiedNodeRequest) {
        if let Some(certified_node) = self.dag.get_node(&node_request) {
            debug!("received node request: {:?}", node_request);
            self.send_certified_node(
                certified_node.clone(),
                Some(vec![node_request.requester()]),
                false,
            )
            .await
        } else {
            debug!(
                "received node request for node not in dag: {:?}",
                node_request
            );
        }
    }

    fn create_node(&mut self, parents: HashSet<NodeMetaData>, payload: Payload) -> Node {
        let timestamp = self.time_service.get_current_timestamp().as_micros() as u64;

        Node::new(
            self.epoch,
            self.round,
            self.author,
            payload,
            parents,
            timestamp,
            self.highest_commit_info.clone(),
        )
    }

    async fn try_advance_round(&mut self, timeout: bool) -> bool {
        if let Some(parents) = self.dag.try_advance_round(timeout) {
            self.round += 1;
            self.awaiting_proposal = true;
            self.next_round_parents = Some(parents);
            self.next_round_payload_filter = Some(self.dag.bullshark.pending_payload());
            true
        } else {
            false
        }
    }

    async fn process_highest_commit_ledger_info(&mut self, commit_info: LedgerInfoWithSignatures) {
        // FIXME(ibalajiarun) handle unwraps
        // TODO(ibalajiarun) move signature verification to bounded_executor in EpochManager
        if self.highest_commit_info.commit_info().round() >= commit_info.commit_info().round() {
            return;
        }

        commit_info
            .verify_signatures(&self.validator_verifier)
            .unwrap();

        if self.mode == Normal
            && commit_info.commit_info().round()
                > self.highest_commit_info.commit_info().round() + self.config.state_sync_window
        {
            // change mode
            self.mode = StateSync(commit_info.clone());
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

    async fn process_message(&mut self, msg: VerifiedEvent) {
        match msg {
            VerifiedEvent::CommitDecision(commit_decision) => {
                self.process_highest_commit_ledger_info(commit_decision.ledger_info().clone())
                    .await;
            },
            VerifiedEvent::CertifiedNodeRequestMsg(node_request) => {
                self.handle_node_request(*node_request).await
            },
            VerifiedEvent::CertifiedNodeMsg(certified_node, ack_required) => {
                self.handle_certified_node(*certified_node, ack_required)
                    .await;
                self.try_advance_round(false).await;
            },
            _ => {
                error!("DAG: unexpected message type: {:?}", msg);
            },
        }
    }

    async fn advance_round(&mut self, payload: Payload) {
        let node = if self.round == 0 {
            let epoch_root_node = format!("{}_{}", ROOT_NODE_FILE, self.epoch);
            // Decide if we need to create a new node or read from file.
            let path = self.root_node_path.join(epoch_root_node);
            // read the path file and create a node
            let node = if path.exists() {
                let mut file = File::open(path).await.unwrap();
                let mut buffer = Vec::new();
                file.read_to_end(&mut buffer).await.unwrap();
                bcs::from_bytes(buffer.as_slice()).unwrap()
            } else {
                let node = self.create_node(HashSet::new(), payload);
                // write the node to file
                let mut file = File::create(path).await.unwrap();
                let serialized = bcs::to_bytes(&node).unwrap();
                file.write_all(serialized.as_slice()).await.unwrap();
                node
            };
            node
        } else {
            // FIXME(ibalajiarun): Fix the unwrap
            let parents = self.next_round_parents.take().unwrap();
            self.create_node(parents, payload)
        };

        self.rb_command = Some(ReliableBroadcastCommand::BroadcastRequest(node));
        // self.timeout = false;
        self.interval_timer.reset();
    }

    async fn process_command(&mut self, cmd: Command) {
        match cmd {
            Command::DagNodeProposal(payload) => {
                debug!("DAG: proposing {}", payload);
                assert!(self.awaiting_proposal);
                self.awaiting_proposal = false;
                self.advance_round(payload).await
            },
            Command::DagStateSyncNotification => {
                self.state_sync_in_progress = false;
                self.mode = Normal
            },
            _ => unreachable!(),
        }
    }

    pub async fn init(&mut self) {
        if self.round == 0 {
            self.advance_round(Payload::empty(true)).await;
            return;
        }
        self.awaiting_proposal = true;
        self.next_round_parents = Some(HashSet::new());
        self.next_round_payload_filter = Some(PayloadFilter::Empty);
    }

    async fn on_remote_fetch_timer(&mut self) {
        self.remote_fetch_missing_nodes().await;
        self.remote_fetch_timer.reset();
    }

    async fn on_interval_timer(&mut self) {
        if self.try_advance_round(true).await {
            // TODO(ibalajiarun): stop the timer here
            self.interval_timer.stop();
        }
        // if self.timeout == false {
        //     self.timeout = true;
        // }
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
            maybe_recipients: None,
        });
    }

    pub async fn notify_commit(&mut self, commit_info: LedgerInfoWithSignatures) {
        self.highest_commit_info = commit_info;
    }
}

#[async_trait]
impl StateMachine for DagDriver {
    async fn tick(&mut self) {
        if self.interval_timer.tick() {
            // debug!("interval_timer ticking...");
            self.on_interval_timer().await;
            self.interval_timer.reset();
        }
        if self.remote_fetch_timer.tick() {
            // debug!("remote_fetch_timer ticking...");
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

    async fn has_ready(&self) -> bool {
        (self.awaiting_proposal && self.next_round_payload_filter.is_some())
            || !self.messages.is_empty()
            || self.rb_command.is_some()
            || { !self.dag.bullshark.ordered_blocks().is_empty() }
            || (!self.state_sync_in_progress && self.mode != Normal)
    }

    async fn ready(&mut self) -> Option<Actions> {
        if !self.has_ready().await {
            return None;
        }
        let mut actions = Actions::default();

        if self.awaiting_proposal && self.next_round_payload_filter.is_some() {
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

        {
            let bs = &mut self.dag.bullshark;
            if !bs.ordered_blocks().is_empty() {
                actions.ordered_blocks = Some(bs.take_ordered_blocks());
            }
        }

        if let StateSync(ref sync) = self.mode {
            if !self.state_sync_in_progress {
                self.state_sync_in_progress = true;
                actions.state_sync = Some(sync.clone());
            }
        }

        Some(actions)
    }
}
