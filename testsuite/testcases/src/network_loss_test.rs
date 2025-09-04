// Copyright © Velor Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{LoadDestination, NetworkLoadTest};
use velor_forge::{
    NetworkContext, NetworkContextSynchronizer, NetworkTest, SwarmChaos, SwarmNetworkLoss, Test,
};
use async_trait::async_trait;

/// This is deprecated. Use [crate::multi_region_network_test::MultiRegionNetworkEmulationTest] instead
pub struct NetworkLossTest;

// Loss parameters
pub const LOSS_PERCENTAGE: u64 = 20;
pub const CORRELATION_PERCENTAGE: u64 = 10;

impl Test for NetworkLossTest {
    fn name(&self) -> &'static str {
        "network::loss-test"
    }
}

#[async_trait]
impl NetworkLoadTest for NetworkLossTest {
    async fn setup<'a>(&self, ctx: &mut NetworkContext<'a>) -> anyhow::Result<LoadDestination> {
        ctx.swarm
            .write()
            .await
            .inject_chaos(SwarmChaos::Loss(SwarmNetworkLoss {
                loss_percentage: LOSS_PERCENTAGE,
                correlation_percentage: CORRELATION_PERCENTAGE,
            }))
            .await?;

        let msg = format!(
            "Injected {}% loss with {}% correlation loss to namespace",
            LOSS_PERCENTAGE, CORRELATION_PERCENTAGE,
        );
        println!("{}", msg);
        ctx.report.report_text(msg);
        Ok(LoadDestination::FullnodesOtherwiseValidators)
    }

    async fn finish<'a>(&self, ctx: &mut NetworkContext<'a>) -> anyhow::Result<()> {
        ctx.swarm
            .write()
            .await
            .remove_chaos(SwarmChaos::Loss(SwarmNetworkLoss {
                loss_percentage: LOSS_PERCENTAGE,
                correlation_percentage: CORRELATION_PERCENTAGE,
            }))
            .await?;
        Ok(())
    }
}

#[async_trait]
impl NetworkTest for NetworkLossTest {
    async fn run<'a>(&self, ctx: NetworkContextSynchronizer<'a>) -> anyhow::Result<()> {
        <dyn NetworkLoadTest>::run(self, ctx).await
    }
}
