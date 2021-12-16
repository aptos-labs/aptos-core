// Copyright (c) The Diem Core Contributors
// SPDX-License-Identifier: Apache-2.0

use crate::{Result, Version};
use anyhow::anyhow;
use debug_interface::AsyncNodeDebugClient;
use diem_config::{config::NodeConfig, network_id::NetworkId};
use diem_rest_client::Client as RestClient;
use diem_sdk::{
    client::{BlockingClient, Client as JsonRpcClient},
    types::PeerId,
};
use std::{
    collections::HashMap,
    time::{Duration, Instant},
};
use url::Url;

#[derive(Debug)]
pub enum HealthCheckError {
    NotRunning,
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

    /// Return the human readable name of this Node
    fn name(&self) -> &str;

    /// Return the version this node is running
    fn version(&self) -> Version;

    /// Return the URL for the JSON-RPC endpoint of this Node
    fn json_rpc_endpoint(&self) -> Url;

    /// Return the URL for the REST API endpoint of this Node
    fn rest_api_endpoint(&self) -> Url;

    /// Return the URL for the debug-interface for this Node
    fn debug_endpoint(&self) -> Url;

    /// Return a reference to the Config this Node is using
    fn config(&self) -> &NodeConfig;

    /// Start this Node.
    /// This should be a noop if the Node is already running.
    async fn start(&mut self) -> Result<()>;

    /// Stop this Node.
    /// This should be a noop if the Node isn't running.
    fn stop(&mut self) -> Result<()>;

    /// Clears this Node's Storage
    fn clear_storage(&mut self) -> Result<()>;

    /// Performs a Health Check on the Node
    async fn health_check(&mut self) -> Result<(), HealthCheckError>;

    fn counter(&self, counter: &str, port: u64) -> Result<f64>;

    fn expose_metric(&self) -> Result<u64>;
}

/// Trait used to represent a running Validator
#[async_trait::async_trait]
pub trait Validator: Node + Sync {
    async fn check_connectivity(&self, expected_peers: usize) -> Result<bool> {
        if expected_peers == 0 {
            return Ok(true);
        }

        self.get_connected_peers(NetworkId::Validator, None)
            .await
            .map(|maybe_n| maybe_n.map(|n| n >= expected_peers as i64).unwrap_or(false))
    }

    async fn wait_for_connectivity(&self, expected_peers: usize, deadline: Instant) -> Result<()> {
        while !self.check_connectivity(expected_peers).await? {
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

        self.get_connected_peers(NetworkId::Public, DIRECTION)
            .await
            .map(|maybe_n| maybe_n.map(|n| n >= EXPECTED_PEERS as i64).unwrap_or(false))
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
    /// Return JSON-RPC client of this Node
    fn async_json_rpc_client(&self) -> JsonRpcClient {
        JsonRpcClient::new(self.json_rpc_endpoint().to_string())
    }

    /// Return REST API client of this Node
    fn rest_client(&self) -> RestClient {
        RestClient::new(self.rest_api_endpoint())
    }

    /// Return JSON-RPC client of this Node
    fn json_rpc_client(&self) -> BlockingClient {
        BlockingClient::new(self.json_rpc_endpoint())
    }

    /// Return a NodeDebugClient for this Node
    fn debug_client(&self) -> AsyncNodeDebugClient {
        AsyncNodeDebugClient::from_url(self.debug_endpoint())
    }

    /// Restarts this Node by calling Node::Stop followed by Node::Start
    async fn restart(&mut self) -> Result<()> {
        self.stop()?;
        self.start().await
    }

    /// Query a Metric for from this Node
    async fn get_metric(&self, metric_name: &str) -> Result<Option<i64>> {
        self.debug_client().get_node_metric(metric_name).await
    }

    async fn get_metric_with_fields(
        &self,
        metric_name: &str,
        fields: HashMap<String, String>,
    ) -> Result<Option<i64>> {
        let filtered: Vec<_> = self
            .debug_client()
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
            Some(filtered.iter().sum())
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
        self.get_metric_with_fields("diem_connections", map).await
    }

    async fn liveness_check(&self, seconds: u64) -> Result<()> {
        self.rest_client().health_check(seconds).await
    }

    async fn wait_until_healthy(&mut self, deadline: Instant) -> Result<()> {
        while Instant::now() < deadline {
            match self.health_check().await {
                Ok(()) => return Ok(()),
                Err(HealthCheckError::NotRunning) => {
                    return Err(anyhow::anyhow!(
                        "Node {}:{} not running",
                        self.name(),
                        self.peer_id()
                    ))
                }
                Err(_) => {} // For other errors we'll retry
            }

            tokio::time::sleep(Duration::from_millis(500)).await;
        }

        Err(anyhow::anyhow!(
            "Timed out waiting for Node {}:{} to be healthy",
            self.name(),
            self.peer_id()
        ))
    }
}
