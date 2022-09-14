// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use anyhow::bail;
use aptos::node::analyze::fetch_metadata::FetchMetadata;
use aptos_sdk::types::PeerId;
use futures::future::join_all;
use std::time::Duration;
use transaction_emitter_lib::emitter::stats::DetailedTxnStats;
use transaction_emitter_lib::TransactionType;

use crate::system_metrics::SystemMetricsThreshold;
use crate::{Swarm, SwarmExt};

#[derive(Clone, Debug)]
pub struct StateProgressThreshold {
    pub max_no_progress_secs: f32,
    pub max_round_gap: u64,
}

#[derive(Clone, Debug)]
pub enum LatencyType {
    Average,
    P50,
    P90,
    P99,
}

pub struct LatencyThreshold {
    pub latency_thresholds: Vec<(u64, Option<TransactionType>, LatencyType)>,
}

#[derive(Default, Clone, Debug)]
pub struct SuccessCriteria {
    pub avg_tps: usize,
    latency_thresholds: Vec<(u64, Option<TransactionType>, LatencyType)>,
    check_no_restarts: bool,
    wait_for_all_nodes_to_catchup: Option<Duration>,
    // Maximum amount of CPU cores and memory bytes used by the nodes.
    system_metrics_threshold: Option<SystemMetricsThreshold>,
    chain_progress_check: Option<StateProgressThreshold>,
}

impl SuccessCriteria {
    pub fn new(tps: usize) -> Self {
        Self {
            avg_tps: tps,
            latency_thresholds: Vec::new(),
            check_no_restarts: false,
            wait_for_all_nodes_to_catchup: None,
            system_metrics_threshold: None,
            chain_progress_check: None,
        }
    }

    pub fn add_no_restarts(mut self) -> Self {
        self.check_no_restarts = true;
        self
    }

    pub fn add_wait_for_catchup_s(mut self, duration_secs: u64) -> Self {
        self.wait_for_all_nodes_to_catchup = Some(Duration::from_secs(duration_secs));
        self
    }

    pub fn add_system_metrics_threshold(mut self, threshold: SystemMetricsThreshold) -> Self {
        self.system_metrics_threshold = Some(threshold);
        self
    }

    pub fn add_chain_progress(mut self, threshold: StateProgressThreshold) -> Self {
        self.chain_progress_check = Some(threshold);
        self
    }

    pub fn add_latency_threshold(mut self, threshold: u64, latency_type: LatencyType) -> Self {
        self.latency_thresholds
            .push((threshold, None, latency_type));
        self
    }

    pub fn add_latency_threshold_for_type(
        mut self,
        threshold: u64,
        transaction_type: TransactionType,
        latency_type: LatencyType,
    ) -> Self {
        self.latency_thresholds
            .push((threshold, Some(transaction_type), latency_type));
        self
    }
}

pub struct SuccessCriteriaChecker {}

impl SuccessCriteriaChecker {
    pub async fn check_for_success(
        success_criteria: &SuccessCriteria,
        swarm: &mut dyn Swarm,
        stats: &DetailedTxnStats,
        window: Duration,
        start_time: i64,
        end_time: i64,
        start_version: u64,
        end_version: u64,
    ) -> anyhow::Result<()> {
        // TODO: Add more success criteria like expired transactions, CPU, memory usage etc
        let avg_tps = stats.total.committed / window.as_secs();
        if avg_tps < success_criteria.avg_tps as u64 {
            bail!(
                "TPS requirement failed. Average TPS {}, minimum TPS requirement {}",
                avg_tps,
                success_criteria.avg_tps,
            )
        }

        Self::check_latency(&success_criteria.latency_thresholds, stats, window)?;

        if let Some(timeout) = success_criteria.wait_for_all_nodes_to_catchup {
            swarm.wait_for_all_nodes_to_catchup_to_next(timeout).await?;
        }

        if success_criteria.check_no_restarts {
            swarm.ensure_no_validator_restart().await?;
            swarm.ensure_no_fullnode_restart().await?;
        }

        // TODO(skedia) Add end-to-end latency from counters after we have support for querying prometheus
        // latency (in addition to checking latency from txn-emitter)

        if let Some(system_metrics_threshold) = success_criteria.system_metrics_threshold.clone() {
            swarm
                .ensure_healthy_system_metrics(
                    start_time as i64,
                    end_time as i64,
                    system_metrics_threshold,
                )
                .await?;
        }

        if let Some(chain_progress_threshold) = &success_criteria.chain_progress_check {
            Self::check_chain_progress(swarm, chain_progress_threshold, start_version, end_version)
                .await?;
        }

        Ok(())
    }

