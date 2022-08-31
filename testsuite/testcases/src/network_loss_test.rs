// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use crate::generate_traffic;
use forge::{NetworkContext, NetworkTest, Result, SwarmChaos, SwarmNetworkLoss, Test};

pub struct NetworkLossTest;

// Loss parameters
pub const LOSS_PERCENTAGE: u64 = 20;
pub const CORRELATION_PERCENTAGE: u64 = 10;

impl Test for NetworkLossTest {
    fn name(&self) -> &'static str {
        "network::loss-test"
    }
}

impl NetworkTest for NetworkLossTest {
    fn run<'t>(&self, ctx: &mut NetworkContext<'t>) -> Result<()> {
        let duration = ctx.global_duration;
        let loss_percentage = LOSS_PERCENTAGE;
        let correlation_percentage = CORRELATION_PERCENTAGE;
        let loss = SwarmChaos::Loss(SwarmNetworkLoss {
            loss_percentage,
            correlation_percentage,
        });
        // emit to all validator
        let all_validators = ctx
            .swarm()
            .validators()
            .map(|v| v.peer_id())
            .collect::<Vec<_>>();

        // Set up loss and emit txns
        ctx.swarm().inject_chaos(loss.clone())?;
        let msg = format!(
            "Injected {}% loss with {}% correlation loss to namespace",
            loss_percentage, correlation_percentage,
        );
        println!("{}", msg);
        ctx.report.report_text(msg);
        let txn_stat = generate_traffic(ctx, &all_validators, duration, 1)?;
        ctx.report
            .report_txn_stats(format!("{}:loss", self.name()), &txn_stat, duration);
        ctx.swarm().remove_chaos(loss)?;

        // ensure we meet the success criteria
        ctx.check_for_success(&txn_stat, &duration)?;

        Ok(())
    }
}
