// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{LoadDestination, NetworkLoadTest};
use aptos_forge::{
    GroupCpuStress, GroupNetEm, NetworkContext, NetworkTest, Swarm, SwarmChaos, SwarmCpuStress,
    SwarmNetEm, Test,
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

/// A test to simulate a real (e.g. mainnet, previewnet) network with multiple regions
/// in different clouds and varying CPU performance. It currently supports only 4
/// regions, due to ChaosMesh limitations.
pub struct RealNetworkSimulationTest {}

impl Test for RealNetworkSimulationTest {
    fn name(&self) -> &'static str {
        "network::real-network-simulation"
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
) -> (SwarmNetEm, SwarmCpuStress) {
    let link_stats_table = get_link_stats_table();

    assert!(all_validators.len() >= link_stats_table.len());

    let number_of_regions = link_stats_table.len();
    let approx_validators_per_region = all_validators.len() / number_of_regions;

    let validator_chunks = all_validators.chunks_exact(approx_validators_per_region);

    let mut group_netems: Vec<GroupNetEm> = validator_chunks
        .clone()
        .zip(link_stats_table.iter().clone())
        .combinations(2)
        .map(|comb| {
            let (from_chunk, (from_region, stats)) = &comb[0];
            let (to_chunk, (to_region, _)) = &comb[1];

            let (bandwidth, latency) = stats.get(*to_region).unwrap();
            let netem = GroupNetEm {
                name: format!("{}-to-{}-netem", from_region, to_region),
                source_nodes: from_chunk.to_vec(),
                target_nodes: to_chunk.to_vec(),
                delay_latency_ms: *latency as u64,
                delay_jitter_ms: 20,
                delay_correlation_percentage: 50,
                loss_percentage: 3,
                loss_correlation_percentage: 50,
                rate: *bandwidth / 1e6 as u64,
            };
            info!("netem {:?}", netem);

            netem
        })
        .collect();

    let (mut self_delays, mut group_cpu_stresses): (Vec<GroupNetEm>, Vec<GroupCpuStress>) =
        validator_chunks
            .clone()
            .zip(link_stats_table.iter().clone())
            .enumerate()
            .map(|(idx, (chunk, (region, _)))| {
                let cpu_stress = GroupCpuStress {
                    name: format!("{}-cpu-stress", region),
                    target_nodes: chunk.to_vec(),
                    num_workers: (number_of_regions - idx) as u64,
                    load_per_worker: 100,
                };
                let delay = GroupNetEm {
                    name: format!("{}-self-netem", region),
                    source_nodes: chunk.to_vec(),
                    target_nodes: chunk.to_vec(),
                    delay_latency_ms: 50,
                    delay_jitter_ms: 5,
                    delay_correlation_percentage: 50,
                    loss_percentage: 1,
                    loss_correlation_percentage: 50,
                    rate: 10 * 1000, // 10 Gbps
                };
                (delay, cpu_stress)
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
        group_netems[0]
            .source_nodes
            .append(remaining_validators.to_vec().as_mut());
        self_delays[0]
            .source_nodes
            .append(remaining_validators.to_vec().as_mut());
        self_delays[0]
            .target_nodes
            .append(remaining_validators.to_vec().as_mut());
        group_cpu_stresses[0]
            .target_nodes
            .append(remaining_validators.to_vec().as_mut());
    }

    (
        SwarmNetEm {
            group_netems: itertools::concat(vec![self_delays, group_netems]),
        },
        SwarmCpuStress { group_cpu_stresses },
    )
}

impl NetworkLoadTest for RealNetworkSimulationTest {
    fn setup(&self, ctx: &mut NetworkContext) -> anyhow::Result<LoadDestination> {
        let all_validators = ctx
            .swarm()
            .validators()
            .map(|v| v.peer_id())
            .collect::<Vec<_>>();

        let (netem, cpu) = create_multi_region_swarm_network_chaos(all_validators);

        // inject netem chaos
        let chaos = SwarmChaos::NetEm(netem);
        ctx.swarm().inject_chaos(chaos)?;

        // inject cpu stress
        let chaos = SwarmChaos::CpuStress(cpu);
        ctx.swarm().inject_chaos(chaos)?;

        Ok(LoadDestination::FullnodesOtherwiseValidators)
    }

    fn finish(&self, swarm: &mut dyn Swarm) -> anyhow::Result<()> {
        swarm.remove_all_chaos()
    }
}

impl NetworkTest for RealNetworkSimulationTest {
    fn run<'t>(&self, ctx: &mut NetworkContext<'t>) -> anyhow::Result<()> {
        <dyn NetworkLoadTest>::run(self, ctx)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::vec;

    #[test]
    fn test_create_multi_region_swarm_network_chaos() {
        aptos_logger::Logger::new().init();

        let all_validators = (0..8).map(|_| PeerId::random()).collect();
        let (netem, cpu_stress) = create_multi_region_swarm_network_chaos(all_validators);

        assert_eq!(netem.group_netems.len(), 10);
        assert_eq!(cpu_stress.group_cpu_stresses.len(), 4);

        let all_validators: Vec<PeerId> = (0..10).map(|_| PeerId::random()).collect();
        let (netem, cpu_stress) = create_multi_region_swarm_network_chaos(all_validators.clone());

        assert_eq!(netem.group_netems.len(), 10);
        assert_eq!(cpu_stress.group_cpu_stresses.len(), 4);
        assert_eq!(netem.group_netems[0].source_nodes.len(), 4);
        assert_eq!(netem.group_netems[0].target_nodes.len(), 4);
        assert_eq!(netem.group_netems[0], GroupNetEm {
            name: "aws--ap-northeast-1-self-netem".to_owned(),
            rate: 10000,
            source_nodes: vec![
                all_validators[0],
                all_validators[1],
                all_validators[8],
                all_validators[9],
            ],
            target_nodes: vec![
                all_validators[0],
                all_validators[1],
                all_validators[8],
                all_validators[9],
            ],
            delay_latency_ms: 50,
            delay_jitter_ms: 5,
            delay_correlation_percentage: 50,
            loss_percentage: 1,
            loss_correlation_percentage: 50
        })
    }
}
