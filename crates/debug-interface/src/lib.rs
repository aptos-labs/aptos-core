// Copyright (c) The Diem Core Contributors
// SPDX-License-Identifier: Apache-2.0

use anyhow::Result;
use reqwest::{blocking, Url};
use std::collections::HashMap;

pub mod node_debug_service;

/// Implement default utility client for NodeDebugInterface
pub struct NodeDebugClient {
    client: blocking::Client,
    url: Url,
}

impl NodeDebugClient {
    /// Create NodeDebugInterfaceClient from a valid socket address.
    pub fn new<A: AsRef<str>>(address: A, port: u16) -> Self {
        let url = Url::parse(&format!("http://{}:{}", address.as_ref(), port)).unwrap();

        Self::from_url(url)
    }

    pub fn from_url(url: Url) -> Self {
        let client = blocking::Client::new();

        Self { client, url }
    }

    /// Retrieves the individual node metric.  Requires all sub fields to match in alphabetical order.
    pub fn get_node_metric<S: AsRef<str>>(&self, metric: S) -> Result<Option<i64>> {
        let metrics = self.get_node_metrics()?;
        Ok(metrics.get(metric.as_ref()).cloned())
    }

    /// Retrieves all node metrics for a given metric name.  Allows for filtering metrics by fields afterwards.
    pub fn get_node_metric_with_name(&self, metric: &str) -> Result<Option<HashMap<String, i64>>> {
        let metrics = self.get_node_metrics()?;
        let search_string = format!("{}{{", metric);

        let result: HashMap<_, _> = metrics
            .iter()
            .filter_map(|(key, value)| {
                if key.starts_with(&search_string) {
                    Some((key.clone(), *value))
                } else {
                    None
                }
            })
            .collect();

        if result.is_empty() {
            Ok(None)
        } else {
            Ok(Some(result))
        }
    }

    pub fn get_node_metrics(&self) -> Result<HashMap<String, i64>> {
        let mut url = self.url.clone();
        url.set_path("metrics");
        let response = self.client.get(url).send()?;

        if !response.status().is_success() {
            anyhow::bail!("Error querying metrics: {}", response.status());
        }

        response
            .json::<HashMap<String, String>>()?
            .into_iter()
            .map(|(k, v)| match v.parse::<i64>() {
                Ok(v) => Ok((k, v)),
                Err(_) => Err(anyhow::format_err!(
                    "Failed to parse stat value to i64 {}: {}",
                    &k,
                    &v
                )),
            })
            .collect()
    }
}

/// Implement default utility client for AsyncNodeDebugInterface
pub struct AsyncNodeDebugClient {
    client: reqwest::Client,
    url: Url,
}

impl AsyncNodeDebugClient {
    /// Create AsyncNodeDebugInterface from a valid socket address.
    pub fn new<A: AsRef<str>>(client: reqwest::Client, address: A, port: u16) -> Self {
        let addr = format!("http://{}:{}", address.as_ref(), port);

        Self {
            client,
            url: Url::parse(&addr).unwrap(),
        }
    }

    pub fn from_url(url: Url) -> Self {
        let client = reqwest::Client::new();

        Self { client, url }
    }

    pub async fn get_node_metric<S: AsRef<str>>(&self, metric: S) -> Result<Option<i64>> {
        let metrics = self.get_node_metrics().await?;
        Ok(metrics.get(metric.as_ref()).cloned())
    }

    /// Retrieves all node metrics for a given metric name.  Allows for filtering metrics by fields afterwards.
    pub async fn get_node_metric_with_name(
        &self,
        metric: &str,
    ) -> Result<Option<HashMap<String, i64>>> {
        let metrics = self.get_node_metrics().await?;
        let search_string = format!("{}{{", metric);

        let result: HashMap<_, _> = metrics
            .iter()
            .filter_map(|(key, value)| {
                if key.starts_with(&search_string) {
                    Some((key.clone(), *value))
                } else {
                    None
                }
            })
            .collect();

        if result.is_empty() {
            Ok(None)
        } else {
            Ok(Some(result))
        }
    }

    pub async fn get_node_metrics(&self) -> Result<HashMap<String, i64>> {
        let mut url = self.url.clone();
        url.set_path("metrics");
        let response = self.client.get(url).send().await?;

        response
            .json::<HashMap<String, String>>()
            .await?
            .into_iter()
            .map(|(k, v)| match v.parse::<i64>() {
                Ok(v) => Ok((k, v)),
                Err(_) => Err(anyhow::format_err!(
                    "Failed to parse stat value to i64 {}: {}",
                    &k,
                    &v
                )),
            })
            .collect()
    }
}
