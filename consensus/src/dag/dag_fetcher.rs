// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{
    dag::{
        dag_network::DAGNetworkSender,
        dag_store::Dag,
        types::{CertifiedNode, DAGMessage, FetchResponse, Node, RemoteFetchRequest},
    },
    network::TConsensusMsg,
};
use aptos_consensus_types::common::Author;
use aptos_infallible::RwLock;
use aptos_logger::error;
use aptos_types::epoch_state::EpochState;
use std::{sync::Arc, time::Duration};
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
                RemoteFetchRequest::new(
                    local_request.node().metadata().clone(),
                    dag_reader.lowest_round(),
                    dag_reader.bitmask(),
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
                // TODO: support chunk response or fallback to state sync
                let mut dag_writer = self.dag.write();
                for rounds in response.certified_nodes() {
                    for node in rounds {
                        if let Err(e) = dag_writer.add_node(node) {
                            error!("Failed to add node {}", e);
                        }
                    }
                }
                local_request.notify();
            }
        }
    }
}
