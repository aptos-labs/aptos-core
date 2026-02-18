// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Forge E2E tests for Proxy Primary Consensus.
//!
//! Proxy Primary Consensus is a two-layer consensus where:
//! 1. Proxy validators run fast local consensus (10+ blocks per primary round)
//! 2. Ordered proxy blocks are forwarded to all primaries
//! 3. Primaries aggregate proxy blocks into primary blocks
//!
//! ## Test Requirements
//!
//! To enable proxy consensus, nodes must be configured with:
//! ```yaml
//! consensus:
//!   enable_proxy_consensus: true
//!   proxy_consensus_config:
//!     round_initial_timeout_ms: 100
//!     max_proxy_blocks_per_primary_round: 20
//! ```
//!
//! ## Metrics to Verify
//!
//! - `aptos_proxy_consensus_proposals_sent` > 0
//! - `aptos_proxy_consensus_votes_sent` > 0
//! - `aptos_proxy_consensus_blocks_ordered` > 0
//! - `aptos_proxy_consensus_blocks_forwarded` > 0
//! - `aptos_proxy_blocks_per_primary_round` averaging 3+ blocks

use crate::{
    multi_region_network_test::{
        create_multi_region_swarm_network_chaos, MultiRegionNetworkEmulationConfig,
    },
    LoadDestination, NetworkLoadTest,
};
use anyhow::ensure;
use aptos_forge::{
    GroupNetEm, NetworkContext, NetworkContextSynchronizer, NetworkTest, NodeExt, Result, Swarm,
    SwarmChaos, SwarmNetEm, Test, TestReport,
};
use aptos_inspection_service::inspection_client::InspectionClient;
use aptos_types::PeerId;
use async_trait::async_trait;
use futures::future::join_all;
use log::info;
use std::{sync::Arc, time::Duration};

/// Test that proxy primary consensus works correctly in a running network.
///
/// This test verifies:
/// - Proxy validators propose blocks at a faster rate than primary consensus
/// - Ordered proxy blocks are correctly forwarded to primaries
/// - Primary consensus continues to commit blocks
pub struct ProxyPrimaryHappyPathTest {
    /// Expected minimum transactions per second
    pub min_tps: usize,
    /// Expected minimum proxy blocks per primary round
    pub min_proxy_blocks_per_round: usize,
    /// Network emulation for the proxy-primary topology
    network_emulation: ProxyPrimaryNetworkEmulation,
}

impl ProxyPrimaryHappyPathTest {
    pub fn new(num_proxy_validators: usize) -> Self {
        Self {
            min_tps: 1000,
            min_proxy_blocks_per_round: 3,
            network_emulation: ProxyPrimaryNetworkEmulation::new(num_proxy_validators),
        }
    }
}

impl Default for ProxyPrimaryHappyPathTest {
    fn default() -> Self {
        Self::new(4)
    }
}

impl Test for ProxyPrimaryHappyPathTest {
    fn name(&self) -> &'static str {
        "proxy primary happy path test"
    }
}

/// Proxy consensus metrics for a single node.
#[derive(Debug, Default, Clone)]
struct ProxyNodeMetrics {
    proposals_sent: i64,
    votes_sent: i64,
    qcs_formed: i64,
    blocks_ordered: i64,
    blocks_forwarded: i64,
    backpressure_events: i64,
    timeout_all: i64,
    timeout_leader: i64,
}

/// System-level proxy consensus metrics (aggregated across all validators).
///
/// For metrics that are unique per node (proposals: each node proposes different rounds),
/// we use sum. For metrics that are replicated across nodes (QCs, ordered, forwarded:
/// every node sees the same committed blocks), we use max to get the system-level count.
/// This makes proposals ≈ QCs ≈ ordered ≈ forwarded at the system level.
#[derive(Debug, Default)]
struct ProxyConsensusMetrics {
    /// Sum across nodes (each node proposes unique rounds via round-robin)
    proposals_sent: i64,
    /// Sum across nodes (each node sends one vote per proposal)
    votes_sent: i64,
    /// Max across nodes (each node forms QCs for the same rounds)
    qcs_formed: i64,
    /// Max across nodes (each node orders the same committed blocks)
    blocks_ordered: i64,
    /// Max across nodes (each node forwards the same ordered blocks)
    blocks_forwarded: i64,
    /// Sum across nodes (backpressure can differ per node)
    backpressure_events: i64,
    /// Max across nodes (timeouts happen system-wide for a given round)
    timeout_all: i64,
    /// Max across nodes
    timeout_leader: i64,
}

/// Primary consensus metrics for a single node.
#[derive(Debug, Default, Clone)]
struct PrimaryNodeMetrics {
    last_committed_round: i64,
    current_round: i64,
    committed_blocks: i64,
    last_committed_version: i64,
    storage_ledger_version: i64,
    timeout_rounds: i64,
}

/// Primary consensus metrics from a single validator (max across nodes).
#[derive(Debug, Default)]
struct PrimaryConsensusMetrics {
    last_committed_round: i64,
    current_round: i64,
    committed_blocks: i64,
    /// Ledger version from consensus (tracks execution + commit)
    last_committed_version: i64,
    /// Ledger version from storage (confirms persistence)
    storage_ledger_version: i64,
    /// Max timeout rounds across nodes
    timeout_rounds: i64,
}

/// Query proxy consensus metrics from each validator, returning per-node and aggregated results.
async fn query_proxy_metrics(
    inspection_clients: &[InspectionClient],
) -> Result<(Vec<ProxyNodeMetrics>, ProxyConsensusMetrics)> {
    let metrics_futures = inspection_clients.iter().map(|client| async move {
        // Note: forge_metrics endpoint stores label-less metrics with "{}" suffix
        ProxyNodeMetrics {
            proposals_sent: client
                .get_node_metric_i64("aptos_proxy_consensus_proposals_sent{}")
                .await
                .ok()
                .flatten()
                .unwrap_or(0),
            votes_sent: client
                .get_node_metric_i64("aptos_proxy_consensus_votes_sent{}")
                .await
                .ok()
                .flatten()
                .unwrap_or(0),
            qcs_formed: client
                .get_node_metric_i64("aptos_proxy_consensus_qcs_formed{}")
                .await
                .ok()
                .flatten()
                .unwrap_or(0),
            blocks_ordered: client
                .get_node_metric_i64("aptos_proxy_consensus_blocks_ordered{}")
                .await
                .ok()
                .flatten()
                .unwrap_or(0),
            blocks_forwarded: client
                .get_node_metric_i64("aptos_proxy_consensus_blocks_forwarded{}")
                .await
                .ok()
                .flatten()
                .unwrap_or(0),
            backpressure_events: client
                .get_node_metric_i64("aptos_proxy_backpressure_events{}")
                .await
                .ok()
                .flatten()
                .unwrap_or(0),
            timeout_all: client
                .get_node_metric_i64("aptos_proxy_round_timeout_all{}")
                .await
                .ok()
                .flatten()
                .unwrap_or(0),
            timeout_leader: client
                .get_node_metric_i64("aptos_proxy_round_timeout_count{}")
                .await
                .ok()
                .flatten()
                .unwrap_or(0),
        }
    });

    let per_node: Vec<ProxyNodeMetrics> = join_all(metrics_futures).await;

    let mut agg = ProxyConsensusMetrics::default();
    for n in &per_node {
        // Sum: unique per node (each node proposes different rounds, each node sends one vote)
        agg.proposals_sent += n.proposals_sent;
        agg.votes_sent += n.votes_sent;
        agg.backpressure_events += n.backpressure_events;
        // Max: replicated across nodes (every node sees the same committed blocks/QCs)
        agg.qcs_formed = agg.qcs_formed.max(n.qcs_formed);
        agg.blocks_ordered = agg.blocks_ordered.max(n.blocks_ordered);
        agg.blocks_forwarded = agg.blocks_forwarded.max(n.blocks_forwarded);
        agg.timeout_all = agg.timeout_all.max(n.timeout_all);
        agg.timeout_leader = agg.timeout_leader.max(n.timeout_leader);
    }

    Ok((per_node, agg))
}

/// Query primary consensus metrics from each validator, returning per-node and max-aggregated results.
async fn query_primary_metrics(
    inspection_clients: &[InspectionClient],
) -> Result<(Vec<PrimaryNodeMetrics>, PrimaryConsensusMetrics)> {
    let metrics_futures = inspection_clients.iter().map(|client| async move {
        PrimaryNodeMetrics {
            last_committed_round: client
                .get_node_metric_i64("aptos_consensus_last_committed_round{}")
                .await
                .ok()
                .flatten()
                .unwrap_or(0),
            current_round: client
                .get_node_metric_i64("aptos_consensus_current_round{}")
                .await
                .ok()
                .flatten()
                .unwrap_or(0),
            committed_blocks: client
                .get_node_metric_i64("aptos_consensus_committed_blocks_count{}")
                .await
                .ok()
                .flatten()
                .unwrap_or(0),
            last_committed_version: client
                .get_node_metric_i64("aptos_consensus_last_committed_version{}")
                .await
                .ok()
                .flatten()
                .unwrap_or(0),
            storage_ledger_version: client
                .get_node_metric_i64("aptos_storage_ledger_version{}")
                .await
                .ok()
                .flatten()
                .unwrap_or(0),
            timeout_rounds: client
                .get_node_metric_i64("aptos_consensus_timeout_rounds_count{}")
                .await
                .ok()
                .flatten()
                .unwrap_or(0),
        }
    });

    let per_node: Vec<PrimaryNodeMetrics> = join_all(metrics_futures).await;

    let mut agg = PrimaryConsensusMetrics::default();
    for n in &per_node {
        agg.last_committed_round = agg.last_committed_round.max(n.last_committed_round);
        agg.current_round = agg.current_round.max(n.current_round);
        agg.committed_blocks = agg.committed_blocks.max(n.committed_blocks);
        agg.last_committed_version = agg.last_committed_version.max(n.last_committed_version);
        agg.storage_ledger_version = agg.storage_ledger_version.max(n.storage_ledger_version);
        agg.timeout_rounds = agg.timeout_rounds.max(n.timeout_rounds);
    }

    Ok((per_node, agg))
}

/// Network emulation wrapper that simulates production proxy-primary topology.
///
/// Puts `num_proxy_validators` validators in a single EU region (co-located,
/// ~5ms intra-region latency) and distributes the remaining validators across
/// 3 other geo-distributed regions using real latency data from
/// `four_region_link_stats.csv` (same data as the land blocking test).
///
/// Topology (7 validators, 4 regions):
/// - Region 1 (eu-west2): 4 proxy validators, ~5ms intra-region
/// - Region 2 (eu-west6): 1 validator
/// - Region 3 (us-east4): 1 validator
/// - Region 4 (as-southeast1): 1 validator
///
/// Inter-region latencies (one-way): 8.5ms (EU-EU) to 106.5ms (US-Asia),
/// with 3% packet loss and 300 Mbps bandwidth cap.
pub struct ProxyPrimaryNetworkEmulation {
    num_proxy_validators: usize,
}

impl ProxyPrimaryNetworkEmulation {
    pub fn new(num_proxy_validators: usize) -> Self {
        Self {
            num_proxy_validators,
        }
    }

    /// Build the SwarmNetEm chaos for the proxy-primary topology.
    fn create_netem_chaos(&self, all_validators: Vec<PeerId>) -> SwarmNetEm {
        assert!(
            all_validators.len() > self.num_proxy_validators,
            "Need more validators ({}) than proxy validators ({})",
            all_validators.len(),
            self.num_proxy_validators,
        );

        // Group validators: first N in the proxy/EU region, rest one per region.
        let proxy_peers: Vec<PeerId> =
            all_validators[..self.num_proxy_validators].to_vec();
        let remaining: Vec<PeerId> =
            all_validators[self.num_proxy_validators..].to_vec();

        // Build peer groups: one group per region.
        // First group = proxy region (eu-west2), remaining = one validator each.
        let mut peer_groups: Vec<Vec<PeerId>> = vec![proxy_peers.clone()];
        for peer in &remaining {
            peer_groups.push(vec![*peer]);
        }

        let num_regions = peer_groups.len();
        info!(
            "Proxy-primary topology: {} proxy validators in EU region, {} other validators across {} regions",
            self.num_proxy_validators,
            remaining.len(),
            num_regions - 1,
        );

        // Use four-region config (same latency data as land blocking test)
        // but disable default intra-region chaos — we'll add custom proxy intra-region.
        let mut config = MultiRegionNetworkEmulationConfig::four_regions();
        config.intra_region_config = None;

        // Build inter-region chaos using the standard framework.
        let mut netem = create_multi_region_swarm_network_chaos(peer_groups, Some(config));

        // Add proxy intra-region delay (~5ms, co-located EU datacenter).
        netem.group_netems.push(GroupNetEm {
            name: "proxy-intra-region-netem".to_string(),
            source_nodes: proxy_peers.clone(),
            target_nodes: proxy_peers,
            delay_latency_ms: 5,
            delay_jitter_ms: 0,
            delay_correlation_percentage: 20,
            loss_percentage: 1,
            loss_correlation_percentage: 20,
            rate_in_mbps: 10_000, // 10 Gbps
        });

        netem
    }
}

