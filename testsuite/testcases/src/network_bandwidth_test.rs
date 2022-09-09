// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use crate::network_chaos_test::NetworkChaosTest;
use forge::{NetworkContext, NetworkTest, SwarmChaos, SwarmNetworkBandwidth, Test};

pub struct NetworkBandwidthTest;

// Bandwidth
// Indicates the rate of bandwidth limit
pub const RATE_MBPS: u64 = 100;
// Indicates the number of bytes waiting in queue
pub const LIMIT_BYTES: u64 = 20971520;
// Indicates the maximum number of bytes that can be sent instantaneously
pub const BUFFER_BYTES: u64 = 10000;

impl Test for NetworkBandwidthTest {
    fn name(&self) -> &'static str {
        "network::bandwidth-test"
    }
}

impl NetworkChaosTest for NetworkBandwidthTest {
    fn get_chaos(&self) -> SwarmChaos {
        SwarmChaos::Bandwidth(SwarmNetworkBandwidth {
            rate: RATE_MBPS,
            limit: LIMIT_BYTES,
            buffer: BUFFER_BYTES,
        })
    }

    fn get_message(&self) -> String {
        format!(
            "Limited bandwidth to {}mbps with limit {} and buffer {} to namespace",
            RATE_MBPS, LIMIT_BYTES, BUFFER_BYTES
        )
    }
}

impl NetworkTest for NetworkBandwidthTest {
    fn run<'t>(&self, ctx: &mut NetworkContext<'t>) -> anyhow::Result<()> {
        <dyn NetworkChaosTest>::run(self, ctx)
    }
}
