// Copyright (c) Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{
    framework::NodeId,
    raikou::types::{Round, *},
};
use itertools::Itertools;
use rand::{seq::SliceRandom, thread_rng};
use serde::{Deserialize, Serialize};
use std::{
    cmp::{max, min},
    collections::{BTreeMap, BTreeSet},
    time::Duration,
};
use tokio::time::Instant;

trait Micros {
    fn as_micros_i32(&self) -> i32;

    fn from_micros_i32(micros: i32) -> Self;
}

impl Micros for Duration {
    fn as_micros_i32(&self) -> i32 {
        self.as_micros() as i32
    }

    fn from_micros_i32(micros: i32) -> Self {
        assert!(micros >= 0);
        Duration::from_micros(micros as u64)
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Serialize, Deserialize)]
/// Penalty tracker report for the optimistically proposed batches from a single node for
/// a single node. See the description of `PenaltyTrackerReport` for details.
pub enum PenaltyTrackerReportEntry {
    Delay(usize, i32),

    Missing(usize, i32),

    None,
}

/// Type alias for readability.
type NodeIdMap<T> = Vec<T>;

/// `Delay(k, x)` on position `i` in the report means that the sender of the report had all the
/// optimistically proposed batches issued by node `i` `x` seconds after (if `x` is positive)
/// or `-x` seconds before (if `x` is negative) the sender of the report received the block
/// from the leader and the batch that was on the `k`th position in the block was received last
/// among all optimistically proposed batches issued by node `i`.
///
/// `Missing(k, x)` on position `i` in the report means that the sender of the report was missing
/// the batch that was on `k`th position in the block issued by node `i` has not been yet received
/// when the report was prepared, `x` seconds after the sender of the report received the block
/// from the leader. Of all such batches, the smallest `k` is reported as it was supposedly
/// received the earliest by the leader.
///
/// `None` on position `i` in the report  means that there were no optimistically proposed batches
/// issued by node `i` in the block.
pub type PenaltyTrackerReports = NodeIdMap<PenaltyTrackerReportEntry>;

#[derive(Clone)]
pub struct Config {
    pub n_nodes: usize,
    pub f: usize,
    pub enable: bool,
    pub batch_expiration_time: Duration,
}

pub struct PenaltyTracker {
    node_id: NodeId,
    config: Config,
    detailed_logging: bool,

    batch_receive_time: BTreeMap<BatchHash, Instant>,
    penalties: NodeIdMap<Duration>,

    // The variables below are relative to the last round this node was leader.
    last_round_this_node_was_leader: Round,
    block_issue_time: Instant,
    proposed_batches: Vec<BatchInfo>,
    batch_authors: BTreeSet<NodeId>,
    reports: BTreeMap<NodeId, BTreeMap<NodeId, i32>>,

    last_selected_quorum: Vec<NodeId>,

    // For metrics
    block_prepare_time: BTreeMap<Round, Instant>,
    block_prepare_penalties: BTreeMap<Round, NodeIdMap<Duration>>,
}

impl PenaltyTracker {
    pub fn new(id: NodeId, config: Config, detailed_logging: bool) -> Self {
        let n_nodes = config.n_nodes;

        Self {
            node_id: id,
            config,
            detailed_logging,
            batch_receive_time: Default::default(),
            penalties: vec![Duration::ZERO; n_nodes],
            last_round_this_node_was_leader: -1,
            proposed_batches: vec![],
            batch_authors: Default::default(),
            block_issue_time: Instant::now(),
            reports: Default::default(),
            last_selected_quorum: vec![],
            block_prepare_time: Default::default(),
            block_prepare_penalties: Default::default(),
        }
    }

    pub fn block_prepare_time(&self, round: Round) -> Instant {
        self.block_prepare_time[&round]
    }

    pub fn block_prepare_penalty(&self, round: Round, node_id: NodeId) -> Duration {
        if self.config.enable {
            self.block_prepare_penalties[&round][node_id]
        } else {
            Duration::ZERO
        }
    }

    pub fn batch_receive_time(&self, digest: BatchHash) -> Instant {
        self.batch_receive_time[&digest]
    }

