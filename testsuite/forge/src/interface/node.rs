// Copyright © Velor Foundation
// Parts of the project are originally copyright © Meta Platforms, Inc.
// SPDX-License-Identifier: Apache-2.0

use crate::{Result, Version};
use anyhow::anyhow;
use velor_config::{config::NodeConfig, network_id::NetworkId};
use velor_inspection_service::inspection_client::InspectionClient;
use velor_rest_client::{VelorBaseUrl, Client as RestClient};
use velor_sdk::types::PeerId;
use std::{
    collections::HashMap,
    time::{Duration, Instant},
};
use url::Url;

#[derive(Debug)]
pub enum HealthCheckError {
    NotRunning(String),
    Failure(anyhow::Error),
    Unknown(anyhow::Error),
}

impl std::fmt::Display for HealthCheckError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "{:?}", self)
    }
}

impl std::error::Error for HealthCheckError {}

/// Trait used to represent a running Validator or FullNode
#[async_trait::async_trait]
pub trait Node: Send + Sync {
    /// Return the PeerId of this Node
    fn peer_id(&self) -> PeerId;

    /// Return index of the node
    fn index(&self) -> usize;

    /// Return the human readable name of this Node
    fn name(&self) -> &str;

    /// Return the version this node is running
    fn version(&self) -> Version;

    /// Return the URL for the REST API endpoint of this Node
    fn rest_api_endpoint(&self) -> Url;

    /// Return the URL for the debug-interface for this Node
    fn inspection_service_endpoint(&self) -> Url;

    /// Return a reference to the Config this Node is using
    fn config(&self) -> &NodeConfig;

    /// Start this Node.
    /// This should be a noop if the Node is already running.
    async fn start(&self) -> Result<()>;

    /// Stop this Node.
    /// This should be a noop if the Node isn't running.
    async fn stop(&self) -> Result<()>;

    async fn get_identity(&self) -> Result<String>;

    async fn set_identity(&self, k8s_secret_name: String) -> Result<()>;
    /// Clears this Node's Storage. This stops the node as well
    async fn clear_storage(&self) -> Result<()>;

    async fn health_check(&self) -> Result<(), HealthCheckError>;

    fn counter(&self, counter: &str, port: u64) -> Result<f64>;

    fn expose_metric(&self) -> Result<u64>;

    fn service_name(&self) -> Option<String>;
}

/// Trait used to represent a running Validator
#[async_trait::async_trait]
pub trait Validator: Node + Sync {
    async fn check_connectivity(
        &self,
        network_id: NetworkId,
        expected_peers: usize,
    ) -> Result<bool> {
        if expected_peers == 0 {
            return Ok(true);
        }

        self.get_connected_peers(network_id, None)
            .await
            .map(|maybe_n| maybe_n.map(|n| n >= expected_peers as i64).unwrap_or(false))
    }

    async fn wait_for_connectivity(
        &self,
        network_id: NetworkId,
        expected_peers: usize,
        deadline: Instant,
    ) -> Result<()> {
        while !self.check_connectivity(network_id, expected_peers).await? {
            if Instant::now() > deadline {
                return Err(anyhow!("waiting for connectivity timed out"));
            }

            tokio::time::sleep(Duration::from_millis(500)).await;
        }

        Ok(())
    }
}

/// Trait used to represent a running FullNode
#[async_trait::async_trait]
pub trait FullNode: Node + Sync {
    //TODO handle VFNs querying if they are connected to a validator
    async fn check_connectivity(&self) -> Result<bool> {
        const DIRECTION: Option<&str> = Some("outbound");
        const EXPECTED_PEERS: usize = 1;

        for &network_id in &[NetworkId::Public, NetworkId::Vfn] {
            let r = self
                .get_connected_peers(network_id, DIRECTION)
                .await
                .map(|maybe_n| maybe_n.map(|n| n >= EXPECTED_PEERS as i64).unwrap_or(false));
            if let Ok(true) = r {
                return Ok(true);
            }
        }
        Ok(false)
    }

