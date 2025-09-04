// Copyright © Velor Foundation
// SPDX-License-Identifier: Apache-2.0

use anyhow::Result;
use reqwest::Url;
use std::collections::HashMap;

/// A simple metric value enum (to represent different value types)
#[derive(Clone, Debug)]
pub enum MetricValue {
    I64(i64),
    F64(f64),
    I64orF64(i64, f64),
}

impl MetricValue {
    /// Convert the value to i64
    pub fn to_i64(&self) -> Result<i64> {
        match self {
            MetricValue::I64(v) => Ok(*v),
            MetricValue::F64(v) => Err(anyhow::format_err!("Value not i64: {}", v)),
            MetricValue::I64orF64(v, _) => Ok(*v),
        }
    }

    /// Convert the value to f64
    pub fn to_f64(&self) -> Result<f64> {
        match self {
            MetricValue::I64(v) => Err(anyhow::format_err!("Value not f64: {}", v)),
            MetricValue::F64(v) => Ok(*v),
            MetricValue::I64orF64(_, v) => Ok(*v),
        }
    }
}

/// A simple inspection client for querying metrics from a node
pub struct InspectionClient {
    client: reqwest::Client,
    url: Url,
}

impl InspectionClient {
    /// Create a new client from the given url
    pub fn new(url: Url) -> Self {
        let client = reqwest::Client::new();
        Self { client, url }
    }

    /// Get an i64 metric value from the node
    pub async fn get_node_metric_i64<S: AsRef<str>>(&self, metric: S) -> Result<Option<i64>> {
        let node_metrics = self.get_forge_metrics().await?;
        node_metrics
            .get(metric.as_ref())
            .map_or(Ok(None), |v| v.to_i64().map(Some))
    }

    /// Retrieves all node metrics for a given metric name
    pub async fn get_node_metric_with_name(
        &self,
        metric_name: &str,
    ) -> Result<Option<HashMap<String, MetricValue>>> {
        let metrics = self.get_forge_metrics().await?;
        let search_string = format!("{}{{", metric_name);

        // Filter out all metrics that don't start with the search string
        let result: HashMap<_, _> = metrics
            .iter()
            .filter_map(|(key, value)| {
                if key.starts_with(&search_string) {
                    Some((key.clone(), value.clone()))
                } else {
                    None
                }
            })
            .collect();

        // Return None if the result is empty
        if result.is_empty() {
            Ok(None)
        } else {
            Ok(Some(result))
        }
    }

    /// Fetches and returns all node metrics by pinging the forge_metrics endpoint
    pub async fn get_forge_metrics(&self) -> Result<HashMap<String, MetricValue>> {
        let mut url = self.url.clone();
        url.set_path("forge_metrics");

        // Fetch the metrics from the node
        let response = self.client.get(url).send().await?;
        response
            .json::<HashMap<String, String>>()
            .await?
            .into_iter()
            .map(|(k, v)| match (v.parse::<i64>(), v.parse::<f64>()) {
                (Ok(v), Err(_)) => Ok((k, MetricValue::I64(v))),
                (Err(_), Ok(v)) => Ok((k, MetricValue::F64(v))),
                (Ok(iv), Ok(fv)) => Ok((k, MetricValue::I64orF64(iv, fv))),
                (Err(_), Err(_)) => Err(anyhow::format_err!(
                    "Failed to parse stat value to i64 or f64 {}: {}",
                    &k,
                    &v
                )),
            })
            .collect()
    }
}