    async fn check_chain_progress(
        swarm: &mut dyn Swarm,
        chain_progress_threshold: &StateProgressThreshold,
        start_version: u64,
        end_version: u64,
    ) -> anyhow::Result<()> {
        // Choose client with newest ledger version to fetch NewBlockEvents from:
        let clients = swarm.get_all_nodes_clients_with_names();
        let ledger_infos = join_all(
            clients
                .iter()
                .map(|(_name, client)| client.get_ledger_information()),
        )
        .await;
        let (_max_v, client) = ledger_infos
            .into_iter()
            .zip(clients.into_iter())
            .flat_map(|(resp, (_, client))| resp.map(|r| (r.into_inner().version, client)))
            .max_by_key(|(v, _c)| *v)
            .unwrap();

        let epochs = FetchMetadata::fetch_new_block_events(&client, None, None)
            .await
            .unwrap();

        let mut max_round_gap = 0;
        let mut max_round_gap_version = 0;
        let mut max_time_gap = 0;
        let mut max_time_gap_version = 0;

        let mut prev_block = None;
        let mut prev_ts = 0;
        let mut failed_from_nil = 0;
        let mut previous_epooch = 0;
        let mut previous_round = 0;
        for block in epochs
            .iter()
            .flat_map(|epoch| epoch.blocks.iter())
            .filter(|b| b.version > start_version && b.version < end_version)
        {
            let is_nil = block.event.proposer() == PeerId::ZERO;

            let current_gap = if previous_epooch == block.event.epoch() {
                block.event.round() - previous_round - 1
            } else {
                (if is_nil { 0 } else { 1 }) + block.event.failed_proposer_indices().len() as u64
            };

            if is_nil {
                failed_from_nil += current_gap;
            } else {
                if prev_ts > 0 {
                    let round_gap = current_gap + failed_from_nil;
                    let time_gap = block.event.proposed_time() as i64 - prev_ts as i64;

                    if time_gap < 0 {
                        println!(
                            "Clock went backwards? {}, {:?}, {:?}",
                            time_gap, block, prev_block
                        );
                    }

                    if round_gap > max_round_gap {
                        max_round_gap = round_gap;
                        max_round_gap_version = block.version;
                    }
                    if time_gap > max_time_gap as i64 {
                        max_time_gap = time_gap as u64;
                        max_time_gap_version = block.version;
                    }
                }

                failed_from_nil = 0;
                prev_ts = block.event.proposed_time();
                prev_block = Some(block);
            }

            previous_epooch = block.event.epoch();
            previous_round = block.event.round();
        }

        let max_time_gap_secs = Duration::from_micros(max_time_gap).as_secs_f32();
        if max_round_gap > chain_progress_threshold.max_round_gap
            || max_time_gap_secs > chain_progress_threshold.max_no_progress_secs as f32
        {
            bail!(
                "Failed chain progress check. Max round gap was {} [limit {}] at version {}. Max no progress secs was {} [limit {}] at version {}.",
                max_round_gap,
                chain_progress_threshold.max_round_gap,
                max_round_gap_version,
                max_time_gap_secs,
                chain_progress_threshold.max_no_progress_secs,
                max_time_gap_version,
            )
        } else {
            println!(
                "Passed progress check. Max round gap was {} [limit {}] at version {}. Max no progress secs was {} [limit {}] at version {}.",
                max_round_gap,
                chain_progress_threshold.max_round_gap,
                max_round_gap_version,
                max_time_gap_secs,
                chain_progress_threshold.max_no_progress_secs,
                max_time_gap_version,
            )
        }

        Ok(())
    }

    fn check_latency(
        latency_thresholds: &Vec<(u64, Option<TransactionType>, LatencyType)>,
        stats: &DetailedTxnStats,
        window: Duration,
    ) -> anyhow::Result<()> {
        let mut failures = Vec::new();
        for (latency_threshold, transaction_type, latency_type) in latency_thresholds {
            let type_stats = if let Some(ttype) = transaction_type {
                stats.per_type.get(ttype).unwrap()
            } else {
                &stats.total
            };

            let rate = type_stats.rate(window);
            let latency = match latency_type {
                LatencyType::Average => rate.latency,
                LatencyType::P50 => rate.p50_latency,
                LatencyType::P90 => rate.p90_latency,
                LatencyType::P99 => rate.p99_latency,
            };

            if latency > *latency_threshold {
                failures.push(
                    format!(
                        "{:?} latency for {} txns is {} and exceeds limit of {}",
                        latency_type,
                        transaction_type.map_or("all".to_string(), |t| format!("{:?}", t)),
                        latency,
                        latency_threshold
                    )
                    .to_string(),
                );
            }
        }
        if !failures.is_empty() {
            bail!("Failed latency check, for {:?}", failures);
        } else {
            Ok(())
        }
    }
}
