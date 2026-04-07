// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use crate::{LoadDestination, NetworkLoadTest};
use aptos_forge::{
    GroupNetEm, NetworkContext, NetworkContextSynchronizer, NetworkTest, Swarm, SwarmChaos,
    SwarmNetEm, Test,
};
use aptos_types::PeerId;
use async_trait::async_trait;
use itertools::{self, EitherOrBoth, Itertools};
use log::info;
use std::{collections::BTreeMap, sync::Arc};

/// The link stats are obtained from https://github.com/doitintl/intercloud-throughput/blob/master/results_202202/results.csv
/// The four regions were hand-picked from the dataset to simulate a multi-region setup
/// with high latencies.
/// Note, we restrict bandwidth to 300 Mbps between all regions. The reasoning is that the dataset
/// is measuring TCP bandwidth only which is primarily affected by RTT, and not the actual bandwidth
/// across the regions, which would vary according to competing traffic, etc.
const FOUR_REGION_LINK_STATS: &[u8] = include_bytes!("data/four_region_link_stats.csv");
const SIX_REGION_LINK_STATS: &[u8] = include_bytes!("data/six_region_link_stats.csv");
/// The two regions were chosen as the most distant regions among the four regions set.
const TWO_REGION_LINK_STATS: &[u8] = include_bytes!("data/two_region_link_stats.csv");

fn get_link_stats_table(csv: &[u8]) -> BTreeMap<String, BTreeMap<String, (u64, f64)>> {
    let mut stats_table = BTreeMap::new();

    let mut rdr = csv::Reader::from_reader(csv);
    rdr.deserialize()
        .for_each(|result: Result<(String, String, u64, f64), _>| {
            if let Ok((from, to, bitrate, latency)) = result {
                stats_table
                    .entry(from)
                    .or_insert_with(BTreeMap::new)
                    .insert(to, (bitrate, latency));
            }
        });
    stats_table
}

fn div_ceil(dividend: usize, divisor: usize) -> usize {
    if dividend.is_multiple_of(divisor) {
        dividend / divisor
    } else {
        dividend / divisor + 1
    }
}

/// Chunks the given set of peers into groups according to the specified weights.
/// Weights are fractional values (e.g., [0.50, 0.20, 0.15, 0.15]) that must sum to ~1.0.
/// Uses the largest-remainder method to ensure chunk sizes sum exactly to the total peer count.
pub(crate) fn weighted_chunk_peers(
    mut peers: Vec<Vec<PeerId>>,
    weights: &[f64],
) -> Vec<Vec<PeerId>> {
    let total = peers.len();
    let weight_sum: f64 = weights.iter().sum();
    assert!(
        (weight_sum - 1.0).abs() < 0.01,
        "Weights must sum to approximately 1.0, got {}",
        weight_sum
    );

    // Compute initial sizes using floor, track remainders
    let mut sizes: Vec<usize> = weights
        .iter()
        .map(|w| (w * total as f64).floor() as usize)
        .collect();
    let mut remainders: Vec<(usize, f64)> = weights
        .iter()
        .enumerate()
        .map(|(i, w)| (i, (w * total as f64).fract()))
        .collect();

    // Distribute remaining slots to regions with largest fractional remainders
    let allocated: usize = sizes.iter().sum();
    let mut remaining = total.saturating_sub(allocated);
    remainders.sort_by(|a, b| b.1.total_cmp(&a.1));
    for (idx, _) in &remainders {
        if remaining == 0 {
            break;
        }
        sizes[*idx] += 1;
        remaining -= 1;
    }

    // Split peers into chunks of computed sizes
    let mut chunks = Vec::with_capacity(weights.len());
    for size in sizes {
        let rest = peers.split_off(size);
        chunks.push(peers.iter().flatten().cloned().collect());
        peers = rest;
    }
    chunks
}

/// Chunks the given set of peers into the specified number of chunks. The difference between the
/// largest chunk and smallest chunk is at most one.
pub(crate) fn chunk_peers(mut peers: Vec<Vec<PeerId>>, num_chunks: usize) -> Vec<Vec<PeerId>> {
    let mut chunks = vec![];
    let mut chunks_remaining = num_chunks;
    while chunks_remaining > 0 {
        let chunk_size = div_ceil(peers.len(), chunks_remaining);
        let remaining = peers.split_off(chunk_size);
        chunks.push(peers.iter().flatten().cloned().collect());
        peers = remaining;

        chunks_remaining -= 1;
    }
    chunks
}

