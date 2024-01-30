// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{LoadDestination, NetworkLoadTest};
use aptos_forge::{NetworkContext, NetworkTest, SwarmChaos, SwarmNetworkPartition, Test};

pub struct NetworkPartitionTest;

// Partition
pub const PARTITION_PERCENTAGE: u64 = 30;

impl Test for NetworkPartitionTest {
    fn name(&self) -> &'static str {
        "network::partition-test"
    }
}

impl NetworkLoadTest for NetworkPartitionTest {
    fn setup(&self, ctx: &mut NetworkContext) -> anyhow::Result<LoadDestination> {
        ctx.runtime
            .block_on(
                ctx.swarm
                    .inject_chaos(SwarmChaos::Partition(SwarmNetworkPartition {
                        partition_percentage: PARTITION_PERCENTAGE,
                    })),
            )?;

        let msg = format!(
            "Partitioned {}% validators in namespace",
            PARTITION_PERCENTAGE
        );
        println!("{}", msg);
        ctx.report.report_text(msg);
        // Just send the load to last validator which is not included in the partition
        Ok(LoadDestination::Peers(vec![ctx
            .swarm
            .validators()
            .last()
            .map(|v| v.peer_id())
            .unwrap()]))
    }

    fn finish(&self, ctx: &mut NetworkContext) -> anyhow::Result<()> {
        ctx.runtime
            .block_on(
                ctx.swarm
                    .remove_chaos(SwarmChaos::Partition(SwarmNetworkPartition {
                        partition_percentage: PARTITION_PERCENTAGE,
                    })),
            )?;
        Ok(())
    }
}

impl NetworkTest for NetworkPartitionTest {
    fn run(&self, ctx: &mut NetworkContext<'_>) -> anyhow::Result<()> {
        <dyn NetworkLoadTest>::run(self, ctx)
    }
}
