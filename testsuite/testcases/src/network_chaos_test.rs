// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use crate::generate_traffic;
use forge::{NetworkContext, NetworkTest, Result, SwarmChaos, Test};
use std::time::{SystemTime, UNIX_EPOCH};

pub trait NetworkChaosTest: Test {
    fn get_chaos(&self) -> SwarmChaos;
    fn get_message(&self) -> String;
}

impl NetworkTest for dyn NetworkChaosTest {
    fn run<'t>(&self, ctx: &mut NetworkContext<'t>) -> Result<()> {
        let start_timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("Time went backwards")
            .as_secs();
        let duration = ctx.global_duration;

        // emit to all validator
        let all_validators = ctx
            .swarm()
            .validators()
            .map(|v| v.peer_id())
            .collect::<Vec<_>>();

        ctx.swarm().inject_chaos(self.get_chaos())?;
        println!("{}", self.get_message());
        ctx.report.report_text(self.get_message());
        let txn_stat = generate_traffic(ctx, &all_validators, duration, 1)?;
        ctx.report
            .report_txn_stats(format!("{}:partition", self.name()), &txn_stat, duration);
        ctx.swarm().remove_chaos(self.get_chaos())?;

        let end_timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("Time went backwards")
            .as_secs();

        // ensure we meet the success criteria
        ctx.check_for_success(
            &txn_stat,
            &duration,
            start_timestamp as i64,
            end_timestamp as i64,
        )?;

        Ok(())
    }
}
