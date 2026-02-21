// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Forge tests for Proxy Primary Consensus.
//!
//! All transaction traffic is directed at proxy validators only, ensuring that blocks
//! must flow through the proxy consensus pipeline before reaching primary consensus.
//! This catches bugs where proxy block verification fails silently.

use crate::{
    create_buffered_load,
    multi_region_network_test::{
        IntraRegionNetEmConfig, InterRegionNetEmConfig, LinkStatsTableWithPeerGroups,
    },
    LoadDestination, NetworkLoadTest, COOLDOWN_DURATION_FRACTION, WARMUP_DURATION_FRACTION,
};
use anyhow::ensure;
use aptos_forge::{
    success_criteria::{SuccessCriteria, SuccessCriteriaChecker},
    EmitJobRequest, NetworkContext, NetworkContextSynchronizer, NetworkTest, Result, Swarm,
    SwarmChaos, SwarmNetEm, Test, TestReport,
};
use aptos_types::PeerId;
use async_trait::async_trait;
use log::info;
use std::{collections::BTreeMap, sync::Arc, time::Duration};

// ---------------------------------------------------------------------------
// ProxyPrimaryNetworkEmulation
// ---------------------------------------------------------------------------

/// Network emulation that co-locates proxy validators in one region (low latency)
/// and geo-distributes non-proxy validators across distant regions.
pub struct ProxyPrimaryNetworkEmulation {
    num_proxy_validators: usize,
}

impl ProxyPrimaryNetworkEmulation {
    pub fn new(num_proxy_validators: usize) -> Self {
        Self {
            num_proxy_validators,
        }
    }

    /// Build a custom link stats table with 4 regions:
    ///   Region 1 (eu-west2):      proxy validators — co-located
    ///   Region 2 (us-east4):      75ms RTT to proxy region
    ///   Region 3 (as-northeast1): 130ms RTT to proxy region
    ///   Region 4 (as-southeast1): 168ms RTT to proxy region
    fn build_link_stats_table() -> BTreeMap<String, BTreeMap<String, (u64, f64)>> {
        let bandwidth: u64 = 300_000_000; // 300 Mbps
        let regions = [
            "1-gcp--eu-west2",
            "2-gcp--us-east4",
            "3-gcp--as-northeast1",
            "4-gcp--as-southeast1",
        ];
        // RTT matrix (symmetric). Only upper-triangle needed; we fill both directions.
        let rtts: &[(&str, &str, f64)] = &[
            (regions[0], regions[1], 75.0),  // eu ↔ us-east
            (regions[0], regions[2], 130.0), // eu ↔ as-northeast
            (regions[0], regions[3], 168.0), // eu ↔ as-southeast
            (regions[1], regions[2], 170.0), // us-east ↔ as-northeast
            (regions[1], regions[3], 213.0), // us-east ↔ as-southeast
            (regions[2], regions[3], 70.0),  // as-northeast ↔ as-southeast
        ];

        let mut table: BTreeMap<String, BTreeMap<String, (u64, f64)>> = BTreeMap::new();
        for &(from, to, rtt) in rtts {
            table
                .entry(from.to_string())
                .or_default()
                .insert(to.to_string(), (bandwidth, rtt));
            table
                .entry(to.to_string())
                .or_default()
                .insert(from.to_string(), (bandwidth, rtt));
        }
        table
    }

    async fn create_netem_chaos(
        &self,
        swarm: Arc<tokio::sync::RwLock<Box<dyn Swarm>>>,
    ) -> SwarmNetEm {
        let all_validators: Vec<PeerId> = {
            let s = swarm.read().await;
            s.validators().map(|v| v.peer_id()).collect()
        };

        let link_stats_table = Self::build_link_stats_table();
        let regions: Vec<String> = link_stats_table.keys().cloned().collect();

        // Region 1: all proxy validators (co-located)
        let proxy_peers: Vec<PeerId> = all_validators[..self.num_proxy_validators].to_vec();

        // Regions 2-N: one non-proxy validator each
        let non_proxy_peers: Vec<PeerId> = all_validators[self.num_proxy_validators..].to_vec();

        let mut peer_groups: LinkStatsTableWithPeerGroups = Vec::new();
        // First region gets all proxy validators
        peer_groups.push((
            regions[0].clone(),
            proxy_peers,
            link_stats_table[&regions[0]].clone(),
        ));
        // Remaining regions get one non-proxy validator each
        for (i, peer) in non_proxy_peers.into_iter().enumerate() {
            let region_idx = i + 1;
            if region_idx < regions.len() {
                peer_groups.push((
                    regions[region_idx].clone(),
                    vec![peer],
                    link_stats_table[&regions[region_idx]].clone(),
                ));
            }
        }

        // Build inter-region chaos (uses default 3% loss, 50% correlation)
        let inter_region_config = InterRegionNetEmConfig::default();
        let inter_region_netem = inter_region_config.build(&peer_groups);

        // Build intra-region chaos with 5ms latency for proxy co-location
        let intra_region_config = IntraRegionNetEmConfig {
            bandwidth_rate_mbps: 10_000, // 10 Gbps
            delay_latency_ms: 5,
            delay_jitter_ms: 0,
            delay_correlation_percentage: 20,
            loss_percentage: 0,
            loss_correlation_percentage: 0,
        };
        let intra_region_netem = intra_region_config.build(peer_groups);

        SwarmNetEm {
            group_netems: itertools::concat(vec![intra_region_netem, inter_region_netem]),
        }
    }
}

