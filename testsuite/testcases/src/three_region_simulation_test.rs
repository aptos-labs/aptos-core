// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{LoadDestination, NetworkLoadTest};
use aptos_forge::{
    GroupNetworkBandwidth, GroupNetworkDelay, NetworkContext, NetworkContextSynchronizer,
    NetworkTest, SwarmChaos, SwarmNetworkBandwidth, SwarmNetworkDelay, Test,
};
use aptos_types::account_address::AccountAddress;
use async_trait::async_trait;
use log::info;

/// Represents a test that simulates a network with 3 regions, all in the same cloud.
pub struct ThreeRegionSameCloudSimulationTest;

impl Test for ThreeRegionSameCloudSimulationTest {
    fn name(&self) -> &'static str {
        "network::three-region-simulation"
    }
}

/// Create a SwarmNetworkDelay with the following topology:
/// 1. 3 equal size group of nodes, each in a different region
/// 2. Each region has minimal network delay amongst its nodes
/// 3. Each region has a network delay to the other two regions, as estimated by https://www.cloudping.co/grid
/// 4. Currently simulating a 50 percentile network delay between us-west <--> af-south <--> eu-north
///
/// This is deprecated and flawed. Use [crate::multi_region_network_test::MultiRegionNetworkEmulationTest] instead
fn create_three_region_swarm_network_delay(
    all_validators: Vec<AccountAddress>,
) -> SwarmNetworkDelay {
    // each region has 1/3 of the validators
    let region_size = all_validators.len() / 3;
    let mut us_west = all_validators;
    let mut af_south = us_west.split_off(region_size);
    let eu_north = af_south.split_off(region_size);

    let group_network_delays = vec![
        GroupNetworkDelay {
            name: "us-west-to-af-south".to_string(),
            source_nodes: us_west.clone(),
            target_nodes: af_south.clone(),
            latency_ms: 300,
            jitter_ms: 50,
            correlation_percentage: 50,
        },
        GroupNetworkDelay {
            name: "us-west-to-eu-north".to_string(),
            source_nodes: us_west.clone(),
            target_nodes: eu_north.clone(),
            latency_ms: 150,
            jitter_ms: 50,
            correlation_percentage: 50,
        },
        GroupNetworkDelay {
            name: "eu-north-to-af-south".to_string(),
            source_nodes: eu_north.clone(),
            target_nodes: af_south.clone(),
            latency_ms: 200,
            jitter_ms: 50,
            correlation_percentage: 50,
        },
    ];

    info!("US_WEST: {:?}", us_west);
    info!("AF_SOUTH B: {:?}", af_south);
    info!("EU_NORTH C: {:?}", eu_north);

    SwarmNetworkDelay {
        group_network_delays,
    }
}

/// 1000 mbps network bandwidth simulation between all regions within
/// the same cloud with dedicated backbone like GCP
fn create_bandwidth_limit() -> SwarmNetworkBandwidth {
    SwarmNetworkBandwidth {
        group_network_bandwidths: vec![GroupNetworkBandwidth {
            name: "forge-namespace-1000mbps-bandwidth".to_owned(),
            rate: 1000, // 1000 megabytes per second
            limit: 20971520,
            buffer: 10000,
        }],
    }
}

#[async_trait]
impl NetworkLoadTest for ThreeRegionSameCloudSimulationTest {
    async fn setup<'a>(&self, ctx: &mut NetworkContext<'a>) -> anyhow::Result<LoadDestination> {
        // inject network delay
        let all_validators = {
            ctx.swarm
                .read()
                .await
                .validators()
                .map(|v| v.peer_id())
                .collect::<Vec<_>>()
        };
        let delay = create_three_region_swarm_network_delay(all_validators);
        let mut swarm = ctx.swarm.write().await;
        let chaos = SwarmChaos::Delay(delay);
        swarm.inject_chaos(chaos).await?;

        // inject bandwidth limit
        let bandwidth = create_bandwidth_limit();
        let chaos = SwarmChaos::Bandwidth(bandwidth);
        swarm.inject_chaos(chaos).await?;

        Ok(LoadDestination::FullnodesOtherwiseValidators)
    }

    async fn finish<'a>(&self, ctx: &mut NetworkContext<'a>) -> anyhow::Result<()> {
        ctx.swarm.write().await.remove_all_chaos().await?;
        Ok(())
    }
}

#[async_trait]
impl NetworkTest for ThreeRegionSameCloudSimulationTest {
    async fn run<'a>(&self, ctx: NetworkContextSynchronizer<'a>) -> anyhow::Result<()> {
        <dyn NetworkLoadTest>::run(self, ctx).await
    }
}