    pub fn prepare_reports(
        &self,
        payload: Payload,
        block_receive_time: Instant,
    ) -> PenaltyTrackerReports {
        assert!(self.config.enable);

        let now = Instant::now();
        assert!(now >= block_receive_time);

        let mut delays = vec![(0, i32::MIN); self.config.n_nodes];
        let mut missing = vec![None; self.config.n_nodes];
        let mut has_batches = vec![false; self.config.n_nodes];

        for (batch_num, batch_info) in payload.sub_blocks().flatten().enumerate() {
            has_batches[batch_info.author] = true;

            if let Some(batch_receive_time) =
                self.batch_receive_time.get(&batch_info.digest).copied()
            {
                let batch_delay = if batch_receive_time > block_receive_time {
                    (batch_receive_time - block_receive_time).as_micros_i32()
                } else {
                    -(block_receive_time - batch_receive_time).as_micros_i32()
                };

                if batch_delay > delays[batch_info.author].1 {
                    delays[batch_info.author] = (batch_num, batch_delay);
                }
            } else if missing[batch_info.author].is_none() {
                missing[batch_info.author] = Some(batch_num);
            }
        }

        (0..self.config.n_nodes)
            .map(|node_id| {
                if !has_batches[node_id] {
                    PenaltyTrackerReportEntry::None
                } else if let Some(batch_num) = missing[node_id] {
                    PenaltyTrackerReportEntry::Missing(
                        batch_num,
                        (now - block_receive_time).as_micros_i32(),
                    )
                } else {
                    let (batch_num, delay) = delays[node_id];
                    assert_ne!(delay, i32::MIN);
                    PenaltyTrackerReportEntry::Delay(batch_num, delay)
                }
            })
            .collect()
    }

    pub fn register_reports(
        &mut self,
        round: Round,
        reporter: NodeId,
        reports: PenaltyTrackerReports,
    ) {
        assert!(self.config.enable);

        if round != self.last_round_this_node_was_leader || self.reports.contains_key(&reporter) {
            return;
        }

        let mut processed_reports = BTreeMap::new();

        for (node_id, report) in reports.into_iter().enumerate() {
            match report {
                PenaltyTrackerReportEntry::Delay(batch_num, delay) => {
                    if self.proposed_batches[batch_num].author != node_id {
                        aptos_logger::warn!(
                            "Received invalid penalty tracker report from node {}",
                            reporter
                        );
                        return;
                    }

                    let propose_delay = self.batch_propose_delay(&self.proposed_batches[batch_num]);
                    assert!(propose_delay >= self.penalties[node_id]);

                    let extra_delay = propose_delay - self.penalties[node_id];
                    let adjusted_delay = delay + extra_delay.as_micros_i32();
                    processed_reports.insert(node_id, adjusted_delay);
                },
                PenaltyTrackerReportEntry::Missing(batch_num, delay) => {
                    if self.proposed_batches[batch_num].author != node_id {
                        aptos_logger::warn!(
                            "Received invalid penalty tracker report from node {}",
                            reporter
                        );
                        return;
                    }

                    // For now, missing votes are treated the same as Delay votes.
                    let propose_delay = self.batch_propose_delay(&self.proposed_batches[batch_num]);
                    assert!(propose_delay >= self.penalties[node_id]);

                    let extra_delay = propose_delay - self.penalties[node_id];
                    let adjusted_delay = delay + extra_delay.as_micros_i32();
                    processed_reports.insert(node_id, adjusted_delay);
                },
                PenaltyTrackerReportEntry::None => {
                    if self.batch_authors.contains(&node_id) {
                        aptos_logger::warn!(
                            "Received invalid penalty tracker report from node {}",
                            reporter
                        );
                        return;
                    }
                },
            }
        }

        self.reports.insert(reporter, processed_reports);
    }

    fn batch_propose_delay(&self, batch_info: &BatchInfo) -> Duration {
        self.block_issue_time - self.batch_receive_time[&batch_info.digest]
    }