impl Test for ProxyPrimaryNetworkEmulation {
    fn name(&self) -> &'static str {
        "network:proxy-primary-network-emulation"
    }
}

#[async_trait]
impl NetworkLoadTest for ProxyPrimaryNetworkEmulation {
    async fn setup<'a>(&self, ctx: &mut NetworkContext<'a>) -> Result<LoadDestination> {
        let chaos = self.create_netem_chaos(ctx.swarm.clone()).await;
        ctx.swarm
            .write()
            .await
            .inject_chaos(SwarmChaos::NetEm(chaos))
            .await?;
        Ok(LoadDestination::AllValidators)
    }

    async fn finish<'a>(&self, ctx: &mut NetworkContext<'a>) -> Result<()> {
        let chaos = self.create_netem_chaos(ctx.swarm.clone()).await;
        ctx.swarm
            .write()
            .await
            .remove_chaos(SwarmChaos::NetEm(chaos))
            .await?;
        Ok(())
    }
}

#[async_trait]
impl NetworkTest for ProxyPrimaryNetworkEmulation {
    async fn run<'a>(&self, ctx: NetworkContextSynchronizer<'a>) -> Result<()> {
        <dyn NetworkLoadTest>::run(self, ctx).await
    }
}

// ---------------------------------------------------------------------------
// ProxyPrimaryTrafficTest
// ---------------------------------------------------------------------------

/// Traffic test that sends all transactions to proxy validators only.
///
/// After the traffic phase, queries Prometheus for proxy and primary consensus
/// metrics and verifies that proxy blocks were consumed by primary.
pub struct ProxyPrimaryTrafficTest {
    /// Number of proxy validators (first N validators by index).
    pub num_proxy_validators: usize,
    /// Inner traffic configuration (MaxLoad).
    pub inner_traffic: EmitJobRequest,
    /// Success criteria for inner traffic TPS.
    pub inner_success_criteria: SuccessCriteria,
}

impl Test for ProxyPrimaryTrafficTest {
    fn name(&self) -> &'static str {
        "proxy primary traffic test"
    }
}

impl ProxyPrimaryTrafficTest {
    async fn get_proxy_peer_ids(
        &self,
        swarm: &Arc<tokio::sync::RwLock<Box<dyn Swarm>>>,
    ) -> Vec<PeerId> {
        let s = swarm.read().await;
        s.validators()
            .map(|v| v.peer_id())
            .take(self.num_proxy_validators)
            .collect()
    }

    /// Query and report proxy/primary consensus metrics separately.
    /// Gracefully skips if Prometheus is not available (local mode).
    async fn report_metrics(
        &self,
        swarm: &Arc<tokio::sync::RwLock<Box<dyn Swarm>>>,
        report: &mut TestReport,
    ) {
        let s = swarm.read().await;

        let proxy_metrics = [
            ("aptos_proxy_consensus_proposals_sent", "Proxy Proposals Sent"),
            ("aptos_proxy_consensus_votes_sent", "Proxy Votes Sent"),
            ("aptos_proxy_consensus_qcs_formed", "Proxy QCs Formed"),
            ("aptos_proxy_consensus_blocks_ordered", "Proxy Blocks Ordered"),
            ("aptos_proxy_consensus_blocks_forwarded", "Proxy Blocks Forwarded"),
        ];

        let primary_metrics = [
            ("aptos_consensus_last_committed_round", "Primary Last Committed Round"),
            ("aptos_consensus_committed_blocks_count", "Primary Committed Blocks"),
            ("aptos_consensus_last_committed_version", "Primary Last Committed Version"),
            (
                "aptos_consensus_proxy_blocks_verified_by_primary",
                "Proxy Blocks Verified by Primary",
            ),
        ];

        let mut out = String::new();
        out.push_str("=== Proxy Consensus Metrics ===\n");
        for (metric_name, label) in &proxy_metrics {
            let query = format!("sum({})", metric_name);
            if let Ok(result) = s.query_metrics(&query, None, None).await {
                if let Some(samples) = result.as_instant() {
                    for iv in samples {
                        out.push_str(&format!("  {}: {}\n", label, iv.sample().value()));
                    }
                }
            }
        }
        out.push_str("=== Primary Consensus Metrics ===\n");
        for (metric_name, label) in &primary_metrics {
            let query = format!("max({})", metric_name);
            if let Ok(result) = s.query_metrics(&query, None, None).await {
                if let Some(samples) = result.as_instant() {
                    for iv in samples {
                        out.push_str(&format!("  {}: {}\n", label, iv.sample().value()));
                    }
                }
            }
        }
        info!("{}", out);
        report.report_text(out);
    }

    /// Verify that the PROXY_BLOCKS_VERIFIED_BY_PRIMARY counter is > 0.
    /// Only asserts in remote mode (where Prometheus is available).
    async fn verify_proxy_blocks_consumed(
        &self,
        swarm: &Arc<tokio::sync::RwLock<Box<dyn Swarm>>>,
    ) -> Result<()> {
        let s = swarm.read().await;
        let query = "sum(aptos_consensus_proxy_blocks_verified_by_primary)";
        match s.query_metrics(query, None, None).await {
            Ok(result) => {
                let samples = result.as_instant().unwrap_or(&[]);
                let total: f64 = samples.iter().map(|iv| iv.sample().value()).sum();
                info!("Proxy blocks verified by primary (total): {}", total);
                ensure!(
                    total > 0.0,
                    "PROXY_BLOCKS_VERIFIED_BY_PRIMARY counter is 0: \
                     proxy blocks were not consumed by primary consensus!"
                );
            },
            Err(e) => {
                // Local mode: Prometheus not available, skip assertion
                info!(
                    "Skipping PROXY_BLOCKS_VERIFIED_BY_PRIMARY check \
                     (Prometheus not available): {}",
                    e
                );
            },
        }
        Ok(())
    }
}

#[async_trait]
impl NetworkLoadTest for ProxyPrimaryTrafficTest {
    async fn setup<'a>(&self, ctx: &mut NetworkContext<'a>) -> Result<LoadDestination> {
        let proxy_peer_ids = self.get_proxy_peer_ids(&ctx.swarm).await;
        info!(
            "Routing outer traffic to {} proxy validators: {:?}",
            proxy_peer_ids.len(),
            proxy_peer_ids
        );
        Ok(LoadDestination::Peers(proxy_peer_ids))
    }

    async fn test(
        &self,
        swarm: Arc<tokio::sync::RwLock<Box<dyn Swarm>>>,
        report: &mut TestReport,
        duration: Duration,
    ) -> Result<()> {
        let proxy_peer_ids = self.get_proxy_peer_ids(&swarm).await;
        info!(
            "Sending inner traffic to {} proxy validators: {:?}",
            proxy_peer_ids.len(),
            proxy_peer_ids
        );

        let stats_by_phase = create_buffered_load(
            swarm.clone(),
            &proxy_peer_ids,
            self.inner_traffic.clone(),
            duration,
            WARMUP_DURATION_FRACTION,
            COOLDOWN_DURATION_FRACTION,
            None,
            None,
        )
        .await?;

        for phase_stats in &stats_by_phase {
            report.report_txn_stats(
                format!("{}: proxy traffic", self.name()),
                &phase_stats.emitter_stats,
            );
            SuccessCriteriaChecker::check_core_for_success(
                &self.inner_success_criteria,
                report,
                &phase_stats.emitter_stats.rate(),
                None,
                Some("proxy traffic".to_string()),
            )?;
        }

        // Collect and report metrics
        self.report_metrics(&swarm, report).await;

        // Verify proxy blocks were consumed by primary
        self.verify_proxy_blocks_consumed(&swarm).await?;

        Ok(())
    }
}

#[async_trait]
impl NetworkTest for ProxyPrimaryTrafficTest {
    async fn run<'a>(&self, ctx: NetworkContextSynchronizer<'a>) -> Result<()> {
        <dyn NetworkLoadTest>::run(self, ctx).await
    }
}
