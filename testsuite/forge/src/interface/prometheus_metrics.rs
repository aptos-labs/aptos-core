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

impl Default for MetricSamples {
    fn default() -> Self {
        Self::new(vec![])
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
    // each of the indexer grpc steps in order
    IndexerFullnodeProcessedBatch,
    IndexerCacheWorkerProcessedBatch,
    IndexerDataServiceAllChunksSent,
    // TODO: add processor insertion into DB latency
    InsertionToBlock,
    BlockCreationToCommit,
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

    pub fn get_samples(&self, slice: &LatencyBreakdownSlice) -> Option<&MetricSamples> {
        self.0.get(slice)
    }

    pub fn join(&self, other: &LatencyBreakdown) -> LatencyBreakdown {
        let mut ret_latency = self.0.clone();
        for (slice, samples) in other.0.iter() {
            ret_latency.insert(slice.clone(), samples.clone());
        }
        LatencyBreakdown::new(ret_latency)
    }
}

pub async fn fetch_latency_breakdown(
    swarm: Arc<tokio::sync::RwLock<Box<(dyn Swarm)>>>,
    start_time: u64,
    end_time: u64,
) -> anyhow::Result<LatencyBreakdown> {
    // Averaging over 1m, and skipping data points at the start that would take averages outside of the interval.
    let start_time_adjusted = start_time + 60;
    let consensus_proposal_to_commit_query = r#"quantile(0.67, rate(aptos_consensus_block_tracing_sum{role=~"validator", stage="committed"}[1m]) / rate(aptos_consensus_block_tracing_count{role=~"validator", stage="committed"}[1m]))"#;

    let insertion_to_block_query = r#"sum(
        rate(aptos_core_mempool_txn_commit_latency_sum{
            role=~"validator",
            stage="commit_accepted_block"
        }[1m])
    ) / sum(
        rate(aptos_core_mempool_txn_commit_latency_count{
            role=~"validator",
            stage="commit_accepted_block"
        }[1m])
    )"#;

    let block_creation_to_commit_query = r#"quantile(
        0.67,
        rate(
            aptos_consensus_block_tracing_sum{
                role=~"validator",
                stage="committed"
            }[1m]
        ) /
        rate(
            aptos_consensus_block_tracing_count{
                role=~"validator",
                stage="committed"
            }[1m]
        )
    ) < 1000000"#;

    let swarm = swarm.read().await;

    let consensus_proposal_to_commit_samples = swarm
        .query_range_metrics(
            consensus_proposal_to_commit_query,
            start_time_adjusted as i64,
            end_time as i64,
            None,
        )
        .await?;

    let insertion_to_block_samples = swarm
        .query_range_metrics(
            insertion_to_block_query,
            start_time_adjusted as i64,
            end_time as i64,
            None,
        )
        .await?;

    let block_creation_to_commit_samples = swarm
        .query_range_metrics(
            block_creation_to_commit_query,
            start_time_adjusted as i64,
            end_time as i64,
            None,
        )
        .await?;

    let mut samples = BTreeMap::new();
    samples.insert(
        LatencyBreakdownSlice::InsertionToBlock,
        MetricSamples::new(insertion_to_block_samples),
    );
    samples.insert(
        LatencyBreakdownSlice::BlockCreationToCommit,
        MetricSamples::new(block_creation_to_commit_samples),
    );
    samples.insert(
        LatencyBreakdownSlice::ConsensusProposalToCommit,
        MetricSamples::new(consensus_proposal_to_commit_samples),
    );

    if swarm.has_indexer() {
        // These counters are defined in ecosystem/indexer-grpc/indexer-grpc-utils/src/counters.rs
        let indexer_fullnode_processed_batch_query =
            r#"max(indexer_grpc_duration_in_secs{step="4", service_type="indexer_fullnode"})"#;
        let indexer_cache_worker_processed_batch_query =
            r#"max(indexer_grpc_duration_in_secs{step="4", service_type="cache_worker"})"#;
        let indexer_data_service_all_chunks_sent_query =
            r#"max(indexer_grpc_duration_in_secs{step="4", service_type="data_service"})"#;

        let indexer_fullnode_processed_batch_samples = swarm
            .query_range_metrics(
                indexer_fullnode_processed_batch_query,
                start_time as i64,
                end_time as i64,
                None,
            )
            .await?;

        let indexer_cache_worker_processed_batch_samples = swarm
            .query_range_metrics(
                indexer_cache_worker_processed_batch_query,
                start_time as i64,
                end_time as i64,
                None,
            )
            .await?;

        let indexer_data_service_all_chunks_sent_samples = swarm
            .query_range_metrics(
                indexer_data_service_all_chunks_sent_query,
                start_time as i64,
                end_time as i64,
                None,
            )
            .await?;

        samples.insert(
            LatencyBreakdownSlice::IndexerFullnodeProcessedBatch,
            MetricSamples::new(indexer_fullnode_processed_batch_samples),
        );
        samples.insert(
            LatencyBreakdownSlice::IndexerCacheWorkerProcessedBatch,
            MetricSamples::new(indexer_cache_worker_processed_batch_samples),
        );
        samples.insert(
            LatencyBreakdownSlice::IndexerDataServiceAllChunksSent,
            MetricSamples::new(indexer_data_service_all_chunks_sent_samples),
        );
    }
    Ok(LatencyBreakdown::new(samples))
}
