// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use super::adapter::TLedgerInfoProvider;
use crate::{
    counters::{CONSENSUS_PROPOSAL_PENDING_DURATION, CONSENSUS_PROPOSAL_PENDING_ROUNDS},
    dag::{
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
    },
};
use anyhow::{bail, ensure};
use aptos_config::config::DagPayloadConfig;
use aptos_consensus_types::common::{Author, Round};
use aptos_infallible::RwLock;
use aptos_logger::{debug, error, info};
use aptos_types::{epoch_state::EpochState, validator_signer::ValidatorSigner};
use async_trait::async_trait;
use std::{collections::BTreeMap, mem, sync::Arc, time::Duration};

const MAX_ORDERING_PIPELINE_LATENCY_REDUCTION: Duration = Duration::from_secs(1);

pub(crate) struct NodeBroadcastHandler {
    dag: Arc<RwLock<Dag>>,
    votes_by_round_peer: BTreeMap<Round, BTreeMap<Author, Vote>>,
    signer: Arc<ValidatorSigner>,
    epoch_state: Arc<EpochState>,
    storage: Arc<dyn DAGStorage>,
    fetch_requester: Arc<dyn TFetchRequester>,
    payload_config: DagPayloadConfig,
    ledger_info_provider: Arc<dyn TLedgerInfoProvider>,
}

impl NodeBroadcastHandler {
    pub fn new(
        dag: Arc<RwLock<Dag>>,
        signer: Arc<ValidatorSigner>,
        epoch_state: Arc<EpochState>,
        storage: Arc<dyn DAGStorage>,
        fetch_requester: Arc<dyn TFetchRequester>,
        payload_config: DagPayloadConfig,
        ledger_info_provider: Arc<dyn TLedgerInfoProvider>,
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
            ledger_info_provider,
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
        ensure!(node.payload().len() as u64 <= self.payload_config.max_receiving_txns_per_round);
        ensure!(
            node.payload().size() as u64 <= self.payload_config.max_receiving_size_per_round_bytes
        );

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

    fn pipeline_pending_latency(&self, proposal_timestamp: Duration) -> Duration {
        let highest_ordered_anchor = if let Some(node) = self.dag.read().highest_ordered_anchor() {
            node
        } else {
            return Duration::from_secs(0);
        };
        let highest_commit_li = self.ledger_info_provider.get_latest_ledger_info();

        let ordered_round = highest_ordered_anchor.round();
        let commit_round = highest_commit_li.ledger_info().round();

        let pending_rounds = ordered_round.checked_sub(commit_round).unwrap();

        let ordered_timestamp = Duration::from_micros(highest_ordered_anchor.timestamp());
        let committed_timestamp =
            Duration::from_micros(highest_commit_li.ledger_info().timestamp_usecs());

        fn latency_from_proposal(proposal_timestamp: Duration, timestamp: Duration) -> Duration {
            if timestamp.is_zero() {
                // latency not known without non-genesis blocks
                Duration::ZERO
            } else {
                proposal_timestamp.checked_sub(timestamp).unwrap()
            }
        }

        let latency_to_committed = latency_from_proposal(proposal_timestamp, committed_timestamp);
        let latency_to_ordered = latency_from_proposal(proposal_timestamp, ordered_timestamp);

        info!(
            pending_rounds = pending_rounds,
            ordered_round = ordered_round,
            commit_round = commit_round,
            latency_to_ordered_ms = latency_to_ordered.as_millis() as u64,
            latency_to_committed_ms = latency_to_committed.as_millis() as u64,
            "Pipeline pending latency on proposal creation",
        );

        CONSENSUS_PROPOSAL_PENDING_ROUNDS.observe(pending_rounds as f64);
        CONSENSUS_PROPOSAL_PENDING_DURATION.observe(latency_to_committed.as_secs_f64());

        latency_to_committed
            .saturating_sub(latency_to_ordered.min(MAX_ORDERING_PIPELINE_LATENCY_REDUCTION))
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

        let pipeline_delay = self.pipeline_pending_latency(Duration::from_micros(node.timestamp()));
        if pipeline_delay > Duration::from_millis(self.payload_config.pipeline_backpressure_ms) {
            bail!(NodeBroadcastHandleError::PipelineBackpressure);
        }

        observe_node(node.timestamp(), NodeStage::NodeReceived);
        debug!(LogSchema::new(LogEvent::ReceiveNode)
            .remote_peer(*node.author())
            .round(node.round()));

        let votes_by_peer = self
            .votes_by_round_peer
            .entry(node.metadata().round())
            .or_insert(BTreeMap::new());
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