/// Creates a table of peers grouped by region. The peers are divided into N groups, where N is the
/// number of regions provided in the link stats table. Any remaining peers are added to the first
/// group.
fn create_link_stats_table_with_peer_groups(
    peers: Vec<Vec<PeerId>>,
    link_stats_table: &LinkStatsTable,
    region_weights: Option<&[f64]>,
) -> LinkStatsTableWithPeerGroups {
    // Verify that we have enough grouped peers to simulate the link stats table
    assert!(peers.len() >= link_stats_table.len());

    // Verify that we have the correct number of regions to simulate the link stats table
    let number_of_regions = link_stats_table.len();
    assert!(
        number_of_regions >= 2,
        "At least 2 regions are required for inter-region network chaos."
    );
    assert!(
        number_of_regions <= 6,
        "ChaosMesh only supports simulating up to 6 regions."
    );

    // Create the link stats table with peer groups
    let peer_chunks = match region_weights {
        Some(weights) => {
            assert_eq!(
                weights.len(),
                number_of_regions,
                "Number of weights ({}) must match number of regions ({})",
                weights.len(),
                number_of_regions
            );
            weighted_chunk_peers(peers, weights)
        },
        None => chunk_peers(peers, number_of_regions),
    };
    peer_chunks
        .into_iter()
        .zip(link_stats_table.iter())
        .map(|(chunk, (from_region, stats))| (from_region.clone(), chunk, stats.clone()))
        .collect()
}

// A map of "source" regions to a map of "destination" region to (bandwidth, latency)
type LinkStatsTable = BTreeMap<String, BTreeMap<String, (u64, f64)>>;
// A map of "source" regions to a tuple of (list of peers, map of "destination" region to (bandwidth, latency))
type LinkStatsTableWithPeerGroups = Vec<(String, Vec<PeerId>, BTreeMap<String, (u64, f64)>)>;

#[derive(Clone)]
pub struct InterRegionNetEmConfig {
    delay_jitter_ms: u64,
    delay_correlation_percentage: u64,
    loss_percentage: u64,
    loss_correlation_percentage: u64,
}

impl Default for InterRegionNetEmConfig {
    fn default() -> Self {
        Self {
            delay_jitter_ms: 0,
            delay_correlation_percentage: 50,
            loss_percentage: 3,
            loss_correlation_percentage: 50,
        }
    }
}

impl InterRegionNetEmConfig {
    /// Configuration calibrated to approximate mainnet conditions.
    /// Reduces packet loss from 3% to 1% (real cloud is 0.01-0.1%).
    pub fn mainnet_calibrated() -> Self {
        Self {
            delay_jitter_ms: 0,
            delay_correlation_percentage: 50,
            loss_percentage: 1,
            loss_correlation_percentage: 50,
        }
    }

    // Creates GroupNetEm for inter-region network chaos
    fn build(&self, peer_groups: &LinkStatsTableWithPeerGroups) -> Vec<GroupNetEm> {
        let group_netems: Vec<GroupNetEm> = peer_groups
            .iter()
            .combinations(2)
            .flat_map(|comb| {
                let (from_region, from_chunk, stats) = &comb[0];
                let (to_region, to_chunk, _) = &comb[1];

                let (bandwidth, rtt_latency) = stats.get(to_region).unwrap();
                let hop_latency = rtt_latency / 2.0;
                let netems = [
                    GroupNetEm {
                        name: format!("{}-to-{}-netem", from_region, to_region),
                        source_nodes: from_chunk.to_vec(),
                        target_nodes: to_chunk.to_vec(),
                        delay_latency_ms: hop_latency as u64,
                        delay_jitter_ms: self.delay_jitter_ms,
                        delay_correlation_percentage: self.delay_correlation_percentage,
                        loss_percentage: self.loss_percentage,
                        loss_correlation_percentage: self.loss_correlation_percentage,
                        rate_in_mbps: *bandwidth / 1e6 as u64,
                    },
                    GroupNetEm {
                        name: format!("{}-to-{}-netem", to_region, from_region),
                        source_nodes: to_chunk.to_vec(),
                        target_nodes: from_chunk.to_vec(),
                        delay_latency_ms: hop_latency as u64,
                        delay_jitter_ms: self.delay_jitter_ms,
                        delay_correlation_percentage: self.delay_correlation_percentage,
                        loss_percentage: self.loss_percentage,
                        loss_correlation_percentage: self.loss_correlation_percentage,
                        rate_in_mbps: *bandwidth / 1e6 as u64,
                    },
                ];
                info!("inter-region netem {:?}", netems);

                netems
            })
            .collect();

        group_netems
    }
}

