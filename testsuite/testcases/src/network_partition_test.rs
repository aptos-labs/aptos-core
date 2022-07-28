// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use crate::generate_traffic;
use forge::{NetworkContext, NetworkTest, Result, SwarmChaos, SwarmNetworkPartition, Test};

pub struct NetworkPartitionTest;

// Partition
pub const PARTITION_PERCENTAGE: u64 = 30;

impl Test for NetworkPartitionTest {
    fn name(&self) -> &'static str {
        "network::partition-test"
    }
}

impl NetworkTest for NetworkPartitionTest {
    fn run<'t>(&self, ctx: &mut NetworkContext<'t>) -> Result<()> {
        let duration = ctx.global_job.duration;

        let partition = SwarmChaos::Partition(SwarmNetworkPartition {
            partition_percentage: PARTITION_PERCENTAGE,
        });

        // emit to all validator
        let all_validators = ctx
            .swarm()
            .validators()
            .map(|v| v.peer_id())
            .collect::<Vec<_>>();

        // INJECT PARTITION AND EMIT TXNS
        ctx.swarm().inject_chaos(partition.clone())?;
        let msg = format!(
            "Partitioned {}% validators in namespace",
            PARTITION_PERCENTAGE
        );
        println!("{}", msg);
        ctx.report.report_text(msg);
        let txn_stat = generate_traffic(ctx, &all_validators, duration, 1, None)?;
        ctx.report
            .report_txn_stats(format!("{}:partition", self.name()), &txn_stat, duration);
        ctx.swarm().remove_chaos(partition)?;

        // ensure we meet the success criteria
        ctx.success_criteria()
            .check_for_success(&txn_stat, &duration)?;

        Ok(())
    }
}
