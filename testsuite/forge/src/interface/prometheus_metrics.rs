// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::Swarm;
use anyhow::Error;
use prometheus_http_query::response::Sample;
use std::{collections::BTreeMap, fmt, sync::Arc};
use tokio::sync::RwLock;

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

pub async fn fetch_fullnode_failures(
    swarm: Arc<tokio::sync::RwLock<Box<dyn Swarm>>>,
) -> anyhow::Result<i64> {
    let consensus_observer_failure_query =
        r#"consensus_observer_state_sync_fallback_counter{role=~"validator|fullnode"}"#;
    fetch_metric_counter(swarm, consensus_observer_failure_query).await
}

async fn fetch_metric_counter(
    swarm: Arc<RwLock<Box<dyn Swarm>>>,
    metric_query: &str,
) -> Result<i64, Error> {
    let result = swarm
        .read()
        .await
        .query_metrics(metric_query, None, None)
        .await?;
    let samples = result.as_instant().unwrap_or(&[]);

    Ok(samples
        .iter()
        .map(|s| s.sample().value().round() as i64)
        .max()
        .unwrap_or(0))
}

pub async fn fetch_validator_error_metrics(
    swarm: Arc<tokio::sync::RwLock<Box<dyn Swarm>>>,
) -> anyhow::Result<i64> {
    let error_query = r#"aptos_error_log_count{role=~"validator"}"#;
    fetch_metric_counter(swarm, error_query).await
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
    MempoolToBlockCreation,
    ConsensusProposalToOrdered,
    ConsensusOrderedToCommit,
    ConsensusProposalToCommit,
    // each of the indexer grpc steps in order
    IndexerFullnodeProcessedBatch,
    IndexerCacheWorkerProcessedBatch,
    IndexerDataServiceAllChunksSent,
    // these two mesasure the same latency, but the metrics are different for the SDK
    IndexerProcessorLatency,
    IndexerProcessorSdkLatency,
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
    let consensus_proposal_to_ordered_query = r#"quantile(0.67, rate(aptos_consensus_block_tracing_sum{role=~"validator", stage="ordered"}[1m]) / rate(aptos_consensus_block_tracing_count{role=~"validator", stage="ordered"}[1m]))"#;
    let consensus_proposal_to_commit_query = r#"quantile(0.67, rate(aptos_consensus_block_tracing_sum{role=~"validator", stage="committed"}[1m]) / rate(aptos_consensus_block_tracing_count{role=~"validator", stage="committed"}[1m]))"#;

    let mempool_to_block_creation_query = r#"sum(
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

    let mempool_to_block_creation_samples = swarm
        .query_range_metrics(
            mempool_to_block_creation_query,
            start_time_adjusted as i64,
            end_time as i64,
            None,
        )
        .await?;

    let mut samples = BTreeMap::new();
    samples.insert(
        LatencyBreakdownSlice::MempoolToBlockCreation,
        MetricSamples::new(mempool_to_block_creation_samples),
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

    if swarm.has_indexer() {
        // These counters are defined in ecosystem/indexer-grpc/indexer-grpc-utils/src/counters.rs
        let indexer_fullnode_processed_batch_query =
            r#"max(indexer_grpc_duration_in_secs{step="4", service_type="indexer_fullnode"})"#;
        let indexer_cache_worker_processed_batch_query =
            r#"max(indexer_grpc_duration_in_secs{step="4", service_type="cache_worker"})"#;
        let indexer_data_service_all_chunks_sent_query =
            r#"max(indexer_grpc_duration_in_secs{step="4", service_type="data_service"})"#;

        // These are processor latencies for both original core processors and those written with the processor SDK: https://github.com/aptos-labs/aptos-indexer-processor-sdk
        // Note the use of empty {}, where additional test-specific labels will be added by Forge
        let indexer_processor_latency_query =
            r#"max(indexer_processor_data_processed_latency_in_secs{})"#;
        let indexer_sdk_processor_latency_query =
            "max(aptos_procsdk_step__processed_transaction_latency_secs{})";

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

        let indexer_processor_latency_samples = swarm
            .query_range_metrics(
                indexer_processor_latency_query,
                start_time as i64,
                end_time as i64,
                None,
            )
            .await?;

        let indexer_processor_sdk_latency_samples = swarm
            .query_range_metrics(
                indexer_sdk_processor_latency_query,
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
        samples.insert(
            LatencyBreakdownSlice::IndexerProcessorLatency,
            MetricSamples::new(indexer_processor_latency_samples),
        );
        samples.insert(
            LatencyBreakdownSlice::IndexerProcessorSdkLatency,
            MetricSamples::new(indexer_processor_sdk_latency_samples),
        );
    }
    Ok(LatencyBreakdown::new(samples))
}
