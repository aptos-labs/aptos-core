// Copyright © Velor Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::NetworkLoadTest;
use velor_forge::{NetworkContextSynchronizer, NetworkTest, Result, Swarm, Test, TestReport};
use async_trait::async_trait;
use rand::{seq::SliceRandom, thread_rng};
use std::{sync::Arc, time::Duration};
use tokio::time::Instant;

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

#[async_trait]
impl NetworkLoadTest for ValidatorRebootStressTest {
    async fn test(
        &self,
        swarm: Arc<tokio::sync::RwLock<Box<dyn Swarm>>>,
        _report: &mut TestReport,
        duration: Duration,
    ) -> Result<()> {
        let start = Instant::now();

        let all_validators = {
            swarm
                .read()
                .await
                .validators()
                .map(|v| v.peer_id())
                .collect::<Vec<_>>()
        };

        while start.elapsed() < duration {
            let addresses: Vec<_> = {
                let mut rng = thread_rng();
                all_validators
                    .choose_multiple(&mut rng, self.num_simultaneously)
                    .cloned()
                    .collect()
            };
            for adr in &addresses {
                let swarm = swarm.read().await;
                let validator_to_reboot = swarm.validator(*adr).unwrap();
                validator_to_reboot.stop().await?;
            }
            if self.down_time_secs > 0.0 {
                tokio::time::sleep(Duration::from_secs_f32(self.down_time_secs)).await;
            }

            for adr in &addresses {
                let swarm = swarm.read().await;
                let validator_to_reboot = swarm.validator(*adr).unwrap();
                validator_to_reboot.start().await?;
            }

            if self.pause_secs > 0.0 {
                tokio::time::sleep(Duration::from_secs_f32(self.pause_secs)).await;
            }
        }

        Ok(())
    }
}

#[async_trait]
impl NetworkTest for ValidatorRebootStressTest {
    async fn run<'a>(&self, ctx: NetworkContextSynchronizer<'a>) -> Result<()> {
        <dyn NetworkLoadTest>::run(self, ctx).await
    }
}
