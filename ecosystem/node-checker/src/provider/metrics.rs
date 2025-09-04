// Copyright © Velor Foundation
// SPDX-License-Identifier: Apache-2.0

use super::{
    cache::OutputCache,
    traits::{Provider, ProviderError},
    CommonProviderConfig,
};
use crate::checker::CheckResult;
use anyhow::{anyhow, Context, Result};
use velor_logger::warn;
use async_trait::async_trait;
use prometheus_parse::{Scrape, Value};
use reqwest::Url;
use serde::{Deserialize, Serialize};
use std::{sync::Arc, time::Duration};

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct MetricsProviderConfig {
    #[serde(default, flatten)]
    pub common: CommonProviderConfig,
}

#[derive(Clone, Debug)]
pub struct MetricsProvider {
    pub config: MetricsProviderConfig,

    client: Arc<reqwest::Client>,

    /// This has both the path and the port already rolled into it.
    metrics_url: Url,

    // This output cache helps prevent the Provider from overfetching the data within
    // a short window of time. Downstream Checkers should be aware of this behaviour.
    output_cache: Arc<OutputCache<Scrape>>,
}

impl MetricsProvider {
    pub fn new(
        config: MetricsProviderConfig,
        client: Arc<reqwest::Client>,
        mut url: Url,
        metrics_port: u16,
    ) -> Self {
        url.set_path("metrics");
        url.set_port(Some(metrics_port)).unwrap();
        let output_cache = Arc::new(OutputCache::new(Duration::from_millis(
            config.common.cache_ttl_ms,
        )));
        Self {
            config,
            client,
            metrics_url: url,
            output_cache,
        }
    }

    pub async fn get_scrape(&self) -> Result<Scrape, ProviderError> {
        let response = self
            .client
            .get(self.metrics_url.clone())
            .send()
            .await
            .with_context(|| format!("Failed to get data from {}", self.metrics_url))
            .map_err(|e| ProviderError::RetryableEndpointError("/metrics", e))?;
        let body = response
            .text()
            .await
            .with_context(|| {
                format!(
                    "Failed to process response body from {} as text",
                    self.metrics_url
                )
            })
            .map_err(|e| ProviderError::ParseError(anyhow!(e)))?;
        Scrape::parse(body.lines().map(|l| Ok(l.to_string())))
            .with_context(|| {
                format!(
                    "Failed to parse response text from {} as a Prometheus scrape",
                    self.metrics_url
                )
            })
            .map_err(|e| ProviderError::ParseError(anyhow!(e)))
    }
}

#[async_trait]
impl Provider for MetricsProvider {
    type Output = Scrape;

    async fn provide(&self) -> Result<Self::Output, ProviderError> {
        self.output_cache.get(self.get_scrape()).await
    }

    fn explanation() -> &'static str {
        "The metrics port was not included in the request."
    }
}

pub struct Label<'a> {
    pub key: &'a str,
    pub value: &'a str,
}

/// This function searches through the given set of metrics and searches for
/// a metric with the given metric name. If no label was given, we return that
/// metric immediately. If a label was given, we search for a metric that
/// has that label.
fn get_metric_value(
    metrics: &Scrape,
    metric_name: &str,
    expected_label: Option<&Label>,
) -> Option<u64> {
    let mut discovered_sample = None;
    for sample in &metrics.samples {
        if sample.metric == metric_name {
            match &expected_label {
                Some(expected_label) => {
                    let label_value = sample.labels.get(expected_label.key);
                    if let Some(label_value) = label_value {
                        if label_value == expected_label.value {
                            discovered_sample = Some(sample);
                            break;
                        }
                    }
                },
                None => {
                    discovered_sample = Some(sample);
                    break;
                },
            }
        }
    }
    match discovered_sample {
        Some(sample) => match &sample.value {
            Value::Counter(v) => Some(v.round() as u64),
            Value::Gauge(v) => Some(v.round() as u64),
            Value::Untyped(v) => Some(v.round() as u64),
            wildcard => {
                warn!("Found unexpected metric type: {:?}", wildcard);
                None
            },
        },
        None => None,
    }
}

/// This is a convenience function that returns the metric value if it was
/// found, or a CheckResult if not.
pub fn get_metric<F>(
    metrics: &Scrape,
    metric_name: &str,
    expected_label: Option<&Label>,
    result_on_missing_fn: F,
) -> GetMetricResult
where
    F: FnOnce() -> CheckResult,
{
    let metric_value = get_metric_value(metrics, metric_name, expected_label);
    match metric_value {
        Some(v) => GetMetricResult::Present(v),
        None => GetMetricResult::Missing(result_on_missing_fn()),
    }
}

#[derive(Debug)]
pub enum GetMetricResult {
    Present(u64),
    Missing(CheckResult),
}

impl GetMetricResult {
    pub fn unwrap(self, check_results: &mut Vec<CheckResult>) -> Option<u64> {
        match self {
            GetMetricResult::Present(value) => Some(value),
            GetMetricResult::Missing(check_result) => {
                check_results.push(check_result);
                None
            },
        }
    }
}