#[derive(Clone)]
pub struct IntraRegionNetEmConfig {
    bandwidth_rate_mbps: u64,
    delay_latency_ms: u64,
    delay_jitter_ms: u64,
    delay_correlation_percentage: u64,
    loss_percentage: u64,
    loss_correlation_percentage: u64,
}

impl Default for IntraRegionNetEmConfig {
    fn default() -> Self {
        Self {
            bandwidth_rate_mbps: 10 * 1000, // 10 Gbps
            delay_latency_ms: 20,
            delay_jitter_ms: 0,
            delay_correlation_percentage: 20,
            loss_percentage: 1,
            loss_correlation_percentage: 20,
        }
    }
}

impl IntraRegionNetEmConfig {
    /// Configuration calibrated to approximate mainnet conditions.
    /// Keeps latency at 20ms and reduces loss from 1% to 0%.
    pub fn mainnet_calibrated() -> Self {
        Self {
            bandwidth_rate_mbps: 10 * 1000, // 10 Gbps
            delay_latency_ms: 20,
            delay_jitter_ms: 0,
            delay_correlation_percentage: 20,
            loss_percentage: 0,
            loss_correlation_percentage: 20,
        }
    }

    fn build(&self, peer_groups: LinkStatsTableWithPeerGroups) -> Vec<GroupNetEm> {
        let group_netems: Vec<GroupNetEm> = peer_groups
            .iter()
            .map(|(region, chunk, _)| {
                let netem = GroupNetEm {
                    name: format!("{}-self-netem", region),
                    source_nodes: chunk.to_vec(),
                    target_nodes: chunk.to_vec(),
                    delay_latency_ms: self.delay_latency_ms,
                    delay_jitter_ms: self.delay_jitter_ms,
                    delay_correlation_percentage: self.delay_correlation_percentage,
                    loss_percentage: self.loss_percentage,
                    loss_correlation_percentage: self.loss_correlation_percentage,
                    rate_in_mbps: self.bandwidth_rate_mbps,
                };
                info!("intra-region netem {:?}", netem);

                netem
            })
            .collect();

        group_netems
    }
}

#[derive(Clone)]
pub struct MultiRegionNetworkEmulationConfig {
    pub link_stats_table: LinkStatsTable,
    pub inter_region_config: InterRegionNetEmConfig,
    pub intra_region_config: Option<IntraRegionNetEmConfig>,
    /// Optional per-region weights for skewed validator placement.
    /// Order corresponds to BTreeMap key order (alphabetical) of link_stats_table.
    /// None means equal distribution across regions.
    pub region_weights: Option<Vec<f64>>,
}

impl Default for MultiRegionNetworkEmulationConfig {
    fn default() -> Self {
        Self {
            link_stats_table: get_link_stats_table(FOUR_REGION_LINK_STATS),
            inter_region_config: InterRegionNetEmConfig::default(),
            intra_region_config: Some(IntraRegionNetEmConfig::default()),
            region_weights: None,
        }
    }
}

impl MultiRegionNetworkEmulationConfig {
    pub fn two_region() -> Self {
        Self {
            link_stats_table: get_link_stats_table(TWO_REGION_LINK_STATS),
            ..Default::default()
        }
    }

    pub fn four_regions() -> Self {
        Self {
            link_stats_table: get_link_stats_table(FOUR_REGION_LINK_STATS),
            ..Default::default()
        }
    }

    pub fn six_regions() -> Self {
        Self {
            link_stats_table: get_link_stats_table(SIX_REGION_LINK_STATS),
            ..Default::default()
        }
    }

