// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use crate::{
    interface::system_metrics::SystemMetricsThreshold, AptosPublicInfo, ChainInfo, FullNode,
    NodeExt, Result, SwarmChaos, Validator, Version,
};
use anyhow::{anyhow, bail};
use aptos_config::config::NodeConfig;
use aptos_logger::info;
use aptos_rest_client::Client as RestClient;
use aptos_sdk::types::PeerId;
use futures::future::try_join_all;
use prometheus_http_query::response::PromqlResult;
use std::time::{Duration, Instant};
use tokio::runtime::Runtime;

/// Trait used to represent a running network comprised of Validators and FullNodes
#[async_trait::async_trait]
pub trait Swarm: Sync {
    /// Performs a health check on the entire swarm, ensuring all Nodes are Live and that no forks
    /// have occurred
    async fn health_check(&mut self) -> Result<()>;

    /// Returns an Iterator of references to all the Validators in the Swarm
    fn validators<'a>(&'a self) -> Box<dyn Iterator<Item = &'a dyn Validator> + 'a>;

    /// Returns an Iterator of mutable references to all the Validators in the Swarm
    fn validators_mut<'a>(&'a mut self) -> Box<dyn Iterator<Item = &'a mut dyn Validator> + 'a>;

    /// Returns a reference to the Validator with the provided PeerId
    fn validator(&self, id: PeerId) -> Option<&dyn Validator>;

    /// Returns a mutable reference to the Validator with the provided PeerId
    fn validator_mut(&mut self, id: PeerId) -> Option<&mut dyn Validator>;

    /// Upgrade a Validator to run specified `Version`
    async fn upgrade_validator(&mut self, id: PeerId, version: &Version) -> Result<()>;

    /// Returns an Iterator of references to all the FullNodes in the Swarm
    fn full_nodes<'a>(&'a self) -> Box<dyn Iterator<Item = &'a dyn FullNode> + 'a>;

    /// Returns an Iterator of mutable references to all the FullNodes in the Swarm
    fn full_nodes_mut<'a>(&'a mut self) -> Box<dyn Iterator<Item = &'a mut dyn FullNode> + 'a>;

    /// Returns a reference to the FullNode with the provided PeerId
    fn full_node(&self, id: PeerId) -> Option<&dyn FullNode>;

    /// Returns a mutable reference to the FullNode with the provided PeerId
    fn full_node_mut(&mut self, id: PeerId) -> Option<&mut dyn FullNode>;

    /// Adds a Validator to the swarm and returns the PeerId
    fn add_validator(&mut self, version: &Version, template: NodeConfig) -> Result<PeerId>;

    /// Removes the Validator with the provided PeerId
    fn remove_validator(&mut self, id: PeerId) -> Result<()>;

    fn add_validator_full_node(
        &mut self,
        version: &Version,
        template: NodeConfig,
        id: PeerId,
    ) -> Result<PeerId>;

    /// Adds a FullNode to the swarm and returns the PeerId
    fn add_full_node(&mut self, version: &Version, template: NodeConfig) -> Result<PeerId>;

    /// Removes the FullNode with the provided PeerId
    fn remove_full_node(&mut self, id: PeerId) -> Result<()>;

    /// Return a list of supported Versions
    fn versions<'a>(&'a self) -> Box<dyn Iterator<Item = Version> + 'a>;

    /// Construct a ChainInfo from this Swarm
    fn chain_info(&mut self) -> ChainInfo<'_>;

    fn logs_location(&mut self) -> String;

    /// Injects all types of chaos
    fn inject_chaos(&mut self, chaos: SwarmChaos) -> Result<()>;
    fn remove_chaos(&mut self, chaos: SwarmChaos) -> Result<()>;
    fn remove_all_chaos(&mut self) -> Result<()>;

    async fn ensure_no_validator_restart(&self) -> Result<()>;
    async fn ensure_no_fullnode_restart(&self) -> Result<()>;

    async fn ensure_healthy_system_metrics(
        &mut self,
        start_time: i64,
        end_time: i64,
        threshold: SystemMetricsThreshold,
    ) -> Result<()>;

    // Get prometheus metrics from the swarm
    async fn query_metrics(
        &self,
        query: &str,
        time: Option<i64>,
        timeout: Option<i64>,
    ) -> Result<PromqlResult>;

    fn aptos_public_info(&mut self) -> AptosPublicInfo<'_> {
        self.chain_info().into_aptos_public_info()
    }

    fn chain_info_for_node(&mut self, idx: usize) -> ChainInfo<'_>;

    fn aptos_public_info_for_node(&mut self, idx: usize) -> AptosPublicInfo<'_> {
        self.chain_info_for_node(idx).into_aptos_public_info()
    }
}

