// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use super::RpcHandler;
use crate::{
    dag::{
        dag_network::DAGNetworkSender,
        dag_store::Dag,
        types::{CertifiedNode, DAGMessage, FetchResponse, Node, RemoteFetchRequest},
    },
    network::TConsensusMsg,
};
use anyhow::ensure;
use aptos_consensus_types::common::Author;
use aptos_infallible::RwLock;
use aptos_logger::error;
use aptos_types::epoch_state::EpochState;
use std::{sync::Arc, time::Duration};
use thiserror::Error as ThisError;
use tokio::sync::{
    mpsc::{Receiver, Sender},
    oneshot,
};

pub enum LocalFetchRequest {
    Node(Node, oneshot::Sender<Node>),
    CertifiedNode(CertifiedNode, oneshot::Sender<CertifiedNode>),
}

impl LocalFetchRequest {
    pub fn responders(&self, validators: &[Author]) -> Vec<Author> {
        match self {
            LocalFetchRequest::Node(node, _) => vec![*node.author()],
            LocalFetchRequest::CertifiedNode(node, _) => {
                node.signatures().get_signers_addresses(validators)
            },
        }
    }

    pub fn notify(self) {
        if match self {
            LocalFetchRequest::Node(node, sender) => sender.send(node).map_err(|_| ()),
            LocalFetchRequest::CertifiedNode(node, sender) => sender.send(node).map_err(|_| ()),
        }
        .is_err()
        {
            error!("Failed to send node back");
        }
    }

    pub fn node(&self) -> &Node {
        match self {
            LocalFetchRequest::Node(node, _) => node,
            LocalFetchRequest::CertifiedNode(node, _) => node,
        }
    }
}

struct DagFetcher {
    epoch_state: Arc<EpochState>,
    network: Arc<dyn DAGNetworkSender>,
    dag: Arc<RwLock<Dag>>,
    request_rx: Receiver<LocalFetchRequest>,
}

impl DagFetcher {
    pub fn new(
        epoch_state: Arc<EpochState>,
        network: Arc<dyn DAGNetworkSender>,
        dag: Arc<RwLock<Dag>>,
    ) -> (Self, Sender<LocalFetchRequest>) {
        let (request_tx, request_rx) = tokio::sync::mpsc::channel(16);
        (
            Self {
                epoch_state,
                network,
                dag,
                request_rx,
            },
            request_tx,
        )
    }

    pub async fn start(mut self) {
        while let Some(local_request) = self.request_rx.recv().await {
            let responders = local_request
                .responders(&self.epoch_state.verifier.get_ordered_account_addresses());
            let remote_request = {
                let dag_reader = self.dag.read();
                if dag_reader.all_exists(local_request.node().parents()) {
                    local_request.notify();
                    continue;
                }

                let (lowest_round, bitmask, missing_nodes) =
                    if let Some((lowest_round, bitmask, missing_nodes)) = dag_reader.bitmask() {
                        (lowest_round, bitmask, missing_nodes)
                    } else {
                        error!("Incomplete round not found, but fetch request received");
                        continue;
                    };

                RemoteFetchRequest::new(
                    local_request.node().metadata().clone(),
                    lowest_round,
                    bitmask,
                    missing_nodes,
                )
            };
            let network_request = DAGMessage::from(remote_request.clone()).into_network_message();
            if let Ok(response) = self
                .network
                .send_rpc_with_fallbacks(responders, network_request, Duration::from_secs(1))
                .await
                .and_then(DAGMessage::try_from)
                .and_then(FetchResponse::try_from)
                .and_then(|response| response.verify(&remote_request, &self.epoch_state.verifier))
            {
                let ceritified_nodes = response.certified_nodes();
                if ceritified_nodes.len() != remote_request.missing_count() {
                    error!(
                        "expected {} nodes, received {}",
                        remote_request.missing_count(),
                        ceritified_nodes.len()
                    );
                    continue;
                }
                // TODO: support chunk response or fallback to state sync
                let mut dag_writer = self.dag.write();
                for node in ceritified_nodes {
                    if let Err(e) = dag_writer.add_node(node) {
                        error!("Failed to add node {}", e);
                    }
                }
                local_request.notify();
            }
        }
    }
}

#[derive(Debug, ThisError)]
pub enum FetchHandleError {
    #[error("not enough nodes to satisfy request")]
    NotEnoughNodes,
}

pub struct FetchHandler {
    dag: Arc<RwLock<Dag>>,
}

impl FetchHandler {
    pub fn new(dag: Arc<RwLock<Dag>>) -> Self {
        Self { dag }
    }
}

impl RpcHandler for FetchHandler {
    type Request = RemoteFetchRequest;
    type Response = FetchResponse;

    fn process(&mut self, message: Self::Request) -> anyhow::Result<Self::Response> {
        let dag_reader = self.dag.read();

        let nodes = dag_reader.get_missing_nodes(message.start_round(), message.exists_bitmask());
        // If this peer cannot satisfy the request, return an error.
        ensure!(
            nodes.len() == message.missing_count(),
            FetchHandleError::NotEnoughNodes
        );

        Ok(FetchResponse::new(
            message.target().epoch(),
            nodes.iter().map(|n| n.as_ref().clone()).collect(),
        ))
    }
}