    /// Configuration calibrated to approximate mainnet conditions:
    /// - EU-heavy validator placement (~71% in EU: 43% eu-west2 + 28% eu-west6)
    /// - Quorum (67%) can form within EU for validator counts >= 7
    /// - Reduced packet loss
    /// Regions in BTreeMap order: 1-gcp--eu-west2, 2-gcp--eu-west6, 3-gcp--us-east4, 4-gcp--as-southeast1
    pub fn mainnet_calibrated() -> Self {
        Self {
            link_stats_table: get_link_stats_table(FOUR_REGION_LINK_STATS),
            inter_region_config: InterRegionNetEmConfig::mainnet_calibrated(),
            intra_region_config: Some(IntraRegionNetEmConfig::mainnet_calibrated()),
            region_weights: Some(vec![0.43, 0.28, 0.15, 0.14]),
        }
    }

    /// Six-region configuration calibrated to approximate mainnet conditions:
    /// - EU-heavy validator placement (~71% in EU: 36% eu-central1 + 35% eu-west-1)
    /// - Reduced packet loss and intra-region latency
    /// Regions in BTreeMap order: aws--ap-northeast-1, aws--eu-central1, aws--eu-west-1,
    ///   aws--sa-east-1, gcp--ca-central-1, gcp--us-central1
    pub fn mainnet_calibrated_six_regions() -> Self {
        Self {
            link_stats_table: get_link_stats_table(SIX_REGION_LINK_STATS),
            inter_region_config: InterRegionNetEmConfig::mainnet_calibrated(),
            intra_region_config: Some(IntraRegionNetEmConfig::mainnet_calibrated()),
            region_weights: Some(vec![0.07, 0.36, 0.35, 0.04, 0.07, 0.11]),
        }
    }
}

/// A test to emulate network conditions for a multi-region setup.
pub struct MultiRegionNetworkEmulationTest {
    network_emulation_config: MultiRegionNetworkEmulationConfig,
}

impl MultiRegionNetworkEmulationTest {
    pub fn new_with_config(network_emulation_config: MultiRegionNetworkEmulationConfig) -> Self {
        Self {
            network_emulation_config,
        }
    }

    pub fn mainnet_calibrated_for_validator_count(num_validators: usize) -> Self {
        if num_validators > 100 {
            Self {
                network_emulation_config:
                    MultiRegionNetworkEmulationConfig::mainnet_calibrated_six_regions(),
            }
        } else {
            Self {
                network_emulation_config: MultiRegionNetworkEmulationConfig::mainnet_calibrated(),
            }
        }
    }

    /// Creates a new SwarmNetEm to be injected via chaos. Note: network
    /// emulation is only done for the validators in the swarm (and not
    /// the fullnodes).
    async fn create_netem_chaos(
        &self,
        swarm: Arc<tokio::sync::RwLock<Box<dyn Swarm>>>,
    ) -> SwarmNetEm {
        let (all_validators, all_vfns) = {
            let swarm = swarm.read().await;
            let all_validators = swarm.validators().map(|v| v.peer_id()).collect::<Vec<_>>();
            let all_vfns = swarm.full_nodes().map(|v| v.peer_id()).collect::<Vec<_>>();
            (all_validators, all_vfns)
        };

        let all_pairs: Vec<_> = all_validators
            .iter()
            .zip_longest(all_vfns)
            .map(|either_or_both| match either_or_both {
                EitherOrBoth::Both(validator, vfn) => vec![*validator, vfn],
                EitherOrBoth::Left(validator) => vec![*validator],
                EitherOrBoth::Right(_) => {
                    panic!("Number of validators must be >= number of VFNs")
                },
            })
            .collect();

        let network_emulation_config = self.network_emulation_config.clone();
        create_multi_region_swarm_network_chaos(all_pairs, Some(network_emulation_config))
    }
}

impl Test for MultiRegionNetworkEmulationTest {
    fn name(&self) -> &'static str {
        "network:multi-region-network-emulation"
    }
}