impl<T: ?Sized> SwarmExt for T where T: Swarm {}

#[async_trait::async_trait]
pub trait SwarmExt: Swarm {
    async fn liveness_check(&self, deadline: Instant) -> Result<()> {
        let liveness_check_seconds = 10;
        let validators = self.validators().collect::<Vec<_>>();
        let full_nodes = self.full_nodes().collect::<Vec<_>>();

        while try_join_all(
            validators
                .iter()
                .map(|node| node.liveness_check(liveness_check_seconds))
                .chain(
                    full_nodes
                        .iter()
                        .map(|node| node.liveness_check(liveness_check_seconds)),
                ),
        )
        .await
        .is_err()
        {
            if Instant::now() > deadline {
                return Err(anyhow!("Swarm liveness check timed out"));
            }

            tokio::time::sleep(Duration::from_millis(500)).await;
        }
        info!("Swarm liveness check passed");
        Ok(())
    }

    /// Waits for the swarm to achieve connectivity
    async fn wait_for_connectivity(&self, deadline: Instant) -> Result<()> {
        let validators = self.validators().collect::<Vec<_>>();
        let full_nodes = self.full_nodes().collect::<Vec<_>>();

        while !try_join_all(
            validators
                .iter()
                .map(|node| node.check_connectivity(validators.len() - 1))
                .chain(full_nodes.iter().map(|node| node.check_connectivity())),
        )
        .await
        .map(|v| v.iter().all(|r| *r))
        .unwrap_or(false)
        {
            if Instant::now() > deadline {
                return Err(anyhow!("waiting for swarm connectivity timed out"));
            }

            tokio::time::sleep(Duration::from_millis(500)).await;
        }
        info!("Swarm connectivity check passed");
        Ok(())
    }

    /// Perform a safety check, ensuring that no forks have occurred in the network.
    fn fork_check(&self) -> Result<()> {
        // Checks if root_hashes are equal across all nodes at a given version
        async fn are_root_hashes_equal_at_version(
            clients: &[RestClient],
            version: u64,
        ) -> Result<bool> {
            let root_hashes = try_join_all(
                clients
                    .iter()
                    .map(|node| node.get_transaction_by_version(version))
                    .collect::<Vec<_>>(),
            )
            .await?
            .into_iter()
            .map(|r| {
                r.into_inner()
                    .transaction_info()
                    .unwrap()
                    .accumulator_root_hash
            })
            .collect::<Vec<_>>();

            Ok(root_hashes.windows(2).all(|w| w[0] == w[1]))
        }

        let runtime = Runtime::new().unwrap();

        let clients = self
            .validators()
            .map(|node| node.rest_client())
            .chain(self.full_nodes().map(|node| node.rest_client()))
            .collect::<Vec<_>>();

        let versions = runtime
            .block_on(try_join_all(
                clients
                    .iter()
                    .map(|node| node.get_ledger_information())
                    .collect::<Vec<_>>(),
            ))?
            .into_iter()
            .map(|resp| resp.into_inner().version)
            .collect::<Vec<u64>>();
        let min_version = versions
            .iter()
            .min()
            .copied()
            .ok_or_else(|| anyhow!("Unable to query nodes for their latest version"))?;
        let max_version = versions
            .iter()
            .max()
            .copied()
            .ok_or_else(|| anyhow!("Unable to query nodes for their latest version"))?;

        if !runtime.block_on(are_root_hashes_equal_at_version(&clients, min_version))? {
            return Err(anyhow!("Fork check failed"));
        }

        runtime.block_on(
            self.wait_for_all_nodes_to_catchup_to_version(max_version, Duration::from_secs(10)),
        )?;

        if !runtime.block_on(are_root_hashes_equal_at_version(&clients, max_version))? {
            return Err(anyhow!("Fork check failed"));
        }

        Ok(())
    }

