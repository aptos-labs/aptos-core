// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use crate::network_chaos_test::NetworkChaosTest;
use forge::{NetworkContext, NetworkTest, SwarmChaos, SwarmNetworkPartition, Test};

pub struct NetworkPartitionTest;

// Partition
pub const PARTITION_PERCENTAGE: u64 = 30;

impl Test for NetworkPartitionTest {
    fn name(&self) -> &'static str {
        "network::partition-test"
    }
}

impl NetworkChaosTest for NetworkPartitionTest {
    fn get_chaos(&self) -> SwarmChaos {
        SwarmChaos::Partition(SwarmNetworkPartition {
            partition_percentage: PARTITION_PERCENTAGE,
        })
    }

    fn get_message(&self) -> String {
        format!(
            "Partitioned {}% validators in namespace",
            PARTITION_PERCENTAGE
        )
    }
}

impl NetworkTest for NetworkPartitionTest {
    fn run<'t>(&self, ctx: &mut NetworkContext<'t>) -> anyhow::Result<()> {
        <dyn NetworkChaosTest>::run(self, ctx)
    }
}
