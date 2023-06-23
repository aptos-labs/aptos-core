// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::dag::{
    dag_store::Dag,
    types::{CertifiedNode, DAGMessage, DAGNetworkSender, Node, NodeMetadata},
};
use aptos_consensus_types::common::{Author, Round};
use aptos_infallible::RwLock;
use aptos_logger::error;
use aptos_types::{epoch_state::EpochState, validator_verifier::ValidatorVerifier};
use serde::{Deserialize, Serialize};
use std::{sync::Arc, time::Duration};
use tokio::sync::{
    mpsc::{Receiver, Sender},
    oneshot,
};

/// Represents a request to fetch missing dependencies for `target`, `start_round` represents
/// the first round we care about in the DAG, `exists_bitmask` is a two dimensional bitmask represents
/// if a node exist at [start_round + index][validator_index].
#[derive(Serialize, Deserialize, Clone)]
struct FetchRequest {
    target: NodeMetadata,
    start_round: Round,
    exists_bitmask: Vec<Vec<bool>>,
}

/// Represents a response to FetchRequest, `certified_nodes` are indexed by [round][validator_index]
/// It should fill in gaps from the `exists_bitmask` according to the parents from the `target_digest` node.
#[derive(Serialize, Deserialize, Clone)]
struct FetchResponse {
    epoch: u64,
    certifies_nodes: Vec<Vec<CertifiedNode>>,
}

impl FetchResponse {
    pub fn verify(
        self,
        _request: &FetchRequest,
        _validator_verifier: &ValidatorVerifier,
    ) -> anyhow::Result<Self> {
        todo!("verification");
    }
}

impl DAGMessage for FetchRequest {
    fn epoch(&self) -> u64 {
        self.target.epoch()
    }
}

impl DAGMessage for FetchResponse {
    fn epoch(&self) -> u64 {
        self.epoch
    }
}

enum FetchCallback {
    Node(Node, oneshot::Sender<Node>),
    CertifiedNode(CertifiedNode, oneshot::Sender<CertifiedNode>),
}

impl FetchCallback {
    pub fn responders(&self, validators: &[Author]) -> Vec<Author> {
        match self {
            FetchCallback::Node(node, _) => vec![*node.author()],
            FetchCallback::CertifiedNode(node, _) => node.certificate().signers(validators),
        }
    }

    pub fn notify(self) {
        if match self {
            FetchCallback::Node(node, sender) => sender.send(node).map_err(|_| ()),
            FetchCallback::CertifiedNode(node, sender) => sender.send(node).map_err(|_| ()),
        }
        .is_err()
        {
            error!("Failed to send node back");
        }
    }
}

struct DagFetcher {
    epoch_state: Arc<EpochState>,
    network: Arc<dyn DAGNetworkSender>,
    dag: Arc<RwLock<Dag>>,
    request_rx: Receiver<(FetchRequest, FetchCallback)>,
}

impl DagFetcher {
    pub fn new(
        epoch_state: Arc<EpochState>,
        network: Arc<dyn DAGNetworkSender>,
        dag: Arc<RwLock<Dag>>,
    ) -> (Self, Sender<(FetchRequest, FetchCallback)>) {
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
        while let Some((request, callback)) = self.request_rx.recv().await {
            let responders =
                callback.responders(&self.epoch_state.verifier.get_ordered_account_addresses());
            let network_request = request.clone().into_network_message();
            if let Ok(response) = self
                .network
                .send_rpc_with_fallbacks(responders, network_request, Duration::from_secs(1))
                .await
                .and_then(FetchResponse::from_network_message)
                .and_then(|response| response.verify(&request, &self.epoch_state.verifier))
            {
                // TODO: support chunk response or fallback to state sync
                let mut dag_writer = self.dag.write();
                for rounds in response.certifies_nodes {
                    for node in rounds {
                        if let Err(e) = dag_writer.add_node(node) {
                            error!("Failed to add node {}", e);
                        }
                    }
                }
                callback.notify();
            }
        }
    }
}
