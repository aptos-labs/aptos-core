// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use super::dag_store::NodeStatus;
use crate::dag::{
    anchor_election::AnchorElection, dag_store::Dag, storage::DAGStorage, types::NodeMetadata,
    CertifiedNode,
};
use aptos_consensus_types::common::{Author, Round};
use aptos_infallible::RwLock;
use aptos_logger::error;
use aptos_types::{epoch_state::EpochState, ledger_info::LedgerInfo};
use std::sync::Arc;

pub trait Notifier: Send {
    fn send(
        &mut self,
        ordered_nodes: Vec<Arc<CertifiedNode>>,
        failed_author: Vec<(Round, Author)>,
    ) -> anyhow::Result<()>;
}

pub struct OrderRule {
    epoch_state: Arc<EpochState>,
    lowest_unordered_anchor_round: Round,
    dag: Arc<RwLock<Dag>>,
    anchor_election: Box<dyn AnchorElection>,
    notifier: Box<dyn Notifier>,
    storage: Arc<dyn DAGStorage>,
}

impl OrderRule {
    pub fn new(
        epoch_state: Arc<EpochState>,
        latest_ledger_info: LedgerInfo,
        dag: Arc<RwLock<Dag>>,
        anchor_election: Box<dyn AnchorElection>,
        notifier: Box<dyn Notifier>,
        storage: Arc<dyn DAGStorage>,
    ) -> Self {
        // TODO: we need to initialize the anchor election based on the dag
        let mut anchors = storage.get_ordered_anchor_ids().unwrap();
        let mut expired = anchors.clone();
        expired.retain(|(id, _)| id.epoch() < epoch_state.epoch);
        if let Err(e) =
            storage.delete_ordered_anchor_ids(expired.into_iter().map(|(id, _)| id).collect())
        {
            error!("Failed to delete expired anchors: {:?}", e);
        }
        anchors.retain(|(id, _)| id.epoch() == epoch_state.epoch);
        let committed_round = if latest_ledger_info.ends_epoch() {
            0
        } else {
            latest_ledger_info.round()
        };
        let mut order_rule = Self {
            epoch_state,
            lowest_unordered_anchor_round: latest_ledger_info.commit_info().round() + 1,
            dag,
            anchor_election,
            notifier,
            storage,
        };
        // Sort by round first, TODO: make the schema encode support the ordering directly
        anchors.sort_by(|(a, _), (b, _)| a.round().cmp(&b.round()));
        for (id, _) in anchors {
            let maybe_anchor = order_rule
                .dag
                .read()
                .get_node_by_round_author(id.round(), id.author())
                .cloned();
            if id.round() <= committed_round {
                // mark already committed node
                if let Some(anchor) = maybe_anchor {
                    order_rule
                        .dag
                        .write()
                        .reachable_mut(&anchor, None)
                        .for_each(|node_statue| node_statue.mark_as_ordered());
                }
            } else {
                // re-process pending anchors
                order_rule.finalize_order(maybe_anchor.expect("Uncommitted anchor should exist"));
            }
        }
        order_rule
    }

    /// Check if two rounds have the same parity
    fn check_parity(r1: Round, r2: Round) -> bool {
        (r1 ^ r2) & 1 == 0
    }

    pub fn process_new_node(&mut self, node_metadata: &NodeMetadata) {
        let round = node_metadata.round();
        // If the node comes from the proposal round in the current instance, it can't trigger any ordering
        if round <= self.lowest_unordered_anchor_round
            || Self::check_parity(round, self.lowest_unordered_anchor_round)
        {
            return;
        }
        // This node's votes can trigger an anchor from previous round to be ordered.
        let mut start_round = round - 1;
        while start_round <= round {
            if let Some(direct_anchor) =
                self.find_first_anchor_with_enough_votes(start_round, round)
            {
                let ordered_anchor = self.find_first_anchor_to_order(direct_anchor);
                self.finalize_order(ordered_anchor);
                // if there's any anchor being ordered, the loop continues to check if new anchor can be ordered as well.
                start_round = self.lowest_unordered_anchor_round;
            } else {
                break;
            }
        }
    }

    /// From the start round until the target_round, try to find if there's any anchor has enough votes to trigger ordering
    pub fn find_first_anchor_with_enough_votes(
        &self,
        mut start_round: Round,
        target_round: Round,
    ) -> Option<Arc<CertifiedNode>> {
        let dag_reader = self.dag.read();
        while start_round < target_round {
            let anchor_author = self.anchor_election.get_anchor(start_round);
            // I "think" it's impossible to get ordered/committed node here but to double check
            if let Some(anchor_node) =
                dag_reader.get_node_by_round_author(start_round, &anchor_author)
            {
                // f+1 or 2f+1?
                if dag_reader
                    .check_votes_for_node(anchor_node.metadata(), &self.epoch_state.verifier)
                {
                    return Some(anchor_node.clone());
                }
            }
            start_round += 2;
        }
        None
    }

    /// Follow an anchor with enough votes to find the first anchor that's recursively reachable by its suffix anchor
    pub fn find_first_anchor_to_order(
        &self,
        mut current_anchor: Arc<CertifiedNode>,
    ) -> Arc<CertifiedNode> {
        let dag_reader = self.dag.read();
        let anchor_round = current_anchor.round();
        let is_anchor = |metadata: &NodeMetadata| -> bool {
            Self::check_parity(metadata.round(), anchor_round)
                && *metadata.author() == self.anchor_election.get_anchor(metadata.round())
        };
        while let Some(prev_anchor) = dag_reader
            .reachable(
                &[current_anchor.metadata().clone()],
                Some(self.lowest_unordered_anchor_round),
                |node_status| matches!(node_status, NodeStatus::Unordered(_)),
            )
            // skip the current anchor itself
            .skip(1)
            .map(|node_status| node_status.as_node())
            .find(|node| is_anchor(node.metadata()))
        {
            current_anchor = prev_anchor.clone();
        }
        current_anchor
    }

    /// Finalize the ordering with the given anchor node, update anchor election and construct blocks for execution.
    pub fn finalize_order(&mut self, anchor: Arc<CertifiedNode>) {
        let failed_authors: Vec<_> = (self.lowest_unordered_anchor_round..anchor.round())
            .step_by(2)
            .map(|failed_round| (failed_round, self.anchor_election.get_anchor(failed_round)))
            .collect();
        assert!(Self::check_parity(
            self.lowest_unordered_anchor_round,
            anchor.round(),
        ));
        self.lowest_unordered_anchor_round = anchor.round() + 1;

        let mut dag_writer = self.dag.write();
        let mut ordered_nodes: Vec<_> = dag_writer
            .reachable_mut(&anchor, None)
            .map(|node_status| {
                node_status.mark_as_ordered();
                node_status.as_node().clone()
            })
            .collect();
        ordered_nodes.reverse();
        if let Err(e) = self
            .storage
            .save_ordered_anchor_id(&anchor.id())
            .and_then(|_| self.notifier.send(ordered_nodes, failed_authors))
        {
            error!("Failed to send ordered nodes {:?}", e);
        }
    }
}
