// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use std::net::{IpAddr, Ipv4Addr};

use super::traits::{MetricCollector, MetricCollectorError};
use anyhow::{anyhow, Context, Result};
use async_trait::async_trait;
use log::debug;
use reqwest::{Client as ReqwestClient, Url};
use std::collections::HashMap;
use std::time::Duration;
use url::Host;

// TODO Make it possible to reject nodes unless they are a specific type.
#[derive(Clone, Debug)]
pub struct ReqwestMetricCollector {
    client: ReqwestClient,

    /// We assume this points to the "base" of a node. We will add ports
    /// and paths to this ourselves.
    node_url: Url,

    /// Metrics port.
    metrics_port: u16,
}

impl ReqwestMetricCollector {
    pub fn new(node_url: Url, metrics_port: u16) -> Self {
        let mut client_builder = ReqwestClient::builder().timeout(Duration::from_secs(4));
        let mut is_localhost = false;
        if let Some(host) = node_url.host() {
            match host {
                Host::Domain(s) => {
                    if s.contains("localhost") {
                        is_localhost = true;
                    }
                }
                Host::Ipv4(ip) => {
                    if ip == Ipv4Addr::LOCALHOST {
                        is_localhost = true;
                    }
                }
                _ => {}
            }
            if is_localhost {
                client_builder = client_builder.local_address(IpAddr::from([127, 0, 0, 1]));
            }
        }
        ReqwestMetricCollector {
            client: client_builder.build().unwrap(),
            node_url,
            metrics_port,
        }
    }

    fn get_url(&self, path: &str) -> Url {
        let mut url = self.node_url.clone();
        url.set_port(Some(self.metrics_port)).unwrap();
        url.set_path(path);
        url
    }

    async fn get_data_from_node(&self, path: &str) -> Result<String, MetricCollectorError> {
        let url = self.get_url(path);
        debug!("Connecting to {}", url);
        let response = self
            .client
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
    async fn collect_system_information(
        &self,
    ) -> Result<HashMap<String, String>, MetricCollectorError> {
        let body = self.get_data_from_node("system_information").await?;
        let raw_map: HashMap<String, serde_json::Value> = serde_json::from_str(&body)
            .context("Failed to process response body as JSON")
            .map_err(|e| MetricCollectorError::ResponseParseError(anyhow!(e)))?;
        let mut out = HashMap::new();
        for (key, value) in raw_map.into_iter() {
            let string_value = value
                .as_str()
                .with_context(|| format!("Failed to convert value to String: {}", value))
                .map_err(|e| MetricCollectorError::ResponseParseError(anyhow!(e)))?
                .to_owned();
            out.insert(key, string_value);
        }
        Ok(out)
    }
}
