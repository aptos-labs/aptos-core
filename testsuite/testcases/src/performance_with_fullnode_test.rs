// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use crate::generate_traffic;
use forge::{NetworkContext, NetworkTest, Result, Test};
use tokio::runtime::Runtime;

pub struct PerformanceBenchmarkWithFN;

impl Test for PerformanceBenchmarkWithFN {
    fn name(&self) -> &'static str {
        "performance benchmark with full nodes"
    }
}

impl NetworkTest for PerformanceBenchmarkWithFN {
    fn run<'t>(&self, ctx: &mut NetworkContext<'t>) -> Result<()> {
        // 12 hours
        let duration = ctx.global_job.duration;

        let all_fullnodes = ctx
            .swarm()
            .full_nodes()
            .map(|v| v.peer_id())
            .collect::<Vec<_>>();

        // Generate some traffic
        let txn_stat = generate_traffic(ctx, &all_fullnodes, duration, 1)?;
        ctx.report
            .report_txn_stats(self.name().to_string(), &txn_stat, duration);
        // ensure we meet the success criteria
        ctx.check_for_success(&txn_stat, &duration)?;

        let runtime = Runtime::new().unwrap();
        runtime.block_on(ctx.swarm().ensure_no_validator_restart())?;
        runtime.block_on(ctx.swarm().ensure_no_fullnode_restart())?;

        Ok(())
    }
}