    async fn wait_for_connectivity(&self, deadline: Instant) -> Result<()> {
        while !self.check_connectivity().await? {
            if Instant::now() > deadline {
                return Err(anyhow!("waiting for connectivity timed out"));
            }

            tokio::time::sleep(Duration::from_millis(500)).await;
        }

        Ok(())
    }
}

impl<T: ?Sized> NodeExt for T where T: Node {}

#[async_trait::async_trait]
pub trait NodeExt: Node {
    /// Return REST API client of this Node
    fn rest_client(&self) -> RestClient {
        RestClient::new(self.rest_api_endpoint())
    }

    /// Return REST API client of this Node
    fn rest_client_with_timeout(&self, timeout: Duration) -> RestClient {
        RestClient::builder(VelorBaseUrl::Custom(self.rest_api_endpoint()))
            .timeout(timeout)
            .build()
    }

    /// Return an InspectionClient for this Node
    fn inspection_client(&self) -> InspectionClient {
        InspectionClient::new(self.inspection_service_endpoint())
    }

    /// Restarts this Node by calling Node::Stop followed by Node::Start
    async fn restart(&mut self) -> Result<()> {
        self.stop().await?;
        self.start().await
    }

    /// Query a Metric for from this Node
    async fn get_metric_i64(&self, metric_name: &str) -> Result<Option<i64>> {
        self.inspection_client()
            .get_node_metric_i64(metric_name)
            .await
    }

    async fn get_metric_with_fields_i64(
        &self,
        metric_name: &str,
        fields: HashMap<String, String>,
    ) -> Result<Option<i64>> {
        let filtered: Vec<_> = self
            .inspection_client()
            .get_node_metric_with_name(metric_name)
            .await?
            .into_iter()
            .flat_map(|map| map.into_iter())
            .filter_map(|(metric, metric_value)| {
                if fields
                    .iter()
                    .all(|(key, value)| metric.contains(&format!("{}={}", key, value)))
                {
                    Some(metric_value)
                } else {
                    None
                }
            })
            .collect();

        Ok(if filtered.is_empty() {
            None
        } else {
            let checked: Result<Vec<i64>> = filtered.into_iter().map(|v| v.to_i64()).collect();
            Some(checked?.into_iter().sum())
        })
    }

    async fn get_connected_peers(
        &self,
        network_id: NetworkId,
        direction: Option<&str>,
    ) -> Result<Option<i64>> {
        let mut map = HashMap::new();
        map.insert("network_id".to_string(), network_id.to_string());
        if let Some(direction) = direction {
            map.insert("direction".to_string(), direction.to_string());
        }
        self.get_metric_with_fields_i64("velor_connections", map)
            .await
    }

    async fn liveness_check(&self, seconds: u64) -> Result<()> {
        Ok(self.rest_client().health_check(seconds).await?)
    }

    async fn wait_until_healthy(&self, deadline: Instant) -> Result<()> {
        let mut healthcheck_error =
            HealthCheckError::Unknown(anyhow::anyhow!("No healthcheck performed yet"));
        while Instant::now() < deadline {
            healthcheck_error = match self.health_check().await {
                Ok(()) => return Ok(()),
                Err(HealthCheckError::NotRunning(error)) => {
                    return Err(anyhow::anyhow!(
                        "Node {}:{} not running! Error: {:?}",
                        self.name(),
                        self.peer_id(),
                        error,
                    ))
                },
                Err(e) => e, // For other errors we'll retry
            };

            tokio::time::sleep(Duration::from_millis(500)).await;
        }

        Err(anyhow::anyhow!(
            "Timed out waiting for Node {}:{} to be healthy: Error: {:?}",
            self.name(),
            self.peer_id(),
            healthcheck_error
        ))
    }
}
