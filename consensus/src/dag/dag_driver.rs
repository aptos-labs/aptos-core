// Copyright (c) Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::dag::bullshark::Bullshark;
use crate::dag::dag::Dag;
use crate::dag::reliable_broadcast::{ReliableBroadcast, ReliableBroadcastCommand};
use crate::state_replication::PayloadClient;
use crate::{
    network::{DagSender, NetworkSender},
    round_manager::VerifiedEvent,
};
use aptos_channels::aptos_channel;
use aptos_config::config::DagConfig;
use aptos_consensus_types::common::{Author, PayloadFilter, Round};
use aptos_consensus_types::node::{
    CertifiedNode, CertifiedNodeAck, CertifiedNodeRequest, Node, NodeMetaData,
};
use aptos_logger::spawn_named;
use aptos_types::validator_signer::ValidatorSigner;
use aptos_types::validator_verifier::ValidatorVerifier;
use aptos_types::PeerId;
use futures::StreamExt;
use std::collections::HashSet;
use std::sync::Arc;
use std::time::Duration;
use tokio::{sync::mpsc::Sender, time};

#[allow(dead_code)]
pub struct DagDriver {
    epoch: u64,
    round: Round,
    author: Author,
    config: DagConfig,
    payload_client: Arc<dyn PayloadClient>,
    timeout: bool,
    network_sender: NetworkSender,
    // TODO: Should we clean more often than once an epoch?
    dag: Dag,
    rb_tx: Sender<ReliableBroadcastCommand>,
}

#[allow(dead_code)]
impl DagDriver {
    pub fn new(
        epoch: u64,
        author: Author,
        config: DagConfig,
        payload_client: Arc<dyn PayloadClient>,
        network_sender: NetworkSender,
        verifier: ValidatorVerifier,
        validator_signer: Arc<ValidatorSigner>,
        rb_network_msg_rx: aptos_channel::Receiver<PeerId, VerifiedEvent>,
        _self_network_msg_rx: aptos_channel::Receiver<PeerId, VerifiedEvent>,
    ) -> Self {
        // TODO: should basically replace round manager. Spawns Bullshark and RB and pass channels around

        let (dag_bullshark_tx, dag_bullshark_rx) = tokio::sync::mpsc::channel(config.channel_size);
        let (rb_tx, rb_rx) = tokio::sync::mpsc::channel(config.channel_size);

        // TODO: Start dummy Bullshark. Then spawn from epoch_manager.rs

        let rb = ReliableBroadcast::new(
            author,
            network_sender.clone(),
            verifier.clone(),
            validator_signer,
        );

        let bullshark = Bullshark::new();

        spawn_named!("reliable_broadcast", rb.start(rb_network_msg_rx, rb_rx));
        spawn_named!("bullshark", bullshark.start(dag_bullshark_rx));

        Self {
            epoch,
            round: 0,
            author,
            config,
            payload_client,
            timeout: false,
            network_sender,
            dag: Dag::new(epoch, dag_bullshark_tx, verifier.clone()),
            rb_tx,
        }
    }

    async fn remote_fetch_missing_nodes(&self) {
        for (node_meta_data, nodes_to_request) in self.dag.missing_nodes_metadata() {
            let request = CertifiedNodeRequest::new(node_meta_data, self.author);
            self.network_sender
                .send_certified_node_request(request, nodes_to_request)
                .await;
        }
    }

    async fn handle_node_request(&mut self, node_request: CertifiedNodeRequest) {
        if let Some(certified_node) = self.dag.get_node(&node_request) {
            self.network_sender
                .send_certified_node(
                    certified_node.clone(),
                    Some(vec![node_request.requester()]),
                    false,
                )
                .await
        }
    }

    async fn create_node(&mut self, parents: HashSet<NodeMetaData>) -> Node {
        let excluded_payload = Vec::new(); // TODO
        let payload_filter = PayloadFilter::from(&excluded_payload);
        let payload = self
            .payload_client
            .pull_payload_for_dag(
                self.round,
                self.config.max_node_txns,
                self.config.max_node_bytes,
                payload_filter,
            )
            .await
            .expect("DAG: fail to retrieve payload");
        Node::new(self.epoch, self.round, self.author, payload, parents)
    }

    async fn try_advance_round(&mut self) -> Option<Node> {
        if let Some(parents) = self.dag.try_advance_round(self.timeout) {
            self.round += 1;
            Some(self.create_node(parents).await)
        } else {
            None
        }
    }

    async fn handle_certified_node(&mut self, certified_node: CertifiedNode, ack_required: bool) {
        let digest = certified_node.digest();
        let source = certified_node.source();
        self.dag.try_add_node(certified_node).await;

        if ack_required {
            let ack = CertifiedNodeAck::new(digest, self.author);
            self.network_sender
                .send_certified_node_ack(ack, vec![source])
                .await
        }
    }

    #[allow(dead_code)]
    pub(crate) async fn start(
        &mut self,
        mut network_msg_rx: aptos_channel::Receiver<PeerId, VerifiedEvent>,
    ) {
        let node = self.create_node(HashSet::new()).await;
        self.rb_tx
            .send(ReliableBroadcastCommand::BroadcastRequest(node))
            .await
            .expect("dag: reliable broadcast receiver dropped");

        let mut interval_missing_nodes = time::interval(Duration::from_millis(500)); // time out should be slightly more than one network round trip.
        let mut interval_timeout = time::interval(Duration::from_millis(1000)); // similar to leader timeout in our consensus
        loop {
            // TODO: shutdown
            tokio::select! {
                biased;

                _ = interval_missing_nodes.tick() => {
                self.remote_fetch_missing_nodes().await
            },

                _ = interval_timeout.tick() => {
                    if self.timeout == false {
                        self.timeout = true;
                        if let Some(node) = self.try_advance_round().await {
                            self.rb_tx.send(ReliableBroadcastCommand::BroadcastRequest(node)).await.expect("dag: reliable broadcast receiver dropped");
                            self.timeout = false;
                            interval_timeout.reset();
                        }

                    }
                }

            Some(msg) = network_msg_rx.next() => {
                    match msg {

                        VerifiedEvent::CertifiedNodeMsg(certified_node, ack_required) => {

                            self.handle_certified_node(*certified_node, ack_required).await;
                            if let Some(node) = self.try_advance_round().await {
                                self.rb_tx.send(ReliableBroadcastCommand::BroadcastRequest(node)).await.expect("dag: reliable broadcast receiver dropped");
                                self.timeout = false;
                                interval_timeout.reset();
                            }

                        },

                        VerifiedEvent::CertifiedNodeRequestMsg(node_request) => {
                            self.handle_node_request(*node_request).await;
                    }
                    _ => unreachable!("reliable broadcast got wrong messsgae"),
                    }
                },
            }
        }
    }
}