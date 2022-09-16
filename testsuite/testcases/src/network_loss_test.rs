// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use crate::{LoadDestination, NetworkLoadTest};
use forge::{NetworkContext, NetworkTest, Swarm, SwarmChaos, SwarmNetworkLoss, Test};

pub struct NetworkLossTest;

// Loss parameters
pub const LOSS_PERCENTAGE: u64 = 20;
pub const CORRELATION_PERCENTAGE: u64 = 10;

impl Test for NetworkLossTest {
    fn name(&self) -> &'static str {
        "network::loss-test"
    }
}

impl NetworkLoadTest for NetworkLossTest {
    fn setup(&self, ctx: &mut NetworkContext) -> anyhow::Result<LoadDestination> {
        ctx.swarm()
            .inject_chaos(SwarmChaos::Loss(SwarmNetworkLoss {
                loss_percentage: LOSS_PERCENTAGE,
                correlation_percentage: CORRELATION_PERCENTAGE,
            }))?;

        let msg = format!(
            "Injected {}% loss with {}% correlation loss to namespace",
            LOSS_PERCENTAGE, CORRELATION_PERCENTAGE,
        );
        println!("{}", msg);
        ctx.report.report_text(msg);
        Ok(LoadDestination::AllNodes)
    }

    fn finish(&self, swarm: &mut dyn Swarm) -> anyhow::Result<()> {
        swarm.remove_chaos(SwarmChaos::Loss(SwarmNetworkLoss {
            loss_percentage: LOSS_PERCENTAGE,
            correlation_percentage: CORRELATION_PERCENTAGE,
        }))
    }
}

impl NetworkTest for NetworkLossTest {
    fn run<'t>(&self, ctx: &mut NetworkContext<'t>) -> anyhow::Result<()> {
        <dyn NetworkLoadTest>::run(self, ctx)
    }
}
