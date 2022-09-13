// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use crate::{LoadDestination, NetworkLoadTest};
use forge::{NetworkContext, NetworkTest, Result, Swarm, Test};
use rand::seq::SliceRandom;
use rand::thread_rng;
use std::time::Duration;
use tokio::runtime::Runtime;
use tokio::time::Instant;

pub struct ValidatorRebootStressTest;

impl Test for ValidatorRebootStressTest {
    fn name(&self) -> &'static str {
        "validator reboot stress test"
    }
}

impl NetworkLoadTest for ValidatorRebootStressTest {
    fn setup(&self, _ctx: &mut NetworkContext) -> Result<LoadDestination> {
        Ok(LoadDestination::AllFullnodes)
    }

    fn test(&self, swarm: &mut dyn Swarm, duration: Duration) -> Result<()> {
        let start = Instant::now();
        let runtime = Runtime::new().unwrap();

        let all_validators = swarm.validators().map(|v| v.peer_id()).collect::<Vec<_>>();

        let mut rng = thread_rng();

        while start.elapsed() < duration {
            let validator_to_reboot = swarm
                .validator_mut(*all_validators.choose(&mut rng).unwrap())
                .unwrap();
            runtime.block_on(async { validator_to_reboot.stop().await })?;
            runtime.block_on(async { validator_to_reboot.start().await })?;
            std::thread::sleep(Duration::from_secs(10));
        }

        Ok(())
    }
}

impl NetworkTest for ValidatorRebootStressTest {
    fn run<'t>(&self, ctx: &mut NetworkContext<'t>) -> Result<()> {
        <dyn NetworkLoadTest>::run(self, ctx)
    }
}
