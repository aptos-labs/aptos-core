use crate::framework::NodeId;
use crate::raikou::types::{BatchSN, Round};
use crate::raikou::{Batch, BatchRef, Block, Config};
use bitvec::prelude::BitVec;
use defaultmap::DefaultBTreeMap;
use itertools::Itertools;
use log::warn;
use std::cmp::{max, min};
use std::collections::{BTreeMap, BTreeSet};
use std::time::Duration;
use tokio::time::Instant;

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
/// Penalty tracker report entry for the optimistic batches from a single node.
pub enum PenaltyTrackerReportEntry {
    // TODO: it should be probably compressed to a single i8 or i16 or f32.
    //       We don't need a lot of precision here.

    /// `Advantage(x)` means that the sender of the report had all the optimistically proposed
    /// batches issued by the node `x` time units before the sender of the report received
    /// the block from the leader.
    Advantage(Duration),
    /// `Missing(sn)` means that the sender of the report was missing the batch with
    /// sequence number `sn` from the node. Of all such batches, the highest-ranked one
    /// (i.e., the first in the block) is reported.
    Missing(BatchSN),
    /// `None` means that there were no batches from this node proposed.
    None,
}

/// Type alias for readability.
type NodeIdMap<T> = Vec<T>;

pub struct PenaltyTracker<S> {
    config: Config<S>,

    batch_receive_time: BTreeMap<BatchRef, Instant>,

    // The variables below are relative to the last round this node was leader.
    last_round_this_node_was_leader: Round,
    proposed_batches: Vec<BatchRef>,
    block_issue_time: Instant,
    received_reports: BTreeSet<NodeId>,
    // The following
    advantage_votes: NodeIdMap<Vec<Duration>>,
    batch_missing_votes: NodeIdMap<Vec<Duration>>,
    penalty: NodeIdMap<Duration>,
}

impl<S> PenaltyTracker<S> {
    pub fn new(config: Config<S>) -> Self {
        let n_nodes = config.n_nodes;

        Self {
            config,
            batch_receive_time: Default::default(),
            last_round_this_node_was_leader: 0,
            proposed_batches: vec![],
            block_issue_time: Instant::now(),
            received_reports: BTreeSet::new(),
            advantage_votes: vec![vec![]; n_nodes],
            batch_missing_votes: vec![vec![]; n_nodes],
            penalty: vec![Duration::ZERO; n_nodes],
        }
    }

    pub fn prepare_reports(
        &self,
        batches: &Vec<BatchRef>,
        block_receive_time: Instant,
    ) -> Vec<PenaltyTrackerReportEntry> {
        assert!(self.config.enable_penalty_system);

        let mut advantages = vec![Duration::MAX; self.config.n_nodes];
        let mut missing = vec![None; self.config.n_nodes];
        for batch_ref in batches.iter().copied() {
            if let Some(batch_receive_time) = self.batch_receive_time.get(&batch_ref).copied() {
                advantages[batch_ref.node] = min(
                    advantages[batch_ref.node],
                    block_receive_time - batch_receive_time,
                );
            } else if missing[batch_ref.node].is_none() {
                missing[batch_ref.node] = Some(batch_ref.sn);
            }
        }

        advantages
            .into_iter()
            .zip(missing)
            .map(|(advantage, missing_sn)| {
                match (advantage, missing_sn) {
                    // No batches from the node.
                    (Duration::MAX, None) => PenaltyTrackerReportEntry::None,
                    // There was a missing batch.
                    (_advantage, Some(sn)) => PenaltyTrackerReportEntry::Missing(sn),
                    // All batches were received `advantage` time units before the block.
                    (advantage, None) => PenaltyTrackerReportEntry::Advantage(advantage),
                }
            })
            .collect()
    }

    pub fn register_reports(
        &mut self,
        round: Round,
        reporting_node: NodeId,
        reports: Vec<PenaltyTrackerReportEntry>,
    ) {
        assert!(self.config.enable_penalty_system);

        if round == self.last_round_this_node_was_leader
            && !self.received_reports.contains(&reporting_node)
        {
            self.received_reports.insert(reporting_node);
            for (node_id, report) in reports.into_iter().enumerate() {
                match report {
                    PenaltyTrackerReportEntry::Advantage(advantage) => {
                        self.advantage_votes[node_id].push(advantage);
                    },
                    PenaltyTrackerReportEntry::Missing(sn) => {
                        let batch_ref = BatchRef { node: node_id, sn };
                        if !self.batch_receive_time.contains_key(&batch_ref) {
                            warn!(
                                "Received a report about an unknown missing batch {:?} \
                                  from node {reporting_node}. Either node {} is Byzantine or \
                                  there is a bug in the code.",
                                batch_ref, reporting_node
                            );
                            continue;
                        }

                        let batch_propose_delay = self.batch_propose_delay(batch_ref);
                        self.batch_missing_votes[node_id].push(batch_propose_delay);
                    },
                    PenaltyTrackerReportEntry::None => {
                        // No action.
                    },
                }
            }
        }
    }

