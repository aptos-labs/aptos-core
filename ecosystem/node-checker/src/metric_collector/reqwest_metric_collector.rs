// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use super::traits::{MetricCollector, MetricCollectorError, SystemInformation};
use crate::configuration::NodeAddress;
use anyhow::{anyhow, Context, Result};
use async_trait::async_trait;
use log::debug;
use reqwest::Url;
use std::{collections::HashMap, time::Duration};

// TODO Make it possible to reject nodes unless they are a specific type.
#[derive(Clone, Debug)]
pub struct ReqwestMetricCollector {
    node_address: NodeAddress,
}

impl ReqwestMetricCollector {
    pub fn new(node_address: NodeAddress) -> Self {
        ReqwestMetricCollector { node_address }
    }

    fn get_url(&self, path: &str) -> Url {
        let mut url = self.node_address.get_metrics_url();
        url.set_path(path);
        url
    }

    async fn get_data_from_node(&self, path: &str) -> Result<String, MetricCollectorError> {
        let url = self.get_url(path);
        debug!("Connecting to {}", url);
        let response = self
            .node_address
            .get_metrics_client(Duration::from_secs(4))
            .get(url.clone())
            .send()
            .await
            // TODO: This context doesn't make it through to the client, fix that.
            .with_context(|| format!("Failed to get data from {}", url))
            .map_err(|e| MetricCollectorError::GetDataError(anyhow!(e)))?;
        let body = response
            .text()
            .await
            .with_context(|| format!("Failed to process response body from {}", url))
            .map_err(|e| MetricCollectorError::ResponseParseError(anyhow!(e)))?;
        Ok(body)
    }
}

#[async_trait]
impl MetricCollector for ReqwestMetricCollector {
    async fn collect_metrics(&self) -> Result<Vec<String>, MetricCollectorError> {
        let body = self.get_data_from_node("metrics").await?;
        Ok(body.lines().map(|line| line.to_owned()).collect())
    }

    /// We know that this endpoint returns JSON that is actually just HashMap<String, String>.
    /// Better than this would be to have that endpoint return a serialized struct that we can
    /// use here to deseralize. TODO for that.
    async fn collect_system_information(&self) -> Result<SystemInformation, MetricCollectorError> {
        let body = self.get_data_from_node("system_information").await?;
        let data: HashMap<String, String> = serde_json::from_str(&body)
            .context("Failed to process response body as valid JSON with string key/values")
            .map_err(|e| MetricCollectorError::ResponseParseError(anyhow!(e)))?;
        Ok(SystemInformation(data))
    }
}
