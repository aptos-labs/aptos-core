// Copyright © Velor Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{LoadDestination, NetworkLoadTest};
use velor_forge::{
    NetworkContext, NetworkContextSynchronizer, NetworkTest, SwarmChaos, SwarmNetworkPartition,
    Test,
};
use async_trait::async_trait;

/// This is deprecated. Use [crate::multi_region_network_test::MultiRegionNetworkEmulationTest] instead
pub struct NetworkPartitionTest;

// Partition
pub const PARTITION_PERCENTAGE: u64 = 30;

impl Test for NetworkPartitionTest {
    fn name(&self) -> &'static str {
        "network::partition-test"
    }
}

#[async_trait]
impl NetworkLoadTest for NetworkPartitionTest {
    async fn setup<'a>(&self, ctx: &mut NetworkContext<'a>) -> anyhow::Result<LoadDestination> {
        ctx.swarm
            .write()
            .await
            .inject_chaos(SwarmChaos::Partition(SwarmNetworkPartition {
                partition_percentage: PARTITION_PERCENTAGE,
            }))
            .await?;

        let msg = format!(
            "Partitioned {}% validators in namespace",
            PARTITION_PERCENTAGE
        );
        println!("{}", msg);
        ctx.report.report_text(msg);
        // Just send the load to last validator which is not included in the partition
        Ok(LoadDestination::Peers(vec![ctx
            .swarm
            .read()
            .await
            .validators()
            .last()
            .map(|v| v.peer_id())
            .unwrap()]))
    }

    async fn finish<'a>(&self, ctx: &mut NetworkContext<'a>) -> anyhow::Result<()> {
        ctx.swarm
            .write()
            .await
            .remove_chaos(SwarmChaos::Partition(SwarmNetworkPartition {
                partition_percentage: PARTITION_PERCENTAGE,
            }))
            .await?;
        Ok(())
    }
}

#[async_trait]
impl NetworkTest for NetworkPartitionTest {
    async fn run<'a>(&self, ctx: NetworkContextSynchronizer<'a>) -> anyhow::Result<()> {
        <dyn NetworkLoadTest>::run(self, ctx).await
    }
}