    /// Waits for all nodes to have caught up to the specified `verison`.
    async fn wait_for_all_nodes_to_catchup_to_version(
        &self,
        version: u64,
        timeout: Duration,
    ) -> Result<()> {
        wait_for_all_nodes_to_catchup_to_version(
            &self.get_all_nodes_clients_with_names(),
            version,
            timeout,
        )
        .await
    }

    /// Wait for all nodes in the network to be caught up. This is done by first querying each node
    /// for its current version, selects the max version, then waits for all nodes to catch up to
    /// that version. Once done, we can guarantee that all transactions committed before invocation
    /// of this function are available at all the nodes in the swarm
    async fn wait_for_all_nodes_to_catchup(&self, timeout: Duration) -> Result<()> {
        wait_for_all_nodes_to_catchup(&self.get_all_nodes_clients_with_names(), timeout).await
    }

    async fn wait_for_all_nodes_to_catchup_to_next(&self, timeout: Duration) -> Result<()> {
        let clients = self.get_all_nodes_clients_with_names();
        let highest_synced_version = get_highest_synced_version(&clients).await?;
        wait_for_all_nodes_to_catchup_to_version(&clients, highest_synced_version + 1, timeout)
            .await
    }

    fn get_validator_clients_with_names(&self) -> Vec<(String, RestClient)> {
        self.validators()
            .map(|node| (node.name().to_string(), node.rest_client()))
            .collect()
    }

    fn get_all_nodes_clients_with_names(&self) -> Vec<(String, RestClient)> {
        self.validators()
            .map(|node| (node.name().to_string(), node.rest_client()))
            .chain(
                self.full_nodes()
                    .map(|node| (node.name().to_string(), node.rest_client())),
            )
            .collect()
    }

    fn get_clients_for_peers(&self, peers: &[PeerId], client_timeout: Duration) -> Vec<RestClient> {
        peers
            .iter()
            .map(|peer| {
                self.validator(*peer)
                    .map(|n| n.rest_client_with_timeout(client_timeout))
                    .unwrap_or_else(|| {
                        self.full_node(*peer)
                            .unwrap()
                            .rest_client_with_timeout(client_timeout)
                    })
            })
            .collect()
    }
}

/// Waits for all nodes to have caught up to the specified `verison`.
pub async fn wait_for_all_nodes_to_catchup_to_version(
    clients: &[(String, RestClient)],
    version: u64,
    timeout: Duration,
) -> Result<()> {
    let start_time = Instant::now();
    loop {
        let results: Result<Vec<_>> = try_join_all(clients.iter().map(|(name, node)| async move {
            Ok((
                name,
                node.get_ledger_information().await?.into_inner().version,
            ))
        }))
        .await;
        let versions = results
            .map(|resps| resps.into_iter().collect::<Vec<_>>())
            .ok();
        let all_caught_up = versions
            .clone()
            .map(|resps| resps.iter().all(|(_, v)| *v >= version))
            .unwrap_or(false);
        if all_caught_up {
            info!(
                "All nodes caught up successfully in {}s",
                start_time.elapsed().as_secs()
            );
            return Ok(());
        }

        if start_time.elapsed() > timeout {
            return Err(anyhow!(
                "Waiting for nodes to catch up to version {} timed out, current status: {:?}",
                version,
                versions.unwrap_or_default()
            ));
        }

        tokio::time::sleep(Duration::from_millis(500)).await;
    }
}

/// Wait for all nodes in the network to be caught up. This is done by first querying each node
/// for its current version, selects the max version, then waits for all nodes to catch up to
/// that version. Once done, we can guarantee that all transactions committed before invocation
/// of this function are available at all the nodes in the swarm
pub async fn wait_for_all_nodes_to_catchup(
    clients: &[(String, RestClient)],
    timeout: Duration,
) -> Result<()> {
    if clients.is_empty() {
        bail!("no nodes available")
    }
    let highest_synced_version = get_highest_synced_version(clients).await?;
    wait_for_all_nodes_to_catchup_to_version(clients, highest_synced_version, timeout).await
}

/// Returns the highest synced version of the given clients
pub async fn get_highest_synced_version(clients: &[(String, RestClient)]) -> Result<u64> {
    let mut latest_version = 0u64;
    for (_, c) in clients {
        latest_version = latest_version.max(
            c.get_ledger_information()
                .await
                .map(|r| r.into_inner().version)
                .unwrap_or(0),
        );
    }
    Ok(latest_version)
}