/// Creates a SwarmNetEm to be injected via chaos. Network emulation is added to all the given
/// peers using the specified config. Peers that must be colocated should be grouped in the same
/// inner vector. They are treated as a single group.
pub fn create_multi_region_swarm_network_chaos(
    all_peers: Vec<Vec<PeerId>>,
    network_emulation_config: Option<MultiRegionNetworkEmulationConfig>,
) -> SwarmNetEm {
    // Determine the network emulation config to use
    let network_emulation_config = network_emulation_config.unwrap_or_default();

    // Create the link stats table for the peer groups
    let peer_groups = create_link_stats_table_with_peer_groups(
        all_peers,
        &network_emulation_config.link_stats_table,
        network_emulation_config.region_weights.as_deref(),
    );

    // Create the inter and intra network emulation configs
    let inter_region_netem = network_emulation_config
        .inter_region_config
        .build(&peer_groups);
    let intra_region_netem = network_emulation_config
        .intra_region_config
        .as_ref()
        .map(|config| config.build(peer_groups))
        .unwrap_or_default();

    SwarmNetEm {
        group_netems: itertools::concat(vec![intra_region_netem, inter_region_netem]),
    }
}

#[async_trait]
impl NetworkLoadTest for MultiRegionNetworkEmulationTest {
    async fn setup<'a>(&self, ctx: &mut NetworkContext<'a>) -> anyhow::Result<LoadDestination> {
        let chaos = self.create_netem_chaos(ctx.swarm.clone()).await;
        ctx.swarm
            .write()
            .await
            .inject_chaos(SwarmChaos::NetEm(chaos))
            .await?;

        Ok(LoadDestination::FullnodesOtherwiseValidators)
    }

    async fn finish<'a>(&self, ctx: &mut NetworkContext<'a>) -> anyhow::Result<()> {
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
impl NetworkTest for MultiRegionNetworkEmulationTest {
    async fn run<'a>(&self, ctx: NetworkContextSynchronizer<'a>) -> anyhow::Result<()> {
        <dyn NetworkLoadTest>::run(self, ctx).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use aptos_types::account_address::AccountAddress;
    use std::vec;

    #[test]
    fn test_create_multi_region_swarm_network_chaos() {
        aptos_logger::Logger::new().init();

        // Create a config with 8 peers and multiple regions
        let all_peers: Vec<_> = (0..8).map(|_| vec![PeerId::random()]).collect();
        let netem = create_multi_region_swarm_network_chaos(all_peers, None);

        // Verify the number of group netems
        assert_eq!(netem.group_netems.len(), 10);

        // Create a config with 10 peers and multiple regions
        let all_peers: Vec<_> = (0..10).map(|_| vec![PeerId::random()]).collect();
        let netem = create_multi_region_swarm_network_chaos(all_peers.clone(), None);

        // Verify the resulting group netems
        assert_eq!(netem.group_netems.len(), 10);
        assert_eq!(netem.group_netems[0].source_nodes.len(), 4);
        assert_eq!(netem.group_netems[0].target_nodes.len(), 4);
        assert_eq!(netem.group_netems[0], GroupNetEm {
            name: "aws--ap-northeast-1-self-netem".to_owned(),
            rate_in_mbps: 10000,
            source_nodes: vec![
                all_peers[0][0],
                all_peers[1][0],
                all_peers[8][0],
                all_peers[9][0],
            ],
            target_nodes: vec![
                all_peers[0][0],
                all_peers[1][0],
                all_peers[8][0],
                all_peers[9][0],
            ],
            delay_latency_ms: 50,
            delay_jitter_ms: 5,
            delay_correlation_percentage: 50,
            loss_percentage: 1,
            loss_correlation_percentage: 50
        })
    }

    #[test]
    fn test_weighted_chunk_peers() {
        // 20 peers with EU-heavy weights
        let peers: Vec<_> = (0..20).map(|_| vec![AccountAddress::random()]).collect();
        let chunks = weighted_chunk_peers(peers, &[0.50, 0.17, 0.17, 0.16]);
        assert_eq!(chunks.len(), 4);
        assert_eq!(chunks[0].len(), 10); // 50%
        let total: usize = chunks.iter().map(|c| c.len()).sum();
        assert_eq!(total, 20);

        // 10 peers with same weights
        let peers: Vec<_> = (0..10).map(|_| vec![AccountAddress::random()]).collect();
        let chunks = weighted_chunk_peers(peers, &[0.50, 0.17, 0.17, 0.16]);
        assert_eq!(chunks.len(), 4);
        assert_eq!(chunks[0].len(), 5); // 50%
        let total: usize = chunks.iter().map(|c| c.len()).sum();
        assert_eq!(total, 10);

        // 7 peers with 50/50 split
        let peers: Vec<_> = (0..7).map(|_| vec![AccountAddress::random()]).collect();
        let chunks = weighted_chunk_peers(peers, &[0.50, 0.50]);
        assert_eq!(chunks.len(), 2);
        assert!(chunks[0].len() == 3 || chunks[0].len() == 4);
        assert_eq!(chunks[0].len() + chunks[1].len(), 7);

        // All weight on one region
        let peers: Vec<_> = (0..8).map(|_| vec![AccountAddress::random()]).collect();
        let chunks = weighted_chunk_peers(peers, &[1.0, 0.0, 0.0, 0.0]);
        assert_eq!(chunks[0].len(), 8);
        assert_eq!(chunks[1].len(), 0);
        assert_eq!(chunks[2].len(), 0);
        assert_eq!(chunks[3].len(), 0);
    }

    #[test]
    fn test_mainnet_calibrated_chaos() {
        aptos_logger::Logger::new().init();

        // 20 validators with mainnet-calibrated config
        let all_peers: Vec<_> = (0..20).map(|_| vec![PeerId::random()]).collect();
        let config = MultiRegionNetworkEmulationConfig::mainnet_calibrated();
        let netem = create_multi_region_swarm_network_chaos(all_peers, Some(config));

        // 4 intra-region + 6 inter-region (4 choose 2 = 6, bidirectional = 12) = 16
        assert_eq!(netem.group_netems.len(), 16);

        // Check that EU regions got more peers (first two groups)
        // Weights: [0.43, 0.28, 0.15, 0.14] for 20 peers = [9, 6, 3, 2] or similar (~71% EU)
        let eu_west2_count = netem.group_netems[0].source_nodes.len();
        let eu_west6_count = netem.group_netems[1].source_nodes.len();
        assert!(eu_west2_count + eu_west6_count >= 14); // ~71% in EU
    }

    #[test]
    fn test_chunk_peers() {
        let peers: Vec<_> = (0..3).map(|_| vec![AccountAddress::random()]).collect();
        let chunks = chunk_peers(peers, 4);
        assert_eq!(chunks[0].len(), 1);
        assert_eq!(chunks[1].len(), 1);
        assert_eq!(chunks[2].len(), 1);
        assert_eq!(chunks[3].len(), 0);

        let peers: Vec<_> = (0..4).map(|_| vec![AccountAddress::random()]).collect();
        let chunks = chunk_peers(peers, 4);
        assert_eq!(chunks[0].len(), 1);
        assert_eq!(chunks[1].len(), 1);
        assert_eq!(chunks[2].len(), 1);
        assert_eq!(chunks[3].len(), 1);

        let peers: Vec<_> = (0..5).map(|_| vec![AccountAddress::random()]).collect();
        let chunks = chunk_peers(peers, 4);
        assert_eq!(chunks[0].len(), 2);
        assert_eq!(chunks[1].len(), 1);
        assert_eq!(chunks[2].len(), 1);
        assert_eq!(chunks[3].len(), 1);

        let peers: Vec<_> = (0..6).map(|_| vec![AccountAddress::random()]).collect();
        let chunks = chunk_peers(peers, 4);
        assert_eq!(chunks[0].len(), 2);
        assert_eq!(chunks[1].len(), 2);
        assert_eq!(chunks[2].len(), 1);
        assert_eq!(chunks[3].len(), 1);

        let peers: Vec<_> = (0..7).map(|_| vec![AccountAddress::random()]).collect();
        let chunks = chunk_peers(peers, 4);
        assert_eq!(chunks[0].len(), 2);
        assert_eq!(chunks[1].len(), 2);
        assert_eq!(chunks[2].len(), 2);
        assert_eq!(chunks[3].len(), 1);

        let peers: Vec<_> = (0..8).map(|_| vec![AccountAddress::random()]).collect();
        let chunks = chunk_peers(peers, 4);
        assert_eq!(chunks[0].len(), 2);
        assert_eq!(chunks[1].len(), 2);
        assert_eq!(chunks[2].len(), 2);
        assert_eq!(chunks[3].len(), 2);
    }
}
