// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{three_region_simulation_test::ExecutionDelayConfig, LoadDestination, NetworkLoadTest};
use aptos_forge::{
    GroupNetworkBandwidth, GroupNetworkDelay, NetworkContext, NetworkTest, Swarm, SwarmChaos,
    SwarmExt, SwarmNetworkBandwidth, SwarmNetworkDelay, Test,
};
use aptos_logger::info;
use aptos_types::PeerId;
use csv::Reader;
use itertools::{self, Itertools};
use rand::Rng;
use std::collections::BTreeMap;
use tokio::runtime::Runtime;

macro_rules! LATENCY_TABLE_CSV {
    () => {
        "data/latency_table.csv"
    };
}

pub struct MultiRegionSimulationTest {
    pub add_execution_delay: Option<ExecutionDelayConfig>,
}

impl Test for MultiRegionSimulationTest {
    fn name(&self) -> &'static str {
        "network::multi-region-simulation"
    }
}

fn get_link_stats_table() -> BTreeMap<String, BTreeMap<String, (u64, f64)>> {
    let mut stats_table = BTreeMap::new();

    let mut rdr = Reader::from_reader(include_bytes!(LATENCY_TABLE_CSV!()).as_slice());
    for result in rdr.deserialize() {
        if let Ok((from, to, bitrate, latency)) = result {
            let from: String = from;
            let to: String = to;
            stats_table
                .entry(from)
                .or_insert_with(BTreeMap::new)
                .insert(to, (bitrate, latency));
        }
    }

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

    let mut group_network_delays: Vec<GroupNetworkDelay> = validator_chunks
        .clone()
        .zip(link_stats_table.iter().clone())
        .combinations(2)
        .map(|comb| {
            let (from_chunk, (from_region, stats)) = &comb[0];
            let (to_chunk, (to_region, _)) = &comb[1];

            let (_, latency) = stats.get(*to_region).unwrap();
            let delay = [
                GroupNetworkDelay {
                    name: format!("{}-to-{}-delay", from_region.clone(), to_region.clone()),
                    source_nodes: from_chunk.to_vec(),
                    target_nodes: to_chunk.to_vec(),
                    latency_ms: *latency as u64,
                    jitter_ms: 5,
                    correlation_percentage: 50,
                },
                GroupNetworkDelay {
                    name: format!("{}-to-{}-delay", to_region.clone(), from_region.clone()),
                    source_nodes: to_chunk.to_vec(),
                    target_nodes: from_chunk.to_vec(),
                    latency_ms: *latency as u64,
                    jitter_ms: 5,
                    correlation_percentage: 50,
                },
            ];
            info!("delay {:?}", delay);
            delay
        })
        .flatten()
        .collect();

    let mut group_network_bandwidth: Vec<GroupNetworkBandwidth> = validator_chunks
        .clone()
        .zip(link_stats_table.iter())
        .combinations(2)
        .map(|comb| {
            let (from_chunk, (from_region, stats)) = &comb[0];
            let (to_chunk, (to_region, _)) = &comb[1];

            let (bitrate, latency) = stats.get(*to_region).unwrap();
            let bandwidth = GroupNetworkBandwidth {
                name: format!("{}-to-{}-bandwidth", from_region.clone(), to_region.clone()),
                source_nodes: from_chunk.to_vec(),
                target_nodes: to_chunk.to_vec(),
                rate: bitrate / 1000_000,
                limit: (2f64 * (*bitrate as f64 / 8f64) * (latency / 1000f64)) as u64,
                buffer: bitrate / 8,
            };
            info!("bandwidth {:?}", bandwidth);

            bandwidth
        })
        .collect();

    let remainder = validator_chunks.remainder();
    let remaining_validators: Vec<PeerId> = validator_chunks
        .skip(number_of_regions)
        .flatten()
        .chain(remainder.into_iter())
        .cloned()
        .collect();
    info!("remaining: {:?}", remaining_validators);
    if remaining_validators.len() > 0 {
        group_network_delays[0]
            .source_nodes
            .append(remaining_validators.to_vec().as_mut());
        group_network_delays[1]
            .target_nodes
            .append(remaining_validators.to_vec().as_mut());
        group_network_bandwidth[0]
            .source_nodes
            .append(remaining_validators.to_vec().as_mut());
    }

    (
        SwarmNetworkDelay {
            group_network_delays,
        },
        SwarmNetworkBandwidth {
            group_network_bandwidth,
        },
    )
}

