// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{
    dag::{
        dag_network::{DAGNetworkSender, RpcHandler},
        dag_store::Dag,
        types::{Node, NodeCertificate, NodeDigest, NodeDigestSignature, TDAGMessage},
    },
    network::TConsensusMsg,
};
use anyhow::{bail, ensure};
use aptos_consensus_types::common::{Author, Round};
use aptos_infallible::RwLock;
use aptos_types::{validator_signer::ValidatorSigner, validator_verifier::ValidatorVerifier};
use futures::{stream::FuturesUnordered, StreamExt};
use std::{collections::BTreeMap, future::Future, sync::Arc, time::Duration};
use thiserror::Error as ThisError;

pub trait BroadcastStatus {
    type Ack: TDAGMessage;
    type Aggregated;
    type Message: TDAGMessage;

    fn add(&mut self, peer: Author, ack: Self::Ack) -> anyhow::Result<Option<Self::Aggregated>>;
}

pub struct ReliableBroadcast {
    validators: Vec<Author>,
    network_sender: Arc<dyn DAGNetworkSender>,
}

impl ReliableBroadcast {
    pub fn new(validators: Vec<Author>, network_sender: Arc<dyn DAGNetworkSender>) -> Self {
        Self {
            validators,
            network_sender,
        }
    }

    pub fn broadcast<S: BroadcastStatus>(
        &self,
        message: S::Message,
        mut aggregating: S,
    ) -> impl Future<Output = S::Aggregated> {
        let receivers: Vec<_> = self.validators.clone();
        let network_sender = self.network_sender.clone();
        async move {
            let mut fut = FuturesUnordered::new();
            let send_message = |receiver, message| {
                let network_sender = network_sender.clone();
                async move {
                    (
                        receiver,
                        network_sender
                            .send_rpc(receiver, message, Duration::from_millis(500))
                            .await,
                    )
                }
            };
            let network_message = message.into().into_network_message();
            for receiver in receivers {
                fut.push(send_message(receiver, network_message.clone()));
            }
            while let Some((receiver, result)) = fut.next().await {
                match result {
                    Ok(msg) => {
                        if let Ok(dag_msg) = msg.try_into() {
                            if let Ok(ack) = S::Ack::try_from(dag_msg) {
                                if let Ok(Some(aggregated)) = aggregating.add(receiver, ack) {
                                    return aggregated;
                                }
                            }
                        }
                    },
                    Err(_) => fut.push(send_message(receiver, network_message.clone())),
                }
            }
            unreachable!("Should aggregate with all responses");
        }
    }
}

#[derive(ThisError, Debug)]
pub enum NodeBroadcastHandleError {
    #[error("invalid parent in node")]
    InvalidParent,
    #[error("missing parents")]
    MissingParents,
    #[error("parents do not meet quorum voting power")]
    NotEnoughParents,
}

pub struct NodeBroadcastHandler {
    dag: Arc<RwLock<Dag>>,
    signatures_by_round_peer: BTreeMap<Round, BTreeMap<Author, NodeDigestSignature>>,
    signer: ValidatorSigner,
    verifier: ValidatorVerifier,
}

impl NodeBroadcastHandler {
    pub fn new(
        dag: Arc<RwLock<Dag>>,
        signer: ValidatorSigner,
        verifier: ValidatorVerifier,
    ) -> Self {
        Self {
            dag,
            signatures_by_round_peer: BTreeMap::new(),
            signer,
            verifier,
        }
    }

    pub fn gc_before_round(&mut self, min_round: Round) {
        self.signatures_by_round_peer.retain(|r, _| r >= &min_round);
    }

    fn validate(&self, node: &Node) -> anyhow::Result<()> {
        let current_round = node.metadata().round();
        // round 0 is a special case and does not require any parents
        if current_round == 0 {
            return Ok(());
        }

        node.verify(&self.verifier)?;

        let prev_round = current_round - 1;

        let dag_reader = self.dag.read();
        // check if the parent round is missing in the DAG
        ensure!(
            prev_round >= dag_reader.lowest_round(),
            NodeBroadcastHandleError::MissingParents
        );

        // check which parents are missing in the DAG
        let missing_parents: Vec<NodeCertificate> = node
            .parents()
            .iter()
            .filter(|parent| !dag_reader.exists(parent.metadata().digest()))
            .cloned()
            .collect();
        if !missing_parents.is_empty() {
            // For each missing parent, verify their signatures and voting power
            ensure!(
                missing_parents.iter().all(|parent| {
                    let node_digest = NodeDigest::new(*parent.metadata().digest());
                    self.verifier
                        .verify_multi_signatures(&node_digest, parent.signatures())
                        .is_ok()
                }),
                NodeBroadcastHandleError::InvalidParent
            );
            // TODO: notify dag fetcher to fetch missing node and drop this node
            bail!(NodeBroadcastHandleError::MissingParents);
        }

        Ok(())
    }
}

impl RpcHandler for NodeBroadcastHandler {
    type Request = Node;
    type Response = NodeDigestSignature;

    fn process(&mut self, node: Self::Request) -> anyhow::Result<Self::Response> {
        self.validate(&node)?;

        let signatures_by_peer = self
            .signatures_by_round_peer
            .entry(node.metadata().round())
            .or_insert(BTreeMap::new());
        // TODO(ibalajiarun): persist node before voting
        match signatures_by_peer.get(node.metadata().author()) {
            None => {
                let signature = node.sign(&self.signer)?;
                let digest_signature =
                    NodeDigestSignature::new(node.metadata().epoch(), node.digest(), signature);
                signatures_by_peer.insert(*node.metadata().author(), digest_signature.clone());
                Ok(digest_signature)
            },
            Some(ack) => Ok(ack.clone()),
        }
    }
}