    fn batch_propose_delay(&self, batch_ref: BatchRef) -> Duration {
        self.block_issue_time - self.batch_receive_time[&batch_ref]
    }

    fn compute_new_penalties(&self) -> Vec<Duration> {
        assert!(self.config.enable_penalty_system);

        let mut updated_penalties = vec![Duration::ZERO; self.config.n_nodes];

        for node_id in 0..self.config.n_nodes {
            if self.batch_missing_votes[node_id].len() >= self.config.f + 1 {
                // If the node had missing batches reported by >f nodes, increase its penalty
                // so that, if all message delays stay the same, only f nodes will be missing
                // batches in the next round.

                let delay = self.batch_missing_votes[node_id]
                    .iter()
                    .sorted_by_key(|x| std::cmp::Reverse(*x))
                    // Skip f largest (potentially malicious) delays and take the (f+1)-st largest.
                    .nth(self.config.f)
                    .copied()
                    .unwrap();

                assert!(delay >= self.penalty[node_id]);
                // Always at least double the penalty when increasing it.
                // Also, to speed up convergence, start from at least
                // `self.config.extra_wait_before_qc_vote`.
                updated_penalties[node_id] = max(
                    max(
                        self.config.extra_wait_before_qc_vote,
                        self.penalty[node_id] * 2,
                    ),
                    delay + self.config.batch_interval,
                );
            } else if self.advantage_votes[node_id].len() >= self.config.n_nodes - self.config.f {
                // If the node had no missing batches reported by >f nodes, decrease its penalty
                // so that, if all message delays stay the same, at least n-f nodes will still
                // not be missing any batches in the next round.

                let mut advantage = self.advantage_votes[node_id]
                    .iter()
                    .sorted_by_key(|x| std::cmp::Reverse(*x))
                    // Take the (n-f)-th largest advantage.
                    .nth(self.config.n_nodes - self.config.f - 1)
                    .copied()
                    .unwrap();

                advantage -= min(advantage, self.config.batch_interval);
                let current_penalty = self.penalty[node_id];

                // Always at most halve the penalty when decreasing it.
                updated_penalties[node_id] = current_penalty - min(advantage, current_penalty / 2);
            }
        }

        updated_penalties
    }

    pub fn on_new_batch(&mut self, batch_ref: BatchRef) {
        // This should be executed even when the penalty system is turned off.
        self.batch_receive_time.insert(batch_ref, Instant::now());
    }

    pub fn prepare_new_block(
        &mut self,
        round: Round,
        batches: &BTreeSet<BatchRef>,
    ) -> Vec<BatchRef> {
        if !self.config.enable_penalty_system {
            return batches
                .iter()
                .copied()
                .sorted_by_key(|batch_ref| self.batch_receive_time[batch_ref])
                .collect();
        }

        // `compute_new_penalties` must be called before any parts of the state are updated.
        let new_penalties = self.compute_new_penalties();

        let now = Instant::now();

        let batches_to_propose: Vec<BatchRef> = batches
            .into_iter()
            .copied()
            .map(|batch_ref| {
                let receive_time = self.batch_receive_time[&batch_ref];
                let safe_propose_time = receive_time + new_penalties[batch_ref.node];
                (safe_propose_time, batch_ref)
            })
            .sorted()
            .filter(|&(safe_propose_time, _)| safe_propose_time <= now)
            .map(|(_, batch_ref)| batch_ref)
            .collect();

        self.last_round_this_node_was_leader = round;
        self.proposed_batches = batches_to_propose.clone();
        self.block_issue_time = now;
        self.received_reports.clear();
        for votes in &mut self.advantage_votes {
            votes.clear();
        }
        for votes in &mut self.batch_missing_votes {
            votes.clear();
        }
        self.penalty = new_penalties;

        batches_to_propose
    }
}
