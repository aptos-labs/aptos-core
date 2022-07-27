// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use crate::generate_traffic;
use forge::{NetworkContext, NetworkTest, Result, Test};

pub struct PerformanceBenchmark;

impl Test for PerformanceBenchmark {
    fn name(&self) -> &'static str {
        "all up"
    }
}

impl NetworkTest for PerformanceBenchmark {
    fn run<'t>(&self, ctx: &mut NetworkContext<'t>) -> Result<()> {
        let duration = ctx.global_job.duration;
        let all_validators = ctx
            .swarm()
            .validators()
            .map(|v| v.peer_id())
            .collect::<Vec<_>>();

        // Generate some traffic
        let txn_stat = generate_traffic(ctx, &all_validators, duration, 1, None)?;
        ctx.report
            .report_txn_stats(self.name().to_string(), &txn_stat, duration);
        // ensure we meet the success criteria
        ctx.success_criteria()
            .check_for_success(&txn_stat, &duration)?;

        Ok(())
    }
}
