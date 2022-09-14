// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use crate::prometheus::construct_query_with_extra_labels;
use again::RetryPolicy;
use anyhow::{anyhow, bail};
use once_cell::sync::Lazy;
use prometheus_http_query::response::Sample;
use prometheus_http_query::Client as PrometheusClient;
use serde::Serialize;
use std::collections::BTreeMap;
use std::time::Duration;

#[derive(Default, Clone, Debug)]
pub struct SystemMetrics {
    cpu_core_metrics: Vec<Sample>,
    memory_bytes_metrics: Vec<Sample>,
}

// This retry policy is used for important client calls necessary for setting
// up the test (e.g. account creation) and collecting its results (e.g. checking
// account sequence numbers). If these fail, the whole test fails. We do not use
// this for submitting transactions, as we have a way to handle when that fails.
// This retry policy means an operation will take 8 seconds at most.
static RETRY_POLICY: Lazy<RetryPolicy> = Lazy::new(|| {
    RetryPolicy::exponential(Duration::from_millis(125))
        .with_max_retries(6)
        .with_jitter(true)
});

impl SystemMetrics {
    pub fn new(cpu_metrics: Vec<Sample>, memory_metrics: Vec<Sample>) -> Self {
        Self {
            cpu_core_metrics: cpu_metrics,
            memory_bytes_metrics: memory_metrics,
        }
    }
}

#[derive(Default, Clone, Debug, Serialize)]
pub struct MetricsThreshold {
    max: usize,
    // % of the data point that can breach the max threshold
    max_breach_pct: usize,
}

impl MetricsThreshold {
    pub fn new(max: usize, max_breach_pct: usize) -> Self {
        Self {
            max,
            max_breach_pct,
        }
    }
}

#[derive(Default, Clone, Debug, Serialize)]
pub struct SystemMetricsThreshold {
    cpu_threshold: MetricsThreshold,
    memory_threshold: MetricsThreshold,
}

impl SystemMetricsThreshold {
    pub fn ensure_threshold(&self, metrics: &SystemMetrics) -> anyhow::Result<()> {
        ensure_metrics_threshold("cpu", &self.cpu_threshold, &metrics.cpu_core_metrics)?;
        ensure_metrics_threshold(
            "memory",
            &self.memory_threshold,
            &metrics.memory_bytes_metrics,
        )?;
        Ok(())
    }
    pub fn new(cpu_threshold: MetricsThreshold, memory_threshold: MetricsThreshold) -> Self {
        Self {
            cpu_threshold,
            memory_threshold,
        }
    }
}

fn ensure_metrics_threshold(
    metrics_name: &str,
    threshold: &MetricsThreshold,
    metrics: &Vec<Sample>,
) -> anyhow::Result<()> {
    if metrics.is_empty() {
        bail!("Empty metrics provided");
    }
    let breach_count = metrics
        .iter()
        .filter(|sample| sample.value() > threshold.max as f64)
        .count();
    let breach_pct = (breach_count * 100) / metrics.len();
    if breach_pct > threshold.max_breach_pct {
        bail!(
            "{:?} metric violated threshold of {:?}, max_breach_pct: {:?}, breach_pct: {:?} ",
            metrics_name,
            threshold.max,
            threshold.max_breach_pct,
            breach_pct
        );
    }
    Ok(())
}

async fn query_prometheus_range_metrics(
    query: &str,
    client: &PrometheusClient,
    start_time: i64,
    end_time: i64,
    internal_secs: f64,
    namespace: &str,
) -> anyhow::Result<Vec<Sample>> {
    RETRY_POLICY
        .retry(move || {
            get_prometheus_range_metrics(
                query,
                client,
                start_time,
                end_time,
                internal_secs,
                namespace,
            )
        })
        .await
        .map_err(|e| anyhow!("Failed to query prometheus for system metrics: {}", e))
}

async fn get_prometheus_range_metrics(
    query: &str,
    client: &PrometheusClient,
    start_time: i64,
    end_time: i64,
    internal_secs: f64,
    namespace: &str,
) -> anyhow::Result<Vec<Sample>> {
    let mut labels_map = BTreeMap::new();
    labels_map.insert("namespace".to_string(), namespace.to_string());
    let response = client
        .query_range(
            construct_query_with_extra_labels(query, labels_map),
            start_time,
            end_time,
            internal_secs,
            None,
        )
        .await?;
    Ok(response
        .as_range()
        .ok_or_else(|| anyhow!("Failed to get range from prometheus response"))?
        .first()
        .ok_or_else(|| anyhow!("Empty range vector returned from prometheus"))?
        .samples()
        .to_vec())
}

pub async fn query_prometheus_system_metrics(
    client: &PrometheusClient,
    start_time: i64,
    end_time: i64,
    internal_secs: f64,
    namespace: &str,
) -> anyhow::Result<SystemMetrics> {
    let cpu_query = r#"avg(rate(container_cpu_usage_seconds_total{container=~"validator"}[30s]))"#;
    let memory_query = r#"avg(container_memory_rss{container=~"validator"})"#;

    let cpu_samples = query_prometheus_range_metrics(
        cpu_query,
        client,
        start_time,
        end_time,
        internal_secs,
        namespace,
    )
    .await?;

    let memory_samples = query_prometheus_range_metrics(
        memory_query,
        client,
        start_time,
        end_time,
        internal_secs,
        namespace,
    )
    .await?;

    Ok(SystemMetrics::new(cpu_samples, memory_samples))
}

#[cfg(test)]
mod tests {

    use super::*;
    #[tokio::test]
    async fn test_empty_metrics_threshold() {
        let cpu_threshold = MetricsThreshold::new(10, 30);
        let memory_threshold = MetricsThreshold::new(100, 40);
        let threshold = SystemMetricsThreshold::new(cpu_threshold, memory_threshold);
        let metrics = SystemMetrics::new(vec![], vec![]);
        threshold.ensure_threshold(&metrics).unwrap_err();
    }
}
