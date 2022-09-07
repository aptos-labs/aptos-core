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
        "continuous progress test"
    }
}

impl NetworkLoadTest for ContinuousProgressTest {
    fn test(&self, swarm: &mut dyn Swarm, duration: Duration) -> Result<()> {
        let runtime = Runtime::new().unwrap();

        // Check that every 27s all nodes make progress,
        // without any failures.
        // (make epoch length (60s) and this duration (27s) not be multiples of one another,
        // to test different timings)
        let check_period_s: usize = 27;
        let target_tps = self.target_tps;

        runtime.block_on(test_consensus_fault_tolerance(
            swarm,
            duration.as_secs() as usize / check_period_s,
            check_period_s as f32,
            1,
            no_failure_injection(),
            Box::new(move |_, _, _, _, cur, previous| {
                // Make sure that every node is making progress, so we compare min(cur) vs max(previous)
                let epochs = cur.iter().map(|s| s.epoch).min().unwrap()
                    - previous.iter().map(|s| s.epoch).max().unwrap();
                let rounds = cur
                    .iter()
                    .map(|s| s.round)
                    .min()
                    .unwrap()
                    .saturating_sub(previous.iter().map(|s| s.round).max().unwrap());
                let transactions = cur.iter().map(|s| s.version).min().unwrap()
                    - previous.iter().map(|s| s.version).max().unwrap();

                assert!(
                    transactions >= (target_tps * check_period_s / 2) as u64,
                    "no progress with active consensus, only {} transactions, expected >= {}",
                    transactions,
                    (target_tps * check_period_s / 2),
                );
                assert!(
                    epochs > 0 || rounds >= (check_period_s / 2) as u64,
                    "no progress with active consensus, only {} epochs and {} rounds, expectd >= {}",
                    epochs,
                    rounds,
                    (check_period_s / 2),
                );
            }),
            false,
        ))?;

        Ok(())
    }
}

impl NetworkTest for ContinuousProgressTest {
    fn run<'t>(&self, ctx: &mut NetworkContext<'t>) -> Result<()> {
        <dyn NetworkLoadTest>::run(self, ctx)
    }
}