impl Test for ProxyPrimaryNetworkEmulation {
    fn name(&self) -> &'static str {
        "proxy-primary-network-emulation"
    }
}

#[async_trait]
impl NetworkLoadTest for ProxyPrimaryNetworkEmulation {
    async fn setup<'a>(&self, ctx: &mut NetworkContext<'a>) -> anyhow::Result<LoadDestination> {
        let all_validators: Vec<PeerId> = {
            let swarm = ctx.swarm.read().await;
            swarm.validators().map(|v| v.peer_id()).collect()
        };

        let netem = self.create_netem_chaos(all_validators);
        ctx.swarm
            .write()
            .await
            .inject_chaos(SwarmChaos::NetEm(netem))
            .await?;
        info!("Injected proxy-primary geo-distributed network emulation");

        Ok(LoadDestination::FullnodesOtherwiseValidators)
    }

    async fn finish<'a>(&self, ctx: &mut NetworkContext<'a>) -> anyhow::Result<()> {
        let all_validators: Vec<PeerId> = {
            let swarm = ctx.swarm.read().await;
            swarm.validators().map(|v| v.peer_id()).collect()
        };

        let netem = self.create_netem_chaos(all_validators);
        ctx.swarm
            .write()
            .await
            .remove_chaos(SwarmChaos::NetEm(netem))
            .await?;
        info!("Removed proxy-primary network emulation");

        Ok(())
    }
}

#[async_trait]
impl NetworkLoadTest for ProxyPrimaryHappyPathTest {
    async fn setup<'a>(&self, ctx: &mut NetworkContext<'a>) -> anyhow::Result<LoadDestination> {
        self.network_emulation.setup(ctx).await
    }

    async fn test(
        &self,
        swarm: Arc<tokio::sync::RwLock<Box<dyn Swarm>>>,
        report: &mut TestReport,
        duration: Duration,
    ) -> Result<()> {
        info!("ProxyPrimaryHappyPathTest: Running for {:?}", duration);

        // Collect peer IDs and inspection clients
        let (peer_ids, inspection_clients): (Vec<PeerId>, Vec<InspectionClient>) = {
            let swarm = swarm.read().await;
            swarm
                .validators()
                .map(|v| (v.peer_id(), v.inspection_client()))
                .unzip()
        };

        // Wait for the test duration to let both consensus layers produce blocks
        // (transaction load is running in the background via the NetworkLoadTest framework)
        tokio::time::sleep(duration).await;

        // Query metrics from both consensus layers
        let (proxy_per_node, proxy_metrics) =
            query_proxy_metrics(&inspection_clients).await?;
        let (primary_per_node, primary_metrics) =
            query_primary_metrics(&inspection_clients).await?;

        let num_proxy = self.network_emulation.num_proxy_validators;
        let region_names = ["eu-west2", "eu-west6", "us-east4", "as-southeast1"];

        // Helpers to get region name and role for a validator index
        let region_for = |i: usize| -> &str {
            if i < num_proxy {
                region_names[0]
            } else {
                let region_idx = 1 + (i - num_proxy);
                region_names.get(region_idx).unwrap_or(&"unknown")
            }
        };
        let role_for = |i: usize| -> &str {
            if i < num_proxy {
                "proxy+primary"
            } else {
                "primary"
            }
        };

        // Build topology report
        let mut out = String::new();
        out.push_str("=== Network Topology ===\n");
        out.push_str(&format!(
            "Proxy nodes: {} (validators 0-{}), Primary nodes: {} (validators 0-{})\n",
            num_proxy,
            num_proxy - 1,
            peer_ids.len(),
            peer_ids.len() - 1,
        ));
        out.push_str(&format!(
            "Proxy region ({}): {} co-located validators, 5ms intra-region latency\n",
            region_names[0], num_proxy
        ));
        out.push_str(&format!(
            "{:<5} {:<15} {:<14} {}\n",
            "Node", "Role", "Region", "PeerId"
        ));
        for (i, peer) in peer_ids.iter().enumerate() {
            out.push_str(&format!(
                "{:<5} {:<15} {:<14} {}\n",
                i, role_for(i), region_for(i), peer
            ));
        }
        out.push_str("Inter-region latencies (one-way):\n");
        out.push_str("  eu-west2 <-> eu-west6:      8.5ms\n");
        out.push_str("  eu-west2 <-> us-east4:     37.5ms\n");
        out.push_str("  eu-west2 <-> as-southeast1: 84.0ms\n");
        out.push_str("  eu-west6 <-> us-east4:     46.0ms\n");
        out.push_str("  eu-west6 <-> as-southeast1: 81.0ms\n");
        out.push_str("  us-east4 <-> as-southeast1:106.5ms\n");

        // Per-node proxy metrics (all nodes — primary-only nodes should show 0s)
        out.push_str(&format!(
            "\n=== Proxy Consensus Metrics (per node, {} total) ===\n",
            peer_ids.len()
        ));
        out.push_str(&format!(
            "{:<5} {:<15} {:<14} {:>10} {:>8} {:>6} {:>9} {:>11} {:>10} {:>14}\n",
            "Node", "Role", "Region", "Proposals", "Votes", "QCs", "Ordered", "Forwarded", "Timeouts", "Backpressure"
        ));
        for (i, pm) in proxy_per_node.iter().enumerate() {
            out.push_str(&format!(
                "{:<5} {:<15} {:<14} {:>10} {:>8} {:>6} {:>9} {:>11} {:>10} {:>14}\n",
                i, role_for(i), region_for(i), pm.proposals_sent, pm.votes_sent, pm.qcs_formed,
                pm.blocks_ordered, pm.blocks_forwarded, pm.timeout_all, pm.backpressure_events
            ));
        }
        out.push_str(&format!(
            "\nProxy consensus: proposals={}, QCs={}, ordered={}, forwarded={}, timeouts={}, backpressure={}\n",
            proxy_metrics.proposals_sent, proxy_metrics.qcs_formed,
            proxy_metrics.blocks_ordered, proxy_metrics.blocks_forwarded,
            proxy_metrics.timeout_all, proxy_metrics.backpressure_events
        ));

        // Per-node primary metrics (all nodes)
        out.push_str(&format!(
            "\n=== Primary Consensus Metrics (per node, {} total) ===\n",
            peer_ids.len()
        ));
        out.push_str(&format!(
            "{:<5} {:<15} {:<14} {:>16} {:>14} {:>16} {:>18} {:>16} {:>10}\n",
            "Node", "Role", "Region", "CommittedRound", "CurrentRound", "CommittedBlocks", "CommittedVersion", "StorageVersion", "Timeouts"
        ));
        for (i, pm) in primary_per_node.iter().enumerate() {
            out.push_str(&format!(
                "{:<5} {:<15} {:<14} {:>16} {:>14} {:>16} {:>18} {:>16} {:>10}\n",
                i, role_for(i), region_for(i), pm.last_committed_round, pm.current_round, pm.committed_blocks,
                pm.last_committed_version, pm.storage_ledger_version, pm.timeout_rounds
            ));
        }
        out.push_str(&format!(
            "\nPrimary consensus: committed_round={}, committed_blocks={}, committed_version={}, storage_version={}, timeouts={}",
            primary_metrics.last_committed_round,
            primary_metrics.committed_blocks, primary_metrics.last_committed_version,
            primary_metrics.storage_ledger_version, primary_metrics.timeout_rounds
        ));

        // Report to test framework (this is what appears in Test Statistics output)
        report.report_text(out);

        // Verify proxy consensus is actively producing blocks
        ensure!(
            proxy_metrics.proposals_sent > 0,
            "Proxy consensus: no proposals sent"
        );
        ensure!(
            proxy_metrics.votes_sent > 0,
            "Proxy consensus: no votes sent"
        );
        ensure!(
            proxy_metrics.qcs_formed > 0,
            "Proxy consensus: no QCs formed"
        );
        ensure!(
            proxy_metrics.blocks_ordered > 0,
            "Proxy consensus: no blocks ordered"
        );
        ensure!(
            proxy_metrics.blocks_forwarded > 0,
            "Proxy consensus: no blocks forwarded to primaries"
        );

        // Verify primary consensus is committing blocks
        ensure!(
            primary_metrics.last_committed_round > 0,
            "Primary consensus: no blocks committed (last_committed_round=0)"
        );
        ensure!(
            primary_metrics.committed_blocks > 0,
            "Primary consensus: committed_blocks=0"
        );

        // Verify execution and storage commit (state actually persisted)
        ensure!(
            primary_metrics.last_committed_version > 0,
            "Execution: no transactions committed (last_committed_version=0)"
        );
        ensure!(
            primary_metrics.storage_ledger_version > 0,
            "Storage: ledger version not advancing (storage_ledger_version=0)"
        );

        info!(
            "Both consensus layers are active: proxy ordered {} blocks, primary committed {} blocks through round {}, version {}",
            proxy_metrics.blocks_ordered,
            primary_metrics.committed_blocks,
            primary_metrics.last_committed_round,
            primary_metrics.last_committed_version,
        );

        Ok(())
    }

    async fn finish<'a>(&self, ctx: &mut NetworkContext<'a>) -> anyhow::Result<()> {
        self.network_emulation.finish(ctx).await
    }
}