fn add_execution_delay(swarm: &mut dyn Swarm, config: &ExecutionDelayConfig) -> anyhow::Result<()> {
    let runtime = Runtime::new().unwrap();
    let validators = swarm.get_validator_clients_with_names();

    runtime.block_on(async {
        let mut rng = rand::thread_rng();
        for (name, validator) in validators {
            let sleep_fraction = if rng.gen_bool(config.inject_delay_node_fraction) {
                rng.gen_range(1_u32, config.inject_delay_max_transaction_percentage)
            } else {
                0
            };
            let name = name.clone();
            info!(
                "Validator {} adding {}% of transactions with 1ms execution delay",
                name, sleep_fraction
            );
            validator
                .set_failpoint(
                    "aptos_vm::execution::user_transaction".to_string(),
                    format!(
                        "{}%delay({})",
                        sleep_fraction, config.inject_delay_per_transaction_ms
                    ),
                )
                .await
                .map_err(|e| {
                    anyhow::anyhow!(
                        "set_failpoint to add execution delay on {} failed, {:?}",
                        name,
                        e
                    )
                })?;
        }
        Ok::<(), anyhow::Error>(())
    })
}

fn remove_execution_delay(swarm: &mut dyn Swarm) -> anyhow::Result<()> {
    let runtime = Runtime::new().unwrap();
    let validators = swarm.get_validator_clients_with_names();

    runtime.block_on(async {
        for (name, validator) in validators {
            let name = name.clone();

            validator
                .set_failpoint(
                    "aptos_vm::execution::block_metadata".to_string(),
                    "off".to_string(),
                )
                .await
                .map_err(|e| {
                    anyhow::anyhow!(
                        "set_failpoint to remove execution delay on {} failed, {:?}",
                        name,
                        e
                    )
                })?;
        }
        Ok::<(), anyhow::Error>(())
    })
}

impl NetworkLoadTest for MultiRegionSimulationTest {
    fn setup(&self, ctx: &mut NetworkContext) -> anyhow::Result<LoadDestination> {
        let all_validators = ctx
            .swarm()
            .validators()
            .map(|v| v.peer_id())
            .collect::<Vec<_>>();

        let (delay, bandwidth) = create_multi_region_swarm_network_chaos(all_validators);

        // inject network delay
        let chaos = SwarmChaos::Delay(delay);
        ctx.swarm().inject_chaos(chaos)?;

        // inject bandwidth limit
        let chaos = SwarmChaos::Bandwidth(bandwidth);
        ctx.swarm().inject_chaos(chaos)?;

        if let Some(config) = &self.add_execution_delay {
            add_execution_delay(ctx.swarm(), config)?;
        }

        Ok(LoadDestination::FullnodesOtherwiseValidators)
    }

    fn finish(&self, swarm: &mut dyn Swarm) -> anyhow::Result<()> {
        if self.add_execution_delay.is_some() {
            remove_execution_delay(swarm)?;
        }

        swarm.remove_all_chaos()
    }
}

impl NetworkTest for MultiRegionSimulationTest {
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

        let all_validators = (0..20).map(|_| PeerId::random()).collect();
        let (delay, bandwidth) = create_multi_region_swarm_network_chaos(all_validators);

        assert_eq!(delay.group_network_delays.len(), 380);
        assert_eq!(bandwidth.group_network_bandwidth.len(), 190);

        let all_validators: Vec<PeerId> = (0..24).map(|_| PeerId::random()).collect();
        let (delay, bandwidth) = create_multi_region_swarm_network_chaos(all_validators.clone());

        assert_eq!(delay.group_network_delays.len(), 380);
        assert_eq!(bandwidth.group_network_bandwidth.len(), 190);
        assert_eq!(delay.group_network_delays[0].source_nodes.len(), 5);
        assert_eq!(delay.group_network_delays[0].target_nodes.len(), 1);
        assert_eq!(delay.group_network_delays[1].target_nodes.len(), 5);
        assert_eq!(delay.group_network_delays[1].source_nodes.len(), 1);
        assert_eq!(delay.group_network_delays[2].source_nodes.len(), 1);
        assert_eq!(bandwidth.group_network_bandwidth[0].source_nodes.len(), 5);
        assert_eq!(bandwidth.group_network_bandwidth[1].source_nodes.len(), 1);
        assert_eq!(
            bandwidth.group_network_bandwidth[0],
            GroupNetworkBandwidth {
                name: "aws--ap-northeast-1-to-aws--ap-southeast-1-bandwidth".to_owned(),
                source_nodes: vec![
                    all_validators[0],
                    all_validators[20],
                    all_validators[21],
                    all_validators[22],
                    all_validators[23]
                ],
                target_nodes: vec![all_validators[1]],
                rate: 118,
                limit: 2165768,
                buffer: 14860288
            }
        )
    }
}
