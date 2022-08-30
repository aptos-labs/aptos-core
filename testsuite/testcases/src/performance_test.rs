// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use crate::generate_traffic;
use forge::{NetworkContext, NetworkTest, Result, Test};
use tokio::runtime::Runtime;

pub struct PerformanceBenchmark;

impl Test for PerformanceBenchmark {
    fn name(&self) -> &'static str {
        "performance benchmark"
    }
}

impl NetworkTest for PerformanceBenchmark {
    fn run<'t>(&self, ctx: &mut NetworkContext<'t>) -> Result<()> {
        let duration = ctx.global_duration;
        let all_validators = ctx
            .swarm()
            .validators()
            .map(|v| v.peer_id())
            .collect::<Vec<_>>();

        let all_fullnodes = ctx
            .swarm()
            .full_nodes()
            .map(|v| v.peer_id())
            .collect::<Vec<_>>();

        let all_nodes = [&all_validators[..], &all_fullnodes[..]].concat();

        // Generate some traffic
        let txn_stat = generate_traffic(ctx, &all_nodes, duration, 1)?;
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
