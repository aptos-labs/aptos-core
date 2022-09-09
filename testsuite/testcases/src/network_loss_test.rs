// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use crate::network_chaos_test::NetworkChaosTest;
use forge::{NetworkContext, NetworkTest, SwarmChaos, SwarmNetworkLoss, Test};

pub struct NetworkLossTest;

// Loss parameters
pub const LOSS_PERCENTAGE: u64 = 20;
pub const CORRELATION_PERCENTAGE: u64 = 10;

impl Test for NetworkLossTest {
    fn name(&self) -> &'static str {
        "network::loss-test"
    }
}

impl NetworkChaosTest for NetworkLossTest {
    fn get_chaos(&self) -> SwarmChaos {
        SwarmChaos::Loss(SwarmNetworkLoss {
            loss_percentage: LOSS_PERCENTAGE,
            correlation_percentage: CORRELATION_PERCENTAGE,
        })
    }

    fn get_message(&self) -> String {
        format!(
            "Injected {}% loss with {}% correlation loss to namespace",
            LOSS_PERCENTAGE, CORRELATION_PERCENTAGE,
        )
    }
}

impl NetworkTest for NetworkLossTest {
    fn run<'t>(&self, ctx: &mut NetworkContext<'t>) -> anyhow::Result<()> {
        <dyn NetworkChaosTest>::run(self, ctx)
    }
}
