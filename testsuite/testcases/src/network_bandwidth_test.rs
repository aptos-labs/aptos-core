// Copyright © Velor Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{LoadDestination, NetworkLoadTest};
use velor_forge::{
    GroupNetworkBandwidth, NetworkContext, NetworkContextSynchronizer, NetworkTest, SwarmChaos,
    SwarmNetworkBandwidth, Test,
};
use async_trait::async_trait;

/// This is deprecated. Use [crate::multi_region_network_test::MultiRegionNetworkEmulationTest] instead
pub struct NetworkBandwidthTest;

// Bandwidth
// Indicates the rate of bandwidth limit
pub const RATE_MBPS: u64 = 100;
// Indicates the number of bytes waiting in queue
pub const LIMIT_BYTES: u64 = 20971520;
// Indicates the maximum number of bytes that can be sent instantaneously
pub const BUFFER_BYTES: u64 = 10000;

impl Test for NetworkBandwidthTest {
    fn name(&self) -> &'static str {
        "network::bandwidth-test"
    }
}

#[async_trait]
impl NetworkLoadTest for NetworkBandwidthTest {
    async fn setup<'a>(&self, ctx: &mut NetworkContext<'a>) -> anyhow::Result<LoadDestination> {
        ctx.swarm
            .write()
            .await
            .inject_chaos(SwarmChaos::Bandwidth(SwarmNetworkBandwidth {
                group_network_bandwidths: vec![GroupNetworkBandwidth {
                    name: format!("forge-namespace-{}mbps-bandwidth", RATE_MBPS),
                    rate: RATE_MBPS,
                    limit: LIMIT_BYTES,
                    buffer: BUFFER_BYTES,
                }],
            }))
            .await?;

        let msg = format!(
            "Limited bandwidth to {}mbps with limit {} and buffer {} to namespace",
            RATE_MBPS, LIMIT_BYTES, BUFFER_BYTES
        );
        println!("{}", msg);
        ctx.report.report_text(msg);

        Ok(LoadDestination::FullnodesOtherwiseValidators)
    }

    async fn finish<'a>(&self, ctx: &mut NetworkContext<'a>) -> anyhow::Result<()> {
        ctx.swarm
            .write()
            .await
            .remove_chaos(SwarmChaos::Bandwidth(SwarmNetworkBandwidth {
                group_network_bandwidths: vec![GroupNetworkBandwidth {
                    name: format!("forge-namespace-{}mbps-bandwidth", RATE_MBPS),
                    rate: RATE_MBPS,
                    limit: LIMIT_BYTES,
                    buffer: BUFFER_BYTES,
                }],
            }))
            .await?;
        Ok(())
    }
}

#[async_trait]
impl NetworkTest for NetworkBandwidthTest {
    async fn run<'a>(&self, ctx: NetworkContextSynchronizer<'a>) -> anyhow::Result<()> {
        <dyn NetworkLoadTest>::run(self, ctx).await
    }
}
