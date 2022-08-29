// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use crate::NetworkLoadTest;
use forge::test_utils::consensus_utils::{no_failure_injection, test_consensus_fault_tolerance};
use forge::{NetworkContext, NetworkTest, Result, Swarm, Test};
use std::time::Duration;
use tokio::runtime::Runtime;

pub struct ContinuousProgressTest {
    pub target_tps: usize,
}

impl Test for ContinuousProgressTest {
    fn name(&self) -> &'static str {
        "performance benchmark"
    }
}

impl NetworkLoadTest for ContinuousProgressTest {
    fn test(&self, swarm: &mut dyn Swarm, duration: Duration) -> Result<()> {
        let runtime = Runtime::new().unwrap();

        // Check that every 20s all nodes make progress,
        // without any failures.
        let check_period_s: usize = 20;
        let target_tps = self.target_tps;

        runtime.block_on(test_consensus_fault_tolerance(
            swarm,
            duration.as_secs() as usize / check_period_s,
            check_period_s as f32,
            1,
            no_failure_injection(),
            Box::new(move |_, _, executed_rounds, executed_transactions, _, _| {
                assert!(
                    executed_transactions >= (target_tps * check_period_s / 2) as u64,
                    "no progress with active consensus, only {} transactions",
                    executed_transactions
                );
                assert!(
                    executed_rounds >= (check_period_s / 2) as u64,
                    "no progress with active consensus, only {} rounds",
                    executed_rounds
                );
            }),
            true,
        ))?;

        Ok(())
    }
}

impl NetworkTest for ContinuousProgressTest {
    fn run<'t>(&self, ctx: &mut NetworkContext<'t>) -> Result<()> {
        <dyn NetworkLoadTest>::run(self, ctx)
    }
}
