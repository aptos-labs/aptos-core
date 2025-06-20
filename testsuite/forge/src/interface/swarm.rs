// Copyright © Aptos Foundation
// Parts of the project are originally copyright © Meta Platforms, Inc.
// SPDX-License-Identifier: Apache-2.0

use crate::{
    AptosPublicInfo, ChainInfo, FullNode, NodeExt, Result, SwarmChaos, Validator, Version,
};
use anyhow::{anyhow, bail};
use aptos_config::{
    config::{NodeConfig, OverrideNodeConfig},
    network_id::NetworkId,
};
use aptos_rest_client::Client as RestClient;
use aptos_sdk::types::PeerId;
use futures::future::{join_all, try_join_all};
use log::info;
use prometheus_http_query::response::{PromqlResult, Sample};
use std::time::{Duration, Instant};

/// Trait used to represent a running network comprised of Validators and FullNodes
#[async_trait::async_trait]
pub trait Swarm: Sync + Send {
    /// Performs a health check on the entire swarm, ensuring all Nodes are Live and that no forks
    /// have occurred
    async fn health_check(&self) -> Result<()>;

    /// Returns an Iterator of references to all the Validators in the Swarm
    fn validators<'a>(&'a self) -> Box<dyn Iterator<Item = &'a dyn Validator> + 'a>;

    /// Returns a reference to the Validator with the provided PeerId
    fn validator(&self, id: PeerId) -> Option<&dyn Validator>;

    /// Upgrade a Validator to run specified `Version`
    async fn upgrade_validator(&mut self, id: PeerId, version: &Version) -> Result<()>;

    /// Returns an Iterator of references to all the FullNodes in the Swarm
    fn full_nodes<'a>(&'a self) -> Box<dyn Iterator<Item = &'a dyn FullNode> + 'a>;

    /// Returns a reference to the FullNode with the provided PeerId
    fn full_node(&self, id: PeerId) -> Option<&dyn FullNode>;

    /// Adds a Validator to the swarm and returns the PeerId
    fn add_validator(&mut self, version: &Version, template: NodeConfig) -> Result<PeerId>;

    /// Removes the Validator with the provided PeerId
    fn remove_validator(&mut self, id: PeerId) -> Result<()>;

    fn add_validator_full_node(
        &mut self,
        version: &Version,
        config: OverrideNodeConfig,
        id: PeerId,
    ) -> Result<PeerId>;

    /// Adds a FullNode to the swarm and returns the PeerId
    async fn add_full_node(
        &mut self,
        version: &Version,
        config: OverrideNodeConfig,
    ) -> Result<PeerId>;

    /// Removes the FullNode with the provided PeerId
    fn remove_full_node(&mut self, id: PeerId) -> Result<()>;

    /// Return a list of supported Versions
    fn versions<'a>(&'a self) -> Box<dyn Iterator<Item = Version> + 'a>;

    /// Construct a ChainInfo from this Swarm
    fn chain_info(&self) -> ChainInfo;

    fn logs_location(&mut self) -> String;

    /// Injects all types of chaos
    async fn inject_chaos(&mut self, chaos: SwarmChaos) -> Result<()>;
    async fn remove_chaos(&mut self, chaos: SwarmChaos) -> Result<()>;
    async fn remove_all_chaos(&mut self) -> Result<()>;

    async fn ensure_no_validator_restart(&self) -> Result<()>;
    async fn ensure_no_fullnode_restart(&self) -> Result<()>;

    // Get prometheus metrics from the swarm
    async fn query_metrics(
        &self,
        query: &str,
        time: Option<i64>,
        timeout: Option<i64>,
    ) -> Result<PromqlResult>;

    async fn query_range_metrics(
        &self,
        query: &str,
        start_time: i64,
        end_time: i64,
        timeout: Option<i64>,
    ) -> Result<Vec<Sample>>;

    fn aptos_public_info(&self) -> AptosPublicInfo {
        self.chain_info().into_aptos_public_info()
    }

    fn chain_info_for_node(&mut self, idx: usize) -> ChainInfo;

    fn aptos_public_info_for_node(&mut self, idx: usize) -> AptosPublicInfo {
        self.chain_info_for_node(idx).into_aptos_public_info()
    }

    fn get_default_pfn_node_config(&self) -> NodeConfig;

    /// Check if the swarm has an indexer. NOTE: in the future we should make this more rich, and include
    /// indexer endpoints, similar to how we collect validator and fullnode endpoints.
    fn has_indexer(&self) -> bool;
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
                .map(|node| node.check_connectivity(NetworkId::Validator, validators.len() - 1))
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

    /// Perform a safety check, ensuring that no forks have occurred in the network.
    async fn fork_check(&self, epoch_duration: Duration) -> Result<()> {
        // Lots of errors can actually occur after an epoch change so guarantee that we change epochs here
        // This can wait for 2x epoch to at least force the caller to be explicit about the epoch duration
        self.wait_for_all_nodes_to_change_epoch(epoch_duration * 2)
            .await?;

        let clients = self
            .validators()
            .map(|node| node.rest_client())
            .chain(self.full_nodes().map(|node| node.rest_client()))
            .collect::<Vec<_>>();

        let versions = try_join_all(
            clients
                .iter()
                .map(|node| node.get_ledger_information())
                .collect::<Vec<_>>(),
        )
        .await?
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

        if !Self::are_root_hashes_equal_at_version(&clients, min_version).await? {
            return Err(anyhow!("Fork check failed"));
        }

        self.wait_for_all_nodes_to_catchup_to_version(max_version, Duration::from_secs(10))
            .await?;

        if !Self::are_root_hashes_equal_at_version(&clients, max_version).await? {
            return Err(anyhow!("Fork check failed"));
        }

        Ok(())
    }

    /// Waits for all nodes to have caught up to the specified `target_version`.
    async fn wait_for_all_nodes_to_catchup_to_version(
        &self,
        target_version: u64,
        timeout: Duration,
    ) -> Result<()> {
        wait_for_all_nodes_to_catchup_to_version(
            &self.get_all_nodes_clients_with_names(),
            target_version,
            timeout,
        )
        .await
    }

    /// Waits for all nodes to have caught up to the specified `target_epoch`.
    async fn wait_for_all_nodes_to_catchup_to_epoch(
        &self,
        target_epoch: u64,
        timeout: Duration,
    ) -> Result<()> {
        wait_for_all_nodes_to_catchup_to_epoch(
            &self.get_all_nodes_clients_with_names(),
            target_epoch,
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

    /// Wait for all nodes in the network to change epochs. This is done by first querying each node
    /// for its current epoch, selecting the max epoch, then waiting for all nodes to sync to max
    /// epoch + 1.
    async fn wait_for_all_nodes_to_change_epoch(&self, timeout: Duration) -> Result<()> {
        let clients = &self.get_all_nodes_clients_with_names();
        if clients.is_empty() {
            bail!("No nodes are available!")
        }

        let highest_synced_epoch = get_highest_synced_epoch(clients).await?;
        wait_for_all_nodes_to_catchup_to_epoch(clients, highest_synced_epoch + 1, timeout).await
    }

    async fn wait_for_all_nodes_to_catchup_to_next(&self, timeout: Duration) -> Result<()> {
        self.wait_for_all_nodes_to_catchup_to_future(timeout, 1)
            .await
    }

    async fn wait_for_all_nodes_to_catchup_to_future(
        &self,
        timeout: Duration,
        versions_to_sync_past: u64,
    ) -> Result<()> {
        let clients = self.get_all_nodes_clients_with_names();
        let highest_synced_version = get_highest_synced_version(&clients).await?;
        wait_for_all_nodes_to_catchup_to_version(
            &clients,
            highest_synced_version + versions_to_sync_past,
            timeout,
        )
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

    async fn get_client_with_newest_ledger_version(&self) -> Option<(u64, RestClient)> {
        let clients = self.get_all_nodes_clients_with_names();
        let ledger_infos = join_all(clients.iter().map(|(_name, client)| async {
            let start = Instant::now();
            let result = client.get_ledger_information().await;

            info!(
                "Fetch from {:?} took {}ms, at version: {}",
                client.path_prefix_string(),
                start.elapsed().as_millis(),
                result
                    .as_ref()
                    .map(|r| r.inner().version as i64)
                    .unwrap_or(-1)
            );
            result
        }))
        .await;
        ledger_infos
            .into_iter()
            .zip(clients)
            .flat_map(|(resp, (_, client))| resp.map(|r| (r.into_inner().version, client)))
            .max_by_key(|(v, _c)| *v)
    }
}

/// Waits for all nodes to have caught up to the specified `target_version`.
pub async fn wait_for_all_nodes_to_catchup_to_version(
    clients: &[(String, RestClient)],
    target_version: u64,
    timeout: Duration,
) -> Result<()> {
    wait_for_all_nodes_to_catchup_to_target_version_or_epoch(
        clients,
        Some(target_version),
        None,
        timeout,
    )
    .await
}

/// Waits for all nodes to have caught up to the specified `target_epoch`.
pub async fn wait_for_all_nodes_to_catchup_to_epoch(
    clients: &[(String, RestClient)],
    target_epoch: u64,
    timeout: Duration,
) -> Result<()> {
    wait_for_all_nodes_to_catchup_to_target_version_or_epoch(
        clients,
        None,
        Some(target_epoch),
        timeout,
    )
    .await
}

/// Waits for all nodes to have caught up to the specified `target_version` or `target_epoch`.
async fn wait_for_all_nodes_to_catchup_to_target_version_or_epoch(
    clients: &[(String, RestClient)],
    target_version: Option<u64>,
    target_epoch: Option<u64>,
    timeout: Duration,
) -> Result<()> {
    if target_version.is_none() && target_epoch.is_none() {
        bail!("No target version or epoch was specified!")
    }

    let start_time = Instant::now();
    loop {
        // Fetch the current versions and epochs of all nodes
        let version_and_epoch_results: Result<Vec<_>> =
            try_join_all(clients.iter().map(|(node_name, node)| async move {
                let node_ledger_info_response = node.get_ledger_information().await?.into_inner();
                Ok((
                    node_name,
                    node_ledger_info_response.version,
                    node_ledger_info_response.epoch,
                ))
            }))
            .await;
        let node_versions_and_epochs =
            version_and_epoch_results.map(|results| results.into_iter().collect::<Vec<_>>());

        // Check if all nodes are caught up to the target version
        let all_caught_up_to_version = target_version
            .map(|target_version| {
                node_versions_and_epochs
                    .as_ref()
                    .map(|responses| {
                        responses
                            .iter()
                            .all(|(_, version, _)| *version >= target_version)
                    })
                    .unwrap_or(false) // No version found
            })
            .unwrap_or(true); // No target version was specified

        // Check if all nodes are caught up to the target epoch
        let all_caught_up_to_epoch = target_epoch
            .map(|target_epoch| {
                node_versions_and_epochs
                    .as_ref()
                    .map(|responses| responses.iter().all(|(_, _, epoch)| *epoch >= target_epoch))
                    .unwrap_or(false) // No epoch found
            })
            .unwrap_or(true); // No target epoch was specified

        // Check if all targets have been met
        if all_caught_up_to_version && all_caught_up_to_epoch {
            info!(
                "All nodes caught up to target version and epoch ({:?}, {:?}) successfully, in {} seconds",
                target_version,
                target_epoch,
                start_time.elapsed().as_secs()
            );
            return Ok(());
        }

        // Check if we've timed out while waiting
        if start_time.elapsed() > timeout {
            return Err(anyhow!(
                "Waiting for nodes to catch up to target version and epoch ({:?}, {:?}) timed out after {} seconds, current status: {:?}",
                target_version,
                target_epoch,
                start_time.elapsed().as_secs(),
                node_versions_and_epochs
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
        bail!("No nodes are available!")
    }
    let highest_synced_version = get_highest_synced_version(clients).await?;
    wait_for_all_nodes_to_catchup_to_version(clients, highest_synced_version, timeout).await
}

/// Returns the highest synced version of the given clients
pub async fn get_highest_synced_version(clients: &[(String, RestClient)]) -> Result<u64> {
    let (highest_synced_version, _) = get_highest_synced_version_and_epoch(clients).await?;
    Ok(highest_synced_version)
}

/// Returns the highest synced epoch of the given clients
pub async fn get_highest_synced_epoch(clients: &[(String, RestClient)]) -> Result<u64> {
    let (_, highest_synced_epoch) = get_highest_synced_version_and_epoch(clients).await?;
    Ok(highest_synced_epoch)
}

/// Returns the highest synced version and epoch of the given clients
pub async fn get_highest_synced_version_and_epoch(
    clients: &[(String, RestClient)],
) -> Result<(u64, u64)> {
    let mut latest_version_and_epoch = (0, 0);
    for (_, client) in clients {
        latest_version_and_epoch = latest_version_and_epoch.max(
            client
                .get_ledger_information()
                .await
                .map(|r| (r.inner().version, r.inner().epoch))
                .unwrap_or((0, 0)),
        );
    }
    Ok(latest_version_and_epoch)
}
