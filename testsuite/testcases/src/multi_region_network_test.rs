// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

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
    if dividend % divisor == 0 {
        dividend / divisor
    } else {
        dividend / divisor + 1
    }
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
    let peer_chunks = chunk_peers(peers, number_of_regions);
    let peer_groups = peer_chunks
        .into_iter()
        .zip(link_stats_table.iter())
        .map(|(chunk, (from_region, stats))| (from_region.clone(), chunk, stats.clone()))
        .collect();

    peer_groups
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
            delay_latency_ms: 2,
            delay_jitter_ms: 0,
            delay_correlation_percentage: 20,
            loss_percentage: 1,
            loss_correlation_percentage: 20,
        }
    }
}

impl IntraRegionNetEmConfig {
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
}

impl Default for MultiRegionNetworkEmulationConfig {
    fn default() -> Self {
        Self {
            link_stats_table: get_link_stats_table(FOUR_REGION_LINK_STATS),
            inter_region_config: InterRegionNetEmConfig::default(),
            intra_region_config: Some(IntraRegionNetEmConfig::default()),
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

    pub fn default_for_validator_count(num_validators: usize) -> Self {
        if num_validators > 100 {
            Self {
                network_emulation_config: MultiRegionNetworkEmulationConfig::six_regions(),
            }
        } else {
            Self {
                network_emulation_config: MultiRegionNetworkEmulationConfig::four_regions(),
            }
        }
    }

    /// Creates a new SwarmNetEm to be injected via chaos. Note: network
    /// emulation is only done for the validators in the swarm (and not
    /// the fullnodes).
    async fn create_netem_chaos(
        &self,
        swarm: Arc<tokio::sync::RwLock<Box<(dyn Swarm)>>>,
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