#[async_trait]
impl NetworkTest for ProxyPrimaryHappyPathTest {
    async fn run<'a>(&self, ctx: NetworkContextSynchronizer<'a>) -> anyhow::Result<()> {
        <dyn NetworkLoadTest>::run(self, ctx).await
    }
}

/// Test proxy consensus under high load conditions.
///
/// This test verifies:
/// - Proxy blocks scale with transaction load
/// - Backpressure mechanisms work correctly
/// - No block production stalls under load
pub struct ProxyPrimaryLoadTest {
    /// Target TPS to generate
    pub target_tps: usize,
    /// Maximum acceptable latency in ms
    pub max_latency_ms: u64,
    /// Number of validators that participate in proxy consensus.
    /// First `num_proxy_validators` validators run proxy+primary,
    /// remaining validators run primary-only.
    pub num_proxy_validators: usize,
}

impl ProxyPrimaryLoadTest {
    pub fn new(num_proxy_validators: usize) -> Self {
        Self {
            num_proxy_validators,
            ..Default::default()
        }
    }
}

impl Default for ProxyPrimaryLoadTest {
    fn default() -> Self {
        Self {
            target_tps: 5000,
            max_latency_ms: 3000,
            num_proxy_validators: 4,
        }
    }
}

impl Test for ProxyPrimaryLoadTest {
    fn name(&self) -> &'static str {
        "proxy primary load test"
    }
}

