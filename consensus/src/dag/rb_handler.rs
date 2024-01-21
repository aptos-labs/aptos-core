// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::dag::{
    dag_fetcher::TFetchRequester,
    dag_network::RpcHandler,
    dag_store::Dag,
    errors::NodeBroadcastHandleError,
    observability::{
        logging::{LogEvent, LogSchema},
        tracing::{observe_node, NodeStage},
    },
    storage::DAGStorage,
    types::{Node, NodeCertificate, Vote},
    NodeId,
};
use anyhow::{bail, ensure};
use aptos_config::config::DagPayloadConfig;
use aptos_consensus_types::common::{Author, Round};
use aptos_infallible::RwLock;
use aptos_logger::{debug, error};
use aptos_types::{
    epoch_state::EpochState, on_chain_config::ValidatorTxnConfig,
    validator_signer::ValidatorSigner, validator_txn::ValidatorTransaction,
};
use async_trait::async_trait;
use std::{collections::BTreeMap, mem, sync::Arc};

pub(crate) struct NodeBroadcastHandler {
    dag: Arc<RwLock<Dag>>,
    votes_by_round_peer: BTreeMap<Round, BTreeMap<Author, Vote>>,
    signer: Arc<ValidatorSigner>,
    epoch_state: Arc<EpochState>,
    storage: Arc<dyn DAGStorage>,
    fetch_requester: Arc<dyn TFetchRequester>,
    payload_config: DagPayloadConfig,
    vtxn_config: ValidatorTxnConfig,
}

impl NodeBroadcastHandler {
    pub fn new(
        dag: Arc<RwLock<Dag>>,
        signer: Arc<ValidatorSigner>,
        epoch_state: Arc<EpochState>,
        storage: Arc<dyn DAGStorage>,
        fetch_requester: Arc<dyn TFetchRequester>,
        payload_config: DagPayloadConfig,
        vtxn_config: ValidatorTxnConfig,
    ) -> Self {
        let epoch = epoch_state.epoch;
        let votes_by_round_peer = read_votes_from_storage(&storage, epoch);

        Self {
            dag,
            votes_by_round_peer,
            signer,
            epoch_state,
            storage,
            fetch_requester,
            payload_config,
            vtxn_config,
        }
    }

    pub fn gc(&mut self) {
        let lowest_round = self.dag.read().lowest_round();
        if let Err(e) = self.gc_before_round(lowest_round) {
            error!("Error deleting votes: {}", e);
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

    fn validate(&self, node: Node) -> anyhow::Result<Node> {
        let num_vtxns = node.validator_txns().len() as u64;
        ensure!(num_vtxns <= self.vtxn_config.per_block_limit_txn_count());
        let vtxn_total_bytes = node
            .validator_txns()
            .iter()
            .map(ValidatorTransaction::size_in_bytes)
            .sum::<usize>() as u64;
        ensure!(vtxn_total_bytes <= self.vtxn_config.per_block_limit_total_bytes());

        let num_txns = num_vtxns + node.payload().len() as u64;
        let txn_bytes = vtxn_total_bytes + node.payload().size() as u64;
        ensure!(num_txns <= self.payload_config.max_receiving_txns_per_round);
        ensure!(txn_bytes <= self.payload_config.max_receiving_size_per_round_bytes);

        let current_round = node.metadata().round();

        let dag_reader = self.dag.read();
        let lowest_round = dag_reader.lowest_round();

        ensure!(
            current_round >= lowest_round,
            NodeBroadcastHandleError::StaleRound(current_round)
        );

        // check which parents are missing in the DAG
        let missing_parents: Vec<NodeCertificate> = node
            .parents()
            .iter()
            .filter(|parent| !dag_reader.exists(parent.metadata()))
            .cloned()
            .collect();
        drop(dag_reader); // Drop the DAG store early as it is no longer required

        if !missing_parents.is_empty() {
            // For each missing parent, verify their signatures and voting power.
            // Otherwise, a malicious node can send bad nodes with fake parents
            // and cause this peer to issue unnecessary fetch requests.
            ensure!(
                missing_parents
                    .iter()
                    .all(|parent| { parent.verify(&self.epoch_state.verifier).is_ok() }),
                NodeBroadcastHandleError::InvalidParent
            );

            // Don't issue fetch requests for parents of the lowest round in the DAG
            // because they are already GC'ed
            if current_round > lowest_round {
                if let Err(err) = self.fetch_requester.request_for_node(node) {
                    error!("request to fetch failed: {}", err);
                }
                bail!(NodeBroadcastHandleError::MissingParents);
            }
        }

        Ok(node)
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
                .insert(*node_id.author(), vote);
        } else {
            to_delete.push(node_id);
        }
    }
    if let Err(err) = storage.delete_votes(to_delete) {
        error!("unable to clear old signatures: {}", err);
    }

    votes_by_round_peer
}

#[async_trait]
impl RpcHandler for NodeBroadcastHandler {
    type Request = Node;
    type Response = Vote;

    async fn process(&mut self, node: Self::Request) -> anyhow::Result<Self::Response> {
        let node = self.validate(node)?;
        observe_node(node.timestamp(), NodeStage::NodeReceived);
        debug!(LogSchema::new(LogEvent::ReceiveNode)
            .remote_peer(*node.author())
            .round(node.round()));

        let votes_by_peer = self
            .votes_by_round_peer
            .entry(node.metadata().round())
            .or_default();
        match votes_by_peer.get(node.metadata().author()) {
            None => {
                let signature = node.sign_vote(&self.signer)?;
                let vote = Vote::new(node.metadata().clone(), signature);

                self.storage.save_vote(&node.id(), &vote)?;
                votes_by_peer.insert(*node.author(), vote.clone());

                debug!(LogSchema::new(LogEvent::Vote)
                    .remote_peer(*node.author())
                    .round(node.round()));
                Ok(vote)
            },
            Some(ack) => Ok(ack.clone()),
        }
    }
}
