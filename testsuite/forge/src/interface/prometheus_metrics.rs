// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::Swarm;
use prometheus_http_query::response::Sample;
use std::{collections::BTreeMap, fmt, sync::Arc};

#[derive(Clone)]
pub struct MetricSamples(Vec<Sample>);

impl MetricSamples {
    pub fn new(samples: Vec<Sample>) -> Self {
        Self(samples)
    }

    pub fn max_sample(&self) -> f64 {
        self.0
            .iter()
            .map(|s| s.value())
            .max_by(|a, b| a.partial_cmp(b).unwrap())
            .unwrap_or_default()
    }

    pub fn avg_sample(&self) -> f64 {
        self.0.iter().map(|s| s.value()).sum::<f64>() / self.0.len() as f64
    }

    pub fn get(&self) -> &Vec<Sample> {
        &self.0
    }
}

impl fmt::Debug for MetricSamples {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "{:?}",
            self.0
                .iter()
                .map(|s| (s.value(), s.timestamp()))
                .collect::<Vec<_>>()
        )
    }
}

#[derive(Clone, Debug)]
pub struct SystemMetrics {
    pub cpu_core_metrics: MetricSamples,
    pub memory_bytes_metrics: MetricSamples,
}

impl SystemMetrics {
    pub fn new(cpu_metrics: Vec<Sample>, memory_metrics: Vec<Sample>) -> Self {
        Self {
            cpu_core_metrics: MetricSamples::new(cpu_metrics),
            memory_bytes_metrics: MetricSamples::new(memory_metrics),
        }
    }
}

pub async fn fetch_error_metrics(
    swarm: Arc<tokio::sync::RwLock<Box<dyn Swarm>>>,
) -> anyhow::Result<i64> {
    let error_query = r#"aptos_error_log_count{role=~"validator"}"#;

    let result = swarm
        .read()
        .await
        .query_metrics(error_query, None, None)
        .await?;
    let error_samples = result.as_instant().unwrap_or(&[]);

    Ok(error_samples
        .iter()
        .map(|s| s.sample().value().round() as i64)
        .max()
        .unwrap_or(0))
}

pub async fn fetch_system_metrics(
    swarm: Arc<tokio::sync::RwLock<Box<dyn Swarm>>>,
    start_time: i64,
    end_time: i64,
) -> anyhow::Result<SystemMetrics> {
    // CPU oscilates, so aggregate over larger period (2m) to avoid noise.
    let cpu_query = r#"avg(rate(container_cpu_usage_seconds_total{container=~"validator"}[2m]))"#;
    let memory_query = r#"avg(container_memory_rss{container=~"validator"})"#;

    let swarm = swarm.read().await;
    let cpu_samples = swarm
        .query_range_metrics(cpu_query, start_time, end_time, None)
        .await?;

    let memory_samples = swarm
        .query_range_metrics(memory_query, start_time, end_time, None)
        .await?;

    Ok(SystemMetrics::new(cpu_samples, memory_samples))
}

#[derive(Clone, Debug, Eq, Hash, PartialEq, PartialOrd, Ord)]
pub enum LatencyBreakdownSlice {
    QsBatchToPos,
    QsPosToProposal,
    ConsensusProposalToOrdered,
    ConsensusOrderedToCommit,
    ConsensusProposalToCommit,
}

#[derive(Clone, Debug)]
pub struct LatencyBreakdown(BTreeMap<LatencyBreakdownSlice, MetricSamples>);

impl LatencyBreakdown {
    pub fn new(latency: BTreeMap<LatencyBreakdownSlice, MetricSamples>) -> Self {
        Self(latency)
    }

    pub fn keys(&self) -> Vec<LatencyBreakdownSlice> {
        self.0.keys().cloned().collect()
    }

    pub fn get_samples(&self, slice: &LatencyBreakdownSlice) -> &MetricSamples {
        self.0
            .get(slice)
            .unwrap_or_else(|| panic!("Missing latency breakdown for {:?}", slice))
    }
}

pub async fn fetch_latency_breakdown(
    swarm: Arc<tokio::sync::RwLock<Box<(dyn Swarm)>>>,
    start_time: u64,
    end_time: u64,
) -> anyhow::Result<LatencyBreakdown> {
    // Averaging over 1m, and skipping data points at the start that would take averages outside of the interval.
    let start_time_adjusted = start_time + 60;
    let consensus_proposal_to_ordered_query = r#"quantile(0.67, rate(aptos_consensus_block_tracing_sum{role=~"validator", stage="ordered"}[1m]) / rate(aptos_consensus_block_tracing_count{role=~"validator", stage="ordered"}[1m]))"#;
    let consensus_proposal_to_commit_query = r#"quantile(0.67, rate(aptos_consensus_block_tracing_sum{role=~"validator", stage="committed"}[1m]) / rate(aptos_consensus_block_tracing_count{role=~"validator", stage="committed"}[1m]))"#;

    let qs_batch_to_pos_query = r#"sum(rate(quorum_store_batch_to_PoS_duration_sum{role=~"validator"}[1m])) / sum(rate(quorum_store_batch_to_PoS_duration_count{role=~"validator"}[1m]))"#;
    let qs_pos_to_proposal_query = r#"sum(rate(quorum_store_pos_to_pull_sum{role=~"validator"}[1m])) / sum(rate(quorum_store_pos_to_pull_count{role=~"validator"}[1m]))"#;

    let swarm = swarm.read().await;
    let consensus_proposal_to_ordered_samples = swarm
        .query_range_metrics(
            consensus_proposal_to_ordered_query,
            start_time_adjusted as i64,
            end_time as i64,
            None,
        )
        .await?;

    let consensus_proposal_to_commit_samples = swarm
        .query_range_metrics(
            consensus_proposal_to_commit_query,
            start_time_adjusted as i64,
            end_time as i64,
            None,
        )
        .await?;

    let consensus_ordered_to_commit_samples = swarm
        .query_range_metrics(
            &format!(
                "{} - {}",
                consensus_proposal_to_commit_query, consensus_proposal_to_ordered_query
            ),
            start_time_adjusted as i64,
            end_time as i64,
            None,
        )
        .await?;

    let qs_batch_to_pos_samples = swarm
        .query_range_metrics(
            qs_batch_to_pos_query,
            start_time_adjusted as i64,
            end_time as i64,
            None,
        )
        .await?;

    let qs_pos_to_proposal_samples = swarm
        .query_range_metrics(
            qs_pos_to_proposal_query,
            start_time_adjusted as i64,
            end_time as i64,
            None,
        )
        .await?;

    let mut samples = BTreeMap::new();
    samples.insert(
        LatencyBreakdownSlice::QsBatchToPos,
        MetricSamples::new(qs_batch_to_pos_samples),
    );
    samples.insert(
        LatencyBreakdownSlice::QsPosToProposal,
        MetricSamples::new(qs_pos_to_proposal_samples),
    );
    samples.insert(
        LatencyBreakdownSlice::ConsensusProposalToOrdered,
        MetricSamples::new(consensus_proposal_to_ordered_samples),
    );
    samples.insert(
        LatencyBreakdownSlice::ConsensusOrderedToCommit,
        MetricSamples::new(consensus_ordered_to_commit_samples),
    );
    samples.insert(
        LatencyBreakdownSlice::ConsensusProposalToCommit,
        MetricSamples::new(consensus_proposal_to_commit_samples),
    );

    Ok(LatencyBreakdown::new(samples))
}