    fn compute_new_penalties_for_quorum(&self, quorum: &Vec<NodeId>) -> Vec<Duration> {
        // The new penalties are computed in such a way that, if the next time this node is
        // the leader all the message delays stay the same, the nodes in `quorum` will have
        // all the batches optimistically proposed by the leader.

        let mut updated_penalties = vec![Duration::ZERO; self.config.n_nodes];

        for node_id in 0..self.config.n_nodes {
            if self.batch_authors.contains(&node_id) {
                let max_reported_delay_in_a_quorum = quorum
                    .iter()
                    .copied()
                    .map(|reporter| self.reports[&reporter][&node_id])
                    .max_by(|x, y| x.partial_cmp(y).unwrap())
                    .unwrap();

                if max_reported_delay_in_a_quorum > 0 {
                    // Increase penalty.
                    updated_penalties[node_id] = self.penalties[node_id]
                        + Duration::from_micros_i32(max_reported_delay_in_a_quorum);
                } else {
                    // Decrease penalty.
                    // Always at most halve the penalty when decreasing it.
                    updated_penalties[node_id] = self.penalties[node_id]
                        - min(
                            self.penalties[node_id] / 2,
                            Duration::from_micros_i32(-max_reported_delay_in_a_quorum),
                        );
                }
            } else {
                // TODO: What to do with nodes that have no optimistically proposed batches?
                //       Most likely, it happens because they already have too large penalty
                //       and their transactions go through the slow path.
                //       At some point we should give them a chance to rehabilitate themselves.
                // TODO: Idea: include their batch hashes in the block, but do not actually
                //       commit them, just to collect reports.
                updated_penalties[node_id] = self.penalties[node_id];
            }
        }

        updated_penalties
    }

    fn random_quorum(&self) -> Vec<NodeId> {
        let reporting_nodes = self.reports.keys().copied().collect_vec();
        reporting_nodes
            .choose_multiple(&mut thread_rng(), self.config.n_nodes - self.config.f)
            .copied()
            .collect_vec()
    }

    fn smallest_sum_quorum(&self) -> Vec<NodeId> {
        // For each node that sent a report, compute the sum of reported delays.
        let mut delay_sums = self
            .reports
            .iter()
            .map(|(reporter, reports)| {
                let delay_sum = reports.iter().map(|(_, delay)| *delay as f64).sum::<f64>();

                (*reporter, delay_sum)
            })
            .collect_vec();

        // Sort by the sum of reported delays in the ascending order.
        delay_sums.sort_by(|(_, x), (_, y)| x.partial_cmp(y).unwrap());

        // Select a quorum of nodes with the smallest sums of delays.
        delay_sums
            .into_iter()
            // Keep the n-f reports with the smallest sum of delays.
            .take(self.config.n_nodes - self.config.f)
            // Get the IDs of the reporting nodes.
            .map(|(node_id, _)| node_id)
            .collect_vec()
    }

    fn compute_new_penalties(&mut self) -> Vec<Duration> {
        assert!(self.config.enable);

        if self.last_round_this_node_was_leader == -1 {
            // This node has not been a leader yet. No information to compute penalties.
            return self.penalties.clone();
        }

        if self.reports.len() < self.config.n_nodes - self.config.f {
            // If there are not enough reports, the network must be in an asynchronous period.
            // Do not change the penalties.
            // TODO: What's the best strategy fo this case?
            aptos_logger::warn!(
                "Not enough reports to compute new penalties after round {} ({} / {}). \
                   Either the network is asynchronous or the penalty tracker is misconfigured.",
                self.last_round_this_node_was_leader,
                self.reports.len(),
                self.config.n_nodes - self.config.f
            );

            return self.penalties.clone();
        }

        let mut candidates = vec![];

        if !self.last_selected_quorum.is_empty() {
            if self
                .last_selected_quorum
                .iter()
                .all(|p| self.reports.contains_key(&p))
            {
                candidates.push((
                    self.compute_new_penalties_for_quorum(&self.last_selected_quorum),
                    self.last_selected_quorum.clone(),
                    "last selected quorum",
                ));
            }
        }

        let random_quorum = self.random_quorum();
        candidates.push((
            self.compute_new_penalties_for_quorum(&random_quorum),
            random_quorum.clone(),
            "random quorum",
        ));

        let smallest_sum_quorum = self.smallest_sum_quorum();
        candidates.push((
            self.compute_new_penalties_for_quorum(&smallest_sum_quorum),
            smallest_sum_quorum.clone(),
            "smallest sum quorum",
        ));

        let (new_penalties, quorum, quorum_name) = candidates
            .into_iter()
            .min_by_key(|(penalties, _, _)| penalties.iter().sum::<Duration>())
            .unwrap();

        self.log_detail(format!(
            "Selected quorum after round {}: {}",
            self.last_round_this_node_was_leader, quorum_name,
        ));

        self.last_selected_quorum = quorum;
        new_penalties
    }

