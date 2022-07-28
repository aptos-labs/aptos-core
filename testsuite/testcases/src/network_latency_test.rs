// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use crate::generate_traffic;
use forge::{NetworkContext, NetworkTest, Result, SwarmChaos, SwarmNetworkDelay, Test};

pub struct NetworkLatencyTest;

// Delay
pub const LATENCY_MS: u64 = 80;
pub const JITTER_MS: u64 = 20;
pub const CORRELATION_PERCENTAGE: u64 = 10;

impl Test for NetworkLatencyTest {
    fn name(&self) -> &'static str {
        "network::latency-test"
    }
}

impl NetworkTest for NetworkLatencyTest {
    fn run<'t>(&self, ctx: &mut NetworkContext<'t>) -> Result<()> {
        let duration = ctx.global_job.duration;
        let delay = SwarmChaos::Delay(SwarmNetworkDelay {
            latency_ms: LATENCY_MS,
            jitter_ms: JITTER_MS,
            correlation_percentage: CORRELATION_PERCENTAGE,
        });
        // emit to all validator
        let all_validators = ctx
            .swarm()
            .validators()
            .map(|v| v.peer_id())
            .collect::<Vec<_>>();

        // INJECT DELAY AND EMIT TXNS
        ctx.swarm().inject_chaos(delay.clone())?;
        let msg = format!(
            "Injected {}ms +- {}ms with {}% correlation latency to namespace",
            LATENCY_MS, JITTER_MS, CORRELATION_PERCENTAGE
        );
        println!("{}", msg);
        ctx.report.report_text(msg);
        let txn_stat = generate_traffic(ctx, &all_validators, duration, 1, None)?;
        ctx.report
            .report_txn_stats(format!("{}:delay", self.name()), &txn_stat, duration);
        ctx.swarm().remove_chaos(delay)?;

        // ensure we meet the success criteria
        ctx.success_criteria()
            .check_for_success(&txn_stat, &duration)?;

        Ok(())
    }
}
