// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use crate::{LoadDestination, NetworkLoadTest};
use forge::{NetworkContext, NetworkTest, Swarm, SwarmChaos, SwarmNetworkPartition, Test};

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
        ctx.swarm()
            .inject_chaos(SwarmChaos::Partition(SwarmNetworkPartition {
                partition_percentage: PARTITION_PERCENTAGE,
            }))?;

        let msg = format!(
            "Partitioned {}% validators in namespace",
            PARTITION_PERCENTAGE
        );
        println!("{}", msg);
        ctx.report.report_text(msg);
        // Just send the load to last validator which is not included in the partition
        Ok(LoadDestination::Peers(vec![ctx
            .swarm()
            .validators()
            .last()
            .map(|v| v.peer_id())
            .unwrap()]))
    }

    fn finish(&self, swarm: &mut dyn Swarm) -> anyhow::Result<()> {
        swarm.remove_chaos(SwarmChaos::Partition(SwarmNetworkPartition {
            partition_percentage: PARTITION_PERCENTAGE,
        }))
    }
}

impl NetworkTest for NetworkPartitionTest {
    fn run<'t>(&self, ctx: &mut NetworkContext<'t>) -> anyhow::Result<()> {
        <dyn NetworkLoadTest>::run(self, ctx)
    }
}
