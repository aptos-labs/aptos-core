// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use std::net::{IpAddr, Ipv4Addr};

use super::traits::{MetricCollector, MetricCollectorError};
use anyhow::{anyhow, Context, Result};
use async_trait::async_trait;
use log::debug;
use reqwest::{Client as ReqwestClient, Url};
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

    fn get_metrics_endpoint(&self) -> Url {
        let mut url = self.node_url.clone();
        url.set_port(Some(self.metrics_port)).unwrap();
        url.set_path("metrics");
        url
    }
}

#[async_trait]
impl MetricCollector for ReqwestMetricCollector {
    async fn collect_metrics(&self) -> Result<Vec<String>, MetricCollectorError> {
        let url = self.get_metrics_endpoint();
        debug!("Connecting to {} to collect metrics", url);
        let response = self
            .client
            .get(url.clone())
            .send()
            .await
            .with_context(|| format!("Failed to get data from {}", url))
            .map_err(|e| MetricCollectorError::GetDataError(anyhow!(e)))?;
        let body = response
            .text()
            .await
            .with_context(|| format!("Failed to process response body from {}", url))
            .map_err(|e| MetricCollectorError::ResponseParseError(anyhow!(e)))?;
        Ok(body.lines().map(|line| line.to_owned()).collect())
    }
}
