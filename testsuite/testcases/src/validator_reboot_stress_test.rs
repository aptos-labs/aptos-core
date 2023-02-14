// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use crate::{LoadDestination, NetworkLoadTest};
use aptos_forge::{NetworkContext, NetworkTest, Result, Swarm, Test};
use rand::{seq::SliceRandom, thread_rng};
use std::time::Duration;
use tokio::{runtime::Runtime, time::Instant};

pub struct ValidatorRebootStressTest {
    pub num_simultaneously: usize,
    pub down_time_secs: f32,
    pub pause_secs: f32,
}

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
            let addresses: Vec<_> = all_validators
                .choose_multiple(&mut rng, self.num_simultaneously)
                .cloned()
                .collect();
            for adr in &addresses {
                let validator_to_reboot = swarm.validator_mut(*adr).unwrap();
                runtime.block_on(async { validator_to_reboot.stop().await })?;
            }
            if self.down_time_secs > 0.0 {
                std::thread::sleep(Duration::from_secs_f32(self.down_time_secs));
            }

            for adr in &addresses {
                let validator_to_reboot = swarm.validator_mut(*adr).unwrap();
                runtime.block_on(async { validator_to_reboot.start().await })?;
            }

            if self.pause_secs > 0.0 {
                std::thread::sleep(Duration::from_secs_f32(self.pause_secs));
            }
        }

        Ok(())
    }
}

impl NetworkTest for ValidatorRebootStressTest {
    fn run<'t>(&self, ctx: &mut NetworkContext<'t>) -> Result<()> {
        <dyn NetworkLoadTest>::run(self, ctx)
    }
}
