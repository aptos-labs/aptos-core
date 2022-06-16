// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use anyhow::Result;
use reqwest::Url;
use std::collections::HashMap;

pub struct InspectionClient {
    client: reqwest::Client,
    url: Url,
}

impl InspectionClient {
    /// Create an InspectionClient from a valid socket address
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

    /// Retrieves all node metrics for a given metric name.
    /// Allows for filtering metrics by fields afterwards.
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
        url.set_path("forge_metrics");
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