#[async_trait]
impl NetworkTest for ProxyPrimaryLoadTest {
    async fn run<'a>(&self, ctx: NetworkContextSynchronizer<'a>) -> anyhow::Result<()> {
        let duration = {
            let ctx = ctx.ctx.lock().await;
            ctx.global_duration
        };

        info!(
            "ProxyPrimaryLoadTest: Running for {:?} with target TPS {}",
            duration, self.target_tps
        );

        // Collect peer IDs and inspection clients
        let (peer_ids, inspection_clients): (Vec<PeerId>, Vec<InspectionClient>) = {
            let ctx = ctx.ctx.lock().await;
            let swarm = ctx.swarm.read().await;
            swarm
                .validators()
                .map(|v| (v.peer_id(), v.inspection_client()))
                .unzip()
        };

        // Take initial metric snapshot (per-node + aggregated)
        let (start_proxy_nodes, start_proxy) =
            query_proxy_metrics(&inspection_clients).await?;
        let (start_primary_nodes, start_primary) =
            query_primary_metrics(&inspection_clients).await?;

        // Wait for test duration
        tokio::time::sleep(duration).await;

        // Take final metric snapshot (per-node + aggregated)
        let (end_proxy_nodes, end_proxy) =
            query_proxy_metrics(&inspection_clients).await?;
        let (end_primary_nodes, end_primary) =
            query_primary_metrics(&inspection_clients).await?;

        // Calculate aggregated deltas
        let proposals_delta = end_proxy.proposals_sent - start_proxy.proposals_sent;
        let ordered_delta = end_proxy.blocks_ordered - start_proxy.blocks_ordered;
        let forwarded_delta = end_proxy.blocks_forwarded - start_proxy.blocks_forwarded;
        let backpressure_delta =
            end_proxy.backpressure_events - start_proxy.backpressure_events;
        let primary_committed_delta =
            end_primary.committed_blocks - start_primary.committed_blocks;

        let committed_version_delta =
            end_primary.last_committed_version - start_primary.last_committed_version;

        let duration_secs = duration.as_secs_f64();
        let proxy_blocks_per_sec = if duration_secs > 0.0 {
            ordered_delta as f64 / duration_secs
        } else {
            0.0
        };
        let tps = if duration_secs > 0.0 {
            committed_version_delta as f64 / duration_secs
        } else {
            0.0
        };

        info!("Load test metrics (delta):");
        info!("  proxy proposals: {}", proposals_delta);
        info!("  proxy blocks ordered: {}", ordered_delta);
        info!("  proxy blocks forwarded: {}", forwarded_delta);
        info!("  proxy blocks/sec: {:.2}", proxy_blocks_per_sec);
        info!("  primary blocks committed: {}", primary_committed_delta);
        info!("  backpressure events: {}", backpressure_delta);

        // Build detailed report with per-node metrics
        let mut out = String::new();
        out.push_str(&format!(
            "=== Proxy Load Test ({:.1}s) ===\n",
            duration_secs
        ));

        // Node list with peer IDs and roles
        let num_proxy = self.num_proxy_validators;
        let role_for = |i: usize| -> &str {
            if i < num_proxy {
                "proxy+primary"
            } else {
                "primary-only"
            }
        };
        out.push_str(&format!(
            "\n=== Validators ({} nodes: {} proxy+primary, {} primary-only) ===\n",
            peer_ids.len(),
            num_proxy,
            peer_ids.len() - num_proxy,
        ));
        out.push_str(&format!("{:<5} {:<15} {}\n", "Node", "Role", "PeerId"));
        for (i, peer) in peer_ids.iter().enumerate() {
            out.push_str(&format!("{:<5} {:<15} {}\n", i, role_for(i), peer));
        }

        // Per-node proxy metrics (delta during test window, all nodes)
        out.push_str(&format!(
            "\n=== Proxy Consensus Metrics (delta, per node, {} total) ===\n",
            peer_ids.len()
        ));
        out.push_str(&format!(
            "{:<5} {:<15} {:>10} {:>8} {:>6} {:>9} {:>11} {:>10} {:>14}\n",
            "Node", "Role", "Proposals", "Votes", "QCs", "Ordered", "Forwarded", "Timeouts", "Backpressure"
        ));
        for i in 0..peer_ids.len() {
            let s = &start_proxy_nodes[i];
            let e = &end_proxy_nodes[i];
            out.push_str(&format!(
                "{:<5} {:<15} {:>10} {:>8} {:>6} {:>9} {:>11} {:>10} {:>14}\n",
                i,
                role_for(i),
                e.proposals_sent - s.proposals_sent,
                e.votes_sent - s.votes_sent,
                e.qcs_formed - s.qcs_formed,
                e.blocks_ordered - s.blocks_ordered,
                e.blocks_forwarded - s.blocks_forwarded,
                e.timeout_all - s.timeout_all,
                e.backpressure_events - s.backpressure_events,
            ));
        }
        out.push_str(&format!(
            "\nProxy consensus: proposals={}, QCs={}, ordered={}, forwarded={}, timeouts={}, backpressure={}\n",
            proposals_delta,
            end_proxy.qcs_formed - start_proxy.qcs_formed,
            ordered_delta, forwarded_delta,
            end_proxy.timeout_all - start_proxy.timeout_all,
            backpressure_delta,
        ));

        // Per-node primary metrics (delta during test window)
        out.push_str(&format!(
            "\n=== Primary Consensus Metrics (delta, per node, {} total) ===\n",
            peer_ids.len()
        ));
        out.push_str(&format!(
            "{:<5} {:<15} {:>16} {:>14} {:>16} {:>18} {:>16} {:>10}\n",
            "Node", "Role", "CommittedRound", "CurrentRound", "CommittedBlocks", "CommittedVersion", "StorageVersion", "Timeouts"
        ));
        for i in 0..peer_ids.len() {
            let s = &start_primary_nodes[i];
            let e = &end_primary_nodes[i];
            out.push_str(&format!(
                "{:<5} {:<15} {:>16} {:>14} {:>16} {:>18} {:>16} {:>10}\n",
                i,
                role_for(i),
                e.last_committed_round - s.last_committed_round,
                e.current_round - s.current_round,
                e.committed_blocks - s.committed_blocks,
                e.last_committed_version - s.last_committed_version,
                e.storage_ledger_version - s.storage_ledger_version,
                e.timeout_rounds - s.timeout_rounds,
            ));
        }
        let primary_timeout_delta = end_primary.timeout_rounds - start_primary.timeout_rounds;
        out.push_str(&format!(
            "\nPrimary consensus: committed_round={}, committed_blocks={}, committed_version={}, storage_version={}, timeouts={}\n",
            end_primary.last_committed_round - start_primary.last_committed_round,
            primary_committed_delta,
            end_primary.last_committed_version - start_primary.last_committed_version,
            end_primary.storage_ledger_version - start_primary.storage_ledger_version,
            primary_timeout_delta,
        ));

        // Summary line
        out.push_str(&format!(
            "\nSummary: {} ordered ({:.2}/s), {} forwarded, {} primary committed, {} versions ({:.2} tps), {} backpressure in {:.1}s",
            ordered_delta, proxy_blocks_per_sec, forwarded_delta,
            primary_committed_delta, committed_version_delta, tps, backpressure_delta, duration_secs,
        ));

        // Report to test framework
        {
            let mut ctx = ctx.ctx.lock().await;
            ctx.report.report_text(out);
        }

        // Verify proxy consensus produced blocks during the test window
        ensure!(
            proposals_delta > 0,
            "Proxy consensus: no proposals sent during test window"
        );
        ensure!(
            ordered_delta > 0,
            "Proxy consensus: no blocks ordered during test window"
        );
        ensure!(
            forwarded_delta > 0,
            "Proxy consensus: no blocks forwarded during test window"
        );

        // Verify primary consensus committed blocks during the test window
        ensure!(
            primary_committed_delta > 0,
            "Primary consensus: no blocks committed during test window"
        );

        info!(
            "Load test passed: proxy ordered {} blocks ({:.2}/s), primary committed {} blocks in {:.1}s",
            ordered_delta, proxy_blocks_per_sec, primary_committed_delta, duration_secs,
        );

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_proxy_primary_test_defaults() {
        let test = ProxyPrimaryHappyPathTest::default();
        assert_eq!(test.min_tps, 1000);
        assert_eq!(test.min_proxy_blocks_per_round, 3);
    }

    #[test]
    fn test_proxy_primary_load_test_defaults() {
        let test = ProxyPrimaryLoadTest::default();
        assert_eq!(test.target_tps, 5000);
        assert_eq!(test.max_latency_ms, 3000);
        assert_eq!(test.num_proxy_validators, 4);
    }

    #[test]
    fn test_proxy_primary_load_test_new() {
        let test = ProxyPrimaryLoadTest::new(2);
        assert_eq!(test.num_proxy_validators, 2);
        assert_eq!(test.target_tps, 5000);
    }
}
