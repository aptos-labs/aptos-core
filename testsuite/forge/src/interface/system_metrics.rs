// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use crate::prometheus::construct_query_with_extra_labels;
use anyhow::bail;
use prometheus_http_query::response::Sample;
use prometheus_http_query::Client as PrometheusClient;
use std::collections::BTreeMap;

#[derive(Default, Clone, Debug)]
pub struct SystemMetrics {
    cpu_core_metrics: Vec<Sample>,
    memory_bytes_metrics: Vec<Sample>,
}

impl SystemMetrics {
    pub fn new(cpu_metrics: Vec<Sample>, memory_metrics: Vec<Sample>) -> Self {
        Self {
            cpu_core_metrics: cpu_metrics,
            memory_bytes_metrics: memory_metrics,
        }
    }
}

#[derive(Default, Clone, Debug)]
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

#[derive(Default, Clone, Debug)]
pub struct SystemMetricsThreshold {
    cpu_threshold: MetricsThreshold,
    memory_threshold: MetricsThreshold,
}

impl SystemMetricsThreshold {
    pub fn ensure_threshold(&self, metrics: &SystemMetrics) -> anyhow::Result<()> {
        ensure_metrics_threshold(&self.cpu_threshold, &metrics.cpu_core_metrics)?;
        ensure_metrics_threshold(&self.memory_threshold, &metrics.memory_bytes_metrics)?;
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
            "Metrics violated threshold, max_breach_pct {:?}, breach_pct{:?} ",
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
    Ok(response.as_range().unwrap()[0].samples().to_vec())
}

pub async fn query_prometheus_system_metrics(
    client: &PrometheusClient,
    start_time: i64,
    end_time: i64,
    internal_secs: f64,
    namespace: &str,
) -> anyhow::Result<SystemMetrics> {
    let cpu_query = r#"avg(rate(container_cpu_usage_seconds_total{container=~"validator"}[30s]))"#;
    let memory_query = r#"avg(container_memory_working_set_bytes{container=~"validator"})"#;

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
    use crate::prometheus::get_prometheus_client;
    use prometheus_http_query::Error as PrometheusError;

    use super::*;
    #[tokio::test]
    async fn test_empty_metrics_threshold() {
        let cpu_threshold = MetricsThreshold::new(10, 30);
        let memory_threshold = MetricsThreshold::new(100, 40);
        let threshold = SystemMetricsThreshold::new(cpu_threshold, memory_threshold);
        let metrics = SystemMetrics::new(vec![], vec![]);
        threshold.ensure_threshold(&metrics).unwrap_err();
    }

    #[tokio::test]
    async fn test_query_prometheus() {
        let client = get_prometheus_client().unwrap();
        let query = r#"rate(container_cpu_usage_seconds_total{pod=~".*validator.*", container="validator"}[1m])"#;
        let response = client.query(query, None, None).await;
        match response {
            Ok(pres) => println!("{:?}", pres),
            Err(PrometheusError::Client(e)) => {
                println!("Skipping test. Failed to create prometheus client: {}", e);
                return;
            }
            Err(e) => panic!("Expected PromqlResult: {}", e),
        }

        let start_timestamp = 1660453807;
        let end_timestamp: i64 = 1660454554;
        let namespace = "forge-pr-2918";

        let response = query_prometheus_system_metrics(
            &client,
            start_timestamp,
            end_timestamp,
            30.0,
            namespace,
        )
        .await;

        match response {
            Ok(metrics) => {
                println!("{:?}", metrics);
                let metrics_threshold = SystemMetricsThreshold::new(
                    MetricsThreshold::new(12, 30),
                    MetricsThreshold::new(3 * 1024 * 1024 * 1024, 40),
                );
                let result = metrics_threshold.ensure_threshold(&metrics);
                if let Err(e) = result {
                    panic!("Failed metrics threshold check {:?}", e);
                }
            }
            _ => panic!("Expected PromqlResult"),
        }
    }
}
