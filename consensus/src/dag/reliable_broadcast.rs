// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use super::{
    storage::DAGStorage,
    types::{CertifiedAck, CertifiedNode},
    NodeId,
};
use crate::{
    dag::{
        dag_network::{DAGNetworkSender, RpcHandler},
        dag_store::Dag,
        types::{Node, NodeCertificate, TDAGMessage, Vote},
    },
    network::TConsensusMsg,
};
use anyhow::{bail, ensure};
use aptos_consensus_types::common::{Author, Round};
use aptos_infallible::RwLock;
use aptos_logger::error;
use aptos_types::{epoch_state::EpochState, validator_signer::ValidatorSigner};
use futures::{stream::FuturesUnordered, StreamExt};
use std::{collections::BTreeMap, future::Future, mem, sync::Arc, time::Duration};
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
    votes_by_round_peer: BTreeMap<Round, BTreeMap<Author, Vote>>,
    signer: ValidatorSigner,
    epoch_state: Arc<EpochState>,
    storage: Arc<dyn DAGStorage>,
}

impl NodeBroadcastHandler {
    pub fn new(
        dag: Arc<RwLock<Dag>>,
        signer: ValidatorSigner,
        epoch_state: Arc<EpochState>,
        storage: Arc<dyn DAGStorage>,
    ) -> Self {
        let epoch = epoch_state.epoch;
        let votes_by_round_peer = read_votes_from_storage(&storage, epoch);

        Self {
            dag,
            votes_by_round_peer,
            signer,
            epoch_state,
            storage,
        }
    }

    pub fn gc_before_round(&mut self, min_round: Round) -> anyhow::Result<()> {
        let to_retain = self.votes_by_round_peer.split_off(&min_round);
        let to_delete = mem::replace(&mut self.votes_by_round_peer, to_retain);

        let to_delete = to_delete
            .iter()
            .flat_map(|(r, peer_and_digest)| {
                peer_and_digest
                    .iter()
                    .map(|(author, _)| NodeId::new(self.epoch_state.epoch, *r, *author))
            })
            .collect();
        self.storage.delete_votes(to_delete)
    }

    fn validate(&self, node: &Node) -> anyhow::Result<()> {
        let current_round = node.metadata().round();

        // round 0 is a special case and does not require any parents
        if current_round == 0 {
            return Ok(());
        }

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
            .filter(|parent| !dag_reader.exists(parent.metadata()))
            .cloned()
            .collect();
        if !missing_parents.is_empty() {
            // For each missing parent, verify their signatures and voting power
            ensure!(
                missing_parents
                    .iter()
                    .all(|parent| { parent.verify(&self.epoch_state.verifier).is_ok() }),
                NodeBroadcastHandleError::InvalidParent
            );
            // TODO: notify dag fetcher to fetch missing node and drop this node
            bail!(NodeBroadcastHandleError::MissingParents);
        }

        Ok(())
    }
}

fn read_votes_from_storage(
    storage: &Arc<dyn DAGStorage>,
    epoch: u64,
) -> BTreeMap<u64, BTreeMap<Author, Vote>> {
    let mut votes_by_round_peer = BTreeMap::new();

    let all_votes = storage.get_votes().unwrap_or_default();
    let mut to_delete = vec![];
    for (node_id, vote) in all_votes {
        if node_id.epoch() == epoch {
            votes_by_round_peer
                .entry(node_id.round())
                .or_insert_with(BTreeMap::new)
                .insert(node_id.author(), vote);
        } else {
            to_delete.push(node_id);
        }
    }
    if let Err(err) = storage.delete_votes(to_delete) {
        error!("unable to clear old signatures: {}", err);
    }

    votes_by_round_peer
}

impl RpcHandler for NodeBroadcastHandler {
    type Request = Node;
    type Response = Vote;

    fn process(&mut self, node: Self::Request) -> anyhow::Result<Self::Response> {
        self.validate(&node)?;

        let votes_by_peer = self
            .votes_by_round_peer
            .entry(node.metadata().round())
            .or_insert(BTreeMap::new());
        match votes_by_peer.get(node.metadata().author()) {
            None => {
                let signature = node.sign_vote(&self.signer)?;
                let vote = Vote::new(node.metadata().clone(), signature);

                self.storage.save_vote(&node.id(), &vote)?;
                votes_by_peer.insert(*node.metadata().author(), vote.clone());

                Ok(vote)
            },
            Some(ack) => Ok(ack.clone()),
        }
    }
}

#[derive(Debug, ThisError)]
pub enum CertifiedNodeHandleError {
    #[error("node already exists")]
    NodeExists,
    #[error("missing parents")]
    MissingParents,
}

pub struct CertifiedNodeHandler {
    dag: Arc<RwLock<Dag>>,
}

impl CertifiedNodeHandler {
    pub fn new(dag: Arc<RwLock<Dag>>) -> Self {
        Self { dag }
    }
}

impl RpcHandler for CertifiedNodeHandler {
    type Request = CertifiedNode;
    type Response = CertifiedAck;

    fn process(&mut self, node: Self::Request) -> anyhow::Result<Self::Response> {
        let epoch = node.metadata().epoch();
        {
            let dag_reader = self.dag.read();
            if dag_reader.exists(node.metadata()) {
                return Ok(CertifiedAck::new(node.metadata().epoch()));
            }

            if !dag_reader.all_exists(node.parents()) {
                // TODO(ibalajiarun): implement fetching logic.
                bail!(CertifiedNodeHandleError::MissingParents);
            }
        }

        let mut dag_writer = self.dag.write();
        dag_writer.add_node(node)?;

        Ok(CertifiedAck::new(epoch))
    }
}
