// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use crate::network_chaos_test::NetworkChaosTest;
use forge::{NetworkContext, NetworkTest, SwarmChaos, SwarmNetworkDelay, Test};

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

impl NetworkChaosTest for NetworkLatencyTest {
    fn get_chaos(&self) -> SwarmChaos {
        SwarmChaos::Delay(SwarmNetworkDelay {
            latency_ms: LATENCY_MS,
            jitter_ms: JITTER_MS,
            correlation_percentage: CORRELATION_PERCENTAGE,
        })
    }

    fn get_message(&self) -> String {
        format!(
            "Injected {}ms +- {}ms with {}% correlation latency to namespace",
            LATENCY_MS, JITTER_MS, CORRELATION_PERCENTAGE
        )
    }
}

impl NetworkTest for NetworkLatencyTest {
    fn run<'t>(&self, ctx: &mut NetworkContext<'t>) -> anyhow::Result<()> {
        <dyn NetworkChaosTest>::run(self, ctx)
    }
}
