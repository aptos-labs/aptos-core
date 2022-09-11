// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use crate::{LoadDestination, NetworkLoadTest};
use forge::{NetworkContext, NetworkTest, Swarm, SwarmChaos, SwarmNetworkDelay, Test};

pub struct NetworkLatencyTest;

// Delay
pub const LATENCY_MS: u64 = 200;
pub const JITTER_MS: u64 = 100;
pub const CORRELATION_PERCENTAGE: u64 = 10;

impl Test for NetworkLatencyTest {
    fn name(&self) -> &'static str {
        "network::latency-test"
    }
}

impl NetworkLoadTest for NetworkLatencyTest {
    fn setup(&self, ctx: &mut NetworkContext) -> anyhow::Result<LoadDestination> {
        ctx.swarm()
            .inject_chaos(SwarmChaos::Delay(SwarmNetworkDelay {
                latency_ms: LATENCY_MS,
                jitter_ms: JITTER_MS,
                correlation_percentage: CORRELATION_PERCENTAGE,
            }))?;
        let msg = format!(
            "Injected {}ms +- {}ms with {}% correlation latency to namespace",
            LATENCY_MS, JITTER_MS, CORRELATION_PERCENTAGE
        );
        println!("{}", msg);
        ctx.report.report_text(msg);
        Ok(LoadDestination::AllNodes)
    }

    fn finish(&self, swarm: &mut dyn Swarm) -> anyhow::Result<()> {
        swarm.remove_chaos(SwarmChaos::Delay(SwarmNetworkDelay {
            latency_ms: LATENCY_MS,
            jitter_ms: JITTER_MS,
            correlation_percentage: CORRELATION_PERCENTAGE,
        }))
    }
}

impl NetworkTest for NetworkLatencyTest {
    fn run<'t>(&self, ctx: &mut NetworkContext<'t>) -> anyhow::Result<()> {
        <dyn NetworkLoadTest>::run(self, ctx)
    }
}