    pub fn on_new_batch(&mut self, digest: BatchHash) {
        // This should be executed even when the penalty system is turned off.
        assert!(!self.batch_receive_time.contains_key(&digest));
        self.batch_receive_time.insert(digest, Instant::now());
    }

    fn split_to_sub_blocks(&self, batches: Vec<BatchInfo>) -> [Vec<BatchInfo>; N_SUB_BLOCKS] {
        let mut sub_blocks: [Vec<BatchInfo>; N_SUB_BLOCKS] = Default::default();

        let smaller_sub_block_size = batches.len() / N_SUB_BLOCKS;
        let larger_sub_block_size = smaller_sub_block_size + 1;
        let n_larger_sub_blocks = batches.len() % N_SUB_BLOCKS;

        let mut iter = batches.into_iter();

        for i in 0..N_SUB_BLOCKS {
            let sub_block_size = if i < n_larger_sub_blocks {
                larger_sub_block_size
            } else {
                smaller_sub_block_size
            };

            sub_blocks[i].reserve_exact(sub_block_size);
            for _ in 0..sub_block_size {
                sub_blocks[i].push(iter.next().unwrap());
            }
        }

        sub_blocks
    }

    pub fn prepare_new_block(
        &mut self,
        round: Round,
        batches: Vec<BatchInfo>,
    ) -> [Vec<BatchInfo>; N_SUB_BLOCKS] {
        if !self.config.enable {
            self.block_prepare_time.insert(round, Instant::now());
            let now = Instant::now();

            let batches = batches
                .into_iter()
                .filter(|batch_info| {
                    (now - self.batch_receive_time[&batch_info.digest])
                        < self.config.batch_expiration_time
                })
                .sorted_by_key(|batch_info| self.batch_receive_time[&batch_info.digest])
                .collect_vec();

            return self.split_to_sub_blocks(batches);
        }

        // `compute_new_penalties` must be called before any parts of the state are updated.
        let new_penalties = self.compute_new_penalties();

        if self.last_round_this_node_was_leader != -1 {
            self.log_detail(format!(
                "New penalties after round {}: {:?}",
                self.last_round_this_node_was_leader, new_penalties
            ));
        }

        let now = Instant::now();
        self.block_prepare_time.insert(round, now);

        self.block_prepare_penalties
            .insert(round, new_penalties.clone());
        let batches_to_propose: Vec<BatchInfo> = batches
            .into_iter()
            .filter(|batch_info| {
                (now - self.batch_receive_time[&batch_info.digest])
                    < self.config.batch_expiration_time
            })
            .map(|batch_info| {
                let receive_time = self.batch_receive_time[&batch_info.digest];
                let safe_propose_time = receive_time + new_penalties[batch_info.author];
                (safe_propose_time, batch_info)
            })
            .filter(|&(safe_propose_time, _)| safe_propose_time <= now)
            .sorted_by_key(|(safe_propose_time, _)| *safe_propose_time)
            .map(|(_, batch_info)| batch_info)
            .collect_vec();

        self.penalties = new_penalties;

        self.last_round_this_node_was_leader = round;
        self.block_issue_time = now;
        self.proposed_batches = batches_to_propose.clone();
        self.batch_authors = batches_to_propose
            .iter()
            .map(|batch_info| batch_info.author)
            .collect();
        self.reports.clear();

        self.split_to_sub_blocks(batches_to_propose)
    }

    fn log_info(&self, msg: String) {
        aptos_logger::info!("Node {}: Penalty tracker: {}", self.node_id, msg);
    }

    fn log_detail(&self, msg: String) {
        if self.detailed_logging {
            self.log_info(msg);
        }
    }
}
