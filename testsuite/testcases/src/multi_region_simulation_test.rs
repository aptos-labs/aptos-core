// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{LoadDestination, NetworkLoadTest};
use aptos_forge::{
    GroupNetworkBandwidth, GroupNetworkDelay, NetworkContext, NetworkTest, Swarm, SwarmChaos,
    SwarmNetworkBandwidth, SwarmNetworkDelay, Test,
};
use aptos_logger::info;
use aptos_types::PeerId;
use csv::Reader;
use itertools::{self, Itertools};
use std::collections::BTreeMap;

/// The link stats are obtained from https://github.com/doitintl/intercloud-throughput/blob/master/results_202202/results.csv
/// The four regions were hand-picked from the dataset to simulate a multi-region setup
/// with high latencies and low bandwidth.
macro_rules! FOUR_REGION_LINK_STATS_CSV {
    () => {
        "data/four_region_link_stats.csv"
    };
}

/// A test to simulate network between multiple regions in different clouds.
/// It currently supports only 4 regions, due to ChaosMesh limitations.
pub struct MultiRegionMultiCloudSimulationTest {}

impl Test for MultiRegionMultiCloudSimulationTest {
    fn name(&self) -> &'static str {
        "network::multi-region-multi-cloud-simulation"
    }
}

fn get_link_stats_table() -> BTreeMap<String, BTreeMap<String, (u64, f64)>> {
    let mut stats_table = BTreeMap::new();

    let mut rdr = Reader::from_reader(include_bytes!(FOUR_REGION_LINK_STATS_CSV!()).as_slice());
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

/// Creates a SwarmNetworkDelay
fn create_multi_region_swarm_network_chaos(
    all_validators: Vec<PeerId>,
) -> (SwarmNetworkDelay, SwarmNetworkBandwidth) {
    let link_stats_table = get_link_stats_table();

    assert!(all_validators.len() >= link_stats_table.len());

    let number_of_regions = link_stats_table.len();
    let approx_validators_per_region = all_validators.len() / number_of_regions;

    let validator_chunks = all_validators.chunks_exact(approx_validators_per_region);

    let (mut group_network_delays, group_network_bandwidths): (
        Vec<GroupNetworkDelay>,
        Vec<GroupNetworkBandwidth>,
    ) = validator_chunks
        .clone()
        .zip(link_stats_table.iter().clone())
        .combinations(2)
        .map(|comb| {
            let (from_chunk, (from_region, stats)) = &comb[0];
            let (to_chunk, (to_region, _)) = &comb[1];

            let (bandwidth, latency) = stats.get(*to_region).unwrap();
            let delay = GroupNetworkDelay {
                name: format!("{}-to-{}-delay", from_region, to_region),
                source_nodes: from_chunk.to_vec(),
                target_nodes: to_chunk.to_vec(),
                latency_ms: *latency as u64,
                jitter_ms: 5,
                correlation_percentage: 50,
            };
            info!("delay {:?}", delay);

            let bandwidth = GroupNetworkBandwidth {
                name: format!("{}-to-{}-bandwidth", from_region, to_region),
                // source_nodes: from_chunk.to_vec(),
                // target_nodes: to_chunk.to_vec(),
                rate: bandwidth / 8,
                limit: 20971520,
                buffer: 10000,
            };
            info!("bandwidth {:?}", bandwidth);

            (delay, bandwidth)
        })
        .unzip();

    let remainder = validator_chunks.remainder();
    let remaining_validators: Vec<PeerId> = validator_chunks
        .skip(number_of_regions)
        .flatten()
        .chain(remainder.iter())
        .cloned()
        .collect();
    info!("remaining: {:?}", remaining_validators);
    if !remaining_validators.is_empty() {
        group_network_delays[0]
            .source_nodes
            .append(remaining_validators.to_vec().as_mut());
    }

    (
        SwarmNetworkDelay {
            group_network_delays,
        },
        SwarmNetworkBandwidth {
            group_network_bandwidths,
        },
    )
}

impl NetworkLoadTest for MultiRegionMultiCloudSimulationTest {
    fn setup(&self, ctx: &mut NetworkContext) -> anyhow::Result<LoadDestination> {
        let all_validators = ctx
            .swarm()
            .validators()
            .map(|v| v.peer_id())
            .collect::<Vec<_>>();

        let (delay, bandwidth) = create_multi_region_swarm_network_chaos(all_validators);

        // inject bandwidth limit
        let chaos = SwarmChaos::Bandwidth(bandwidth);
        ctx.swarm().inject_chaos(chaos)?;

        // inject network delay
        let chaos = SwarmChaos::Delay(delay);
        ctx.swarm().inject_chaos(chaos)?;

        Ok(LoadDestination::FullnodesOtherwiseValidators)
    }

    fn finish(&self, swarm: &mut dyn Swarm) -> anyhow::Result<()> {
        swarm.remove_all_chaos()
    }
}

impl NetworkTest for MultiRegionMultiCloudSimulationTest {
    fn run<'t>(&self, ctx: &mut NetworkContext<'t>) -> anyhow::Result<()> {
        <dyn NetworkLoadTest>::run(self, ctx)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_multi_region_swarm_network_chaos() {
        aptos_logger::Logger::new().init();

        let all_validators = (0..8).map(|_| PeerId::random()).collect();
        let (delay, bandwidth) = create_multi_region_swarm_network_chaos(all_validators);

        assert_eq!(delay.group_network_delays.len(), 6);
        assert_eq!(bandwidth.group_network_bandwidths.len(), 6);

        let all_validators: Vec<PeerId> = (0..10).map(|_| PeerId::random()).collect();
        let (delay, bandwidth) = create_multi_region_swarm_network_chaos(all_validators);

        assert_eq!(delay.group_network_delays.len(), 6);
        assert_eq!(bandwidth.group_network_bandwidths.len(), 6);
        assert_eq!(delay.group_network_delays[0].source_nodes.len(), 4);
        assert_eq!(delay.group_network_delays[0].target_nodes.len(), 2);
        assert_eq!(
            bandwidth.group_network_bandwidths[0],
            GroupNetworkBandwidth {
                name: "aws--ap-northeast-1-to-aws--eu-west-1-bandwidth".to_owned(),
                rate: 5160960,
                limit: 20971520,
                buffer: 10000,
            }
        )
    }
}
