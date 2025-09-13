// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use super::{dag_store::DagStore, health::HealthBackoff, order_rule::TOrderRule};
use crate::{
    dag::{
        dag_fetcher::TFetchRequester,
        dag_network::RpcHandler,
        errors::NodeBroadcastHandleError,
        observability::{
            logging::{LogEvent, LogSchema},
            tracing::{observe_node, NodeStage},
        },
        storage::DAGStorage,
        types::{Node, NodeCertificate, Vote},
        NodeId,
    },
    util::is_vtxn_expected,
};
use anyhow::{bail, ensure, Context};
use aptos_config::config::DagPayloadConfig;
use aptos_consensus_types::common::{Author, Round};
use aptos_infallible::Mutex;
use aptos_logger::{debug, error};
use aptos_types::{
    epoch_state::EpochState,
    on_chain_config::{OnChainJWKConsensusConfig, OnChainRandomnessConfig, ValidatorTxnConfig},
    validator_signer::ValidatorSigner,
    validator_txn::ValidatorTransaction,
};
use async_trait::async_trait;
use claims::assert_some;
use dashmap::DashSet;
use std::{collections::BTreeMap, mem, sync::Arc};

pub(crate) struct NodeBroadcastHandler {
    dag: Arc<DagStore>,
    order_rule: Arc<dyn TOrderRule>,
    /// Note: The mutex around BTreeMap is to work around Rust Sync semantics.
    /// Fine grained concurrency is implemented by the DashSet below.
    votes_by_round_peer: Mutex<BTreeMap<Round, BTreeMap<Author, Vote>>>,
    votes_fine_grained_lock: DashSet<(Round, Author)>,
    signer: Arc<ValidatorSigner>,
    epoch_state: Arc<EpochState>,
    storage: Arc<dyn DAGStorage>,
    fetch_requester: Arc<dyn TFetchRequester>,
    payload_config: DagPayloadConfig,
    vtxn_config: ValidatorTxnConfig,
    randomness_config: OnChainRandomnessConfig,
    jwk_consensus_config: OnChainJWKConsensusConfig,
    health_backoff: HealthBackoff,
}

impl NodeBroadcastHandler {
    pub fn new(
        dag: Arc<DagStore>,
        order_rule: Arc<dyn TOrderRule>,
        signer: Arc<ValidatorSigner>,
        epoch_state: Arc<EpochState>,
        storage: Arc<dyn DAGStorage>,
        fetch_requester: Arc<dyn TFetchRequester>,
        payload_config: DagPayloadConfig,
        vtxn_config: ValidatorTxnConfig,
        randomness_config: OnChainRandomnessConfig,
        jwk_consensus_config: OnChainJWKConsensusConfig,
        health_backoff: HealthBackoff,
    ) -> Self {
        let epoch = epoch_state.epoch;
        let votes_by_round_peer = read_votes_from_storage(&storage, epoch);

        Self {
            dag,
            order_rule,
            votes_by_round_peer: Mutex::new(votes_by_round_peer),
            votes_fine_grained_lock: DashSet::with_capacity(epoch_state.verifier.len() * 10),
            signer,
            epoch_state,
            storage,
            fetch_requester,
            payload_config,
            vtxn_config,
            randomness_config,
            jwk_consensus_config,
            health_backoff,
        }
    }

    pub fn gc(&self) {
        let lowest_round = self.dag.read().lowest_round();
        if let Err(e) = self.gc_before_round(lowest_round) {
            error!("Error deleting votes: {}", e);
        }
    }

    pub fn gc_before_round(&self, min_round: Round) -> anyhow::Result<()> {
        let mut votes_by_round_peer_guard = self.votes_by_round_peer.lock();
        let to_retain = votes_by_round_peer_guard.split_off(&min_round);
        let to_delete = mem::replace(&mut *votes_by_round_peer_guard, to_retain);
        drop(votes_by_round_peer_guard);

        let to_delete = to_delete
            .iter()
            .flat_map(|(r, peer_and_digest)| {
                peer_and_digest
                    .keys()
                    .map(|author| NodeId::new(self.epoch_state.epoch, *r, *author))
            })
            .collect();
        self.storage.delete_votes(to_delete)
    }

    fn validate(&self, node: Node) -> anyhow::Result<Node> {
        ensure!(
            node.epoch() == self.epoch_state.epoch,
            "different epoch {}, current {}",
            node.epoch(),
            self.epoch_state.epoch
        );

        let num_vtxns = node.validator_txns().len() as u64;
        ensure!(num_vtxns <= self.vtxn_config.per_block_limit_txn_count());
        for vtxn in node.validator_txns() {
            let vtxn_type_name = vtxn.type_name();
            ensure!(
                is_vtxn_expected(&self.randomness_config, &self.jwk_consensus_config, vtxn),
                "unexpected validator transaction: {:?}",
                vtxn_type_name
            );
            vtxn.verify(self.epoch_state.verifier.as_ref())
                .context(format!("{} verification failed", vtxn_type_name))?;
        }
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

    async fn process(&self, node: Self::Request) -> anyhow::Result<Self::Response> {
        ensure!(
            !self.health_backoff.stop_voting(),
            NodeBroadcastHandleError::VoteRefused
        );

        let key = (node.round(), *node.author());
        ensure!(
            self.votes_fine_grained_lock.insert(key),
            "concurrent insertion"
        );
        defer!({
            assert_some!(self.votes_fine_grained_lock.remove(&key));
        });

        let node = self.validate(node)?;
        observe_node(node.timestamp(), NodeStage::NodeReceived);
        debug!(LogSchema::new(LogEvent::ReceiveNode)
            .remote_peer(*node.author())
            .round(node.round()));

        if let Some(ack) = self
            .votes_by_round_peer
            .lock()
            .entry(node.round())
            .or_default()
            .get(node.author())
        {
            return Ok(ack.clone());
        }

        let signature = node.sign_vote(&self.signer)?;
        let vote = Vote::new(node.metadata().clone(), signature);
        self.storage.save_vote(&node.id(), &vote)?;
        self.votes_by_round_peer
            .lock()
            .get_mut(&node.round())
            .expect("must exist")
            .insert(*node.author(), vote.clone());

        self.dag.write().update_votes(&node, false);
        self.order_rule.process_new_node(node.metadata());

        debug!(LogSchema::new(LogEvent::Vote)
            .remote_peer(*node.author())
            .round(node.round()));
        Ok(vote)
    }
}
