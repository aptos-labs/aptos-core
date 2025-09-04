// Copyright © Velor Foundation
// SPDX-License-Identifier: Apache-2.0

use super::dag_store::DagStore;
use crate::dag::{
    adapter::OrderedNotifier,
    anchor_election::AnchorElection,
    dag_store::NodeStatus,
    observability::{
        logging::{LogEvent, LogSchema},
        tracing::{observe_node, NodeStage},
    },
    storage::CommitEvent,
    types::NodeMetadata,
    CertifiedNode,
};
use velor_consensus_types::common::Round;
use velor_infallible::Mutex;
use velor_logger::debug;
use velor_types::epoch_state::EpochState;
use std::sync::Arc;

pub trait TOrderRule: Send + Sync {
    fn process_new_node(&self, node_metadata: &NodeMetadata);

    fn process_all(&self);
}

pub struct OrderRule {
    epoch_state: Arc<EpochState>,
    lowest_unordered_anchor_round: Round,
    dag: Arc<DagStore>,
    anchor_election: Arc<dyn AnchorElection>,
    notifier: Arc<dyn OrderedNotifier>,
    dag_window_size_config: Round,
}

impl OrderRule {
    pub fn new(
        epoch_state: Arc<EpochState>,
        lowest_unordered_anchor_round: Round,
        dag: Arc<DagStore>,
        anchor_election: Arc<dyn AnchorElection>,
        notifier: Arc<dyn OrderedNotifier>,
        dag_window_size_config: Round,
        commit_events: Option<Vec<CommitEvent>>,
    ) -> Self {
        if let Some(commit_events) = commit_events {
            // make sure it's sorted
            assert!(commit_events
                .windows(2)
                .all(|w| (w[0].epoch(), w[0].round()) < (w[1].epoch(), w[1].round())));
            for event in commit_events {
                if event.epoch() == epoch_state.epoch {
                    let maybe_anchor = dag
                        .read()
                        .get_node_by_round_author(event.round(), event.author())
                        .cloned();
                    if let Some(anchor) = maybe_anchor {
                        dag.write()
                            .reachable_mut(&anchor, None)
                            .for_each(|node_status| node_status.mark_as_ordered());
                    }
                }
                anchor_election.update_reputation(event);
            }
        }
        let mut order_rule = Self {
            epoch_state,
            lowest_unordered_anchor_round,
            dag,
            anchor_election,
            notifier,
            dag_window_size_config,
        };
        // re-check if anything can be ordered to recover pending anchors
        order_rule.process_all();
        order_rule
    }

    /// Check if two rounds have the same parity
    fn check_parity(r1: Round, r2: Round) -> bool {
        (r1 ^ r2) & 1 == 0
    }

    /// Find if there's anchors that can be ordered start from `start_round` until `round`,
    /// if so find next one until nothing can be ordered.
    fn check_ordering_between(&mut self, mut start_round: Round, round: Round) {
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
    fn find_first_anchor_with_enough_votes(
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
            } else {
                debug!(
                    anchor = anchor_author,
                    "Anchor not found for round {}", start_round
                );
            }
            start_round += 2;
        }
        None
    }

    /// Follow an anchor with enough votes to find the first anchor that's recursively reachable by its suffix anchor
    fn find_first_anchor_to_order(
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
                Some(current_anchor.metadata().clone()).iter(),
                Some(self.lowest_unordered_anchor_round),
                |node_status| matches!(node_status, NodeStatus::Unordered { .. }),
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
    fn finalize_order(&mut self, anchor: Arc<CertifiedNode>) {
        // Check we're in the expected instance
        assert!(Self::check_parity(
            self.lowest_unordered_anchor_round,
            anchor.round(),
        ));
        let lowest_round_to_reach = anchor.round().saturating_sub(self.dag_window_size_config);

        // Ceil it to the closest unordered anchor round
        let lowest_anchor_round = std::cmp::max(
            self.lowest_unordered_anchor_round,
            lowest_round_to_reach
                + !Self::check_parity(lowest_round_to_reach, anchor.round()) as u64,
        );
        assert!(Self::check_parity(lowest_anchor_round, anchor.round()));

        let failed_authors_and_rounds: Vec<_> = (lowest_anchor_round..anchor.round())
            .step_by(2)
            .map(|failed_round| (failed_round, self.anchor_election.get_anchor(failed_round)))
            .collect();
        let parents = anchor
            .parents()
            .iter()
            .map(|cert| *cert.metadata().author())
            .collect();
        let event = CommitEvent::new(
            anchor.id(),
            parents,
            failed_authors_and_rounds
                .iter()
                .map(|(_, author)| *author)
                .collect(),
        );
        self.anchor_election.update_reputation(event);

        let mut dag_writer = self.dag.write();
        let mut ordered_nodes: Vec<_> = dag_writer
            .reachable_mut(&anchor, Some(lowest_round_to_reach))
            .map(|node_status| {
                node_status.mark_as_ordered();
                node_status.as_node().clone()
            })
            .collect();

        observe_node(anchor.timestamp(), NodeStage::AnchorOrdered);
        for node in ordered_nodes.iter().skip(1) {
            observe_node(node.timestamp(), NodeStage::NodeOrdered);
        }
        ordered_nodes.reverse();

        debug!(
            LogSchema::new(LogEvent::OrderedAnchor),
            id = anchor.id(),
            lowest_unordered_anchor_round = self.lowest_unordered_anchor_round,
            "Reached round {} with {} nodes",
            lowest_anchor_round,
            ordered_nodes.len()
        );

        self.lowest_unordered_anchor_round = anchor.round() + 1;
        self.notifier
            .send_ordered_nodes(ordered_nodes, failed_authors_and_rounds);
    }

    /// Check if this node can trigger anchors to be ordered
    pub fn process_new_node(&mut self, node_metadata: &NodeMetadata) {
        let round = node_metadata.round();

        debug!(
            lowest_unordered_round = self.lowest_unordered_anchor_round,
            node_round = round,
            "Trigger Ordering"
        );
        // If the node comes from the proposal round in the current instance, it can't trigger any ordering
        if round <= self.lowest_unordered_anchor_round
            || Self::check_parity(round, self.lowest_unordered_anchor_round)
        {
            return;
        }
        // This node's votes can trigger an anchor from previous round to be ordered.
        let start_round = round - 1;
        self.check_ordering_between(start_round, round)
    }

    /// Check the whole dag to see if anything can be ordered.
    pub fn process_all(&mut self) {
        let start_round = self.lowest_unordered_anchor_round;
        let round = self.dag.read().highest_round();
        self.check_ordering_between(start_round, round);
    }
}

impl TOrderRule for Mutex<OrderRule> {
    fn process_new_node(&self, node_metadata: &NodeMetadata) {
        self.lock().process_new_node(node_metadata)
    }

    fn process_all(&self) {
        self.lock().process_all()
    }
}
