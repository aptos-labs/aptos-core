// Copyright © Velor Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{LoadDestination, NetworkLoadTest};
use velor_forge::{
    NetworkContext, NetworkContextSynchronizer, NetworkTest, Result, Swarm, Test, TestReport,
};
use async_trait::async_trait;
use rand::{seq::SliceRandom, thread_rng};
use std::{sync::Arc, time::Duration};
use tokio::time::Instant;

// The buffer (in seconds) at the end of the test to allow for graceful shutdown
const END_OF_TEST_BUFFER_SECS: u64 = 60;

// The wait time (in seconds) between fullnode reboots
const WAIT_TIME_BETWEEN_REBOOTS_SECS: u64 = 10;

pub struct FullNodeRebootStressTest;

impl Test for FullNodeRebootStressTest {
    fn name(&self) -> &'static str {
        "fullnode reboot stress test"
    }
}

#[async_trait]
impl NetworkLoadTest for FullNodeRebootStressTest {
    async fn setup<'a>(&self, _ctx: &mut NetworkContext<'a>) -> Result<LoadDestination> {
        Ok(LoadDestination::AllFullnodes)
    }

    async fn test(
        &self,
        swarm: Arc<tokio::sync::RwLock<Box<dyn Swarm>>>,
        _report: &mut TestReport,
        duration: Duration,
    ) -> Result<()> {
        // Start the test timer
        let start = Instant::now();

        // Ensure the total test duration is at least as long as the buffer
        let end_of_test_buffer = Duration::from_secs(END_OF_TEST_BUFFER_SECS);
        if duration <= end_of_test_buffer {
            panic!(
                "Total test duration must be at least: {:?}! Given duration: {:?}",
                end_of_test_buffer, duration
            );
        }

        // Collect all the fullnodes
        let all_fullnodes = {
            swarm
                .read()
                .await
                .full_nodes()
                .map(|v| v.peer_id())
                .collect::<Vec<_>>()
        };

        // Reboot fullnodes until the test duration is reached
        let test_reboot_duration = duration - end_of_test_buffer;
        while start.elapsed() < test_reboot_duration {
            {
                let swarm = swarm.read().await;
                let fullnode_to_reboot = {
                    let mut rng = thread_rng();
                    swarm
                        .full_node(*all_fullnodes.choose(&mut rng).unwrap())
                        .unwrap()
                };
                fullnode_to_reboot.stop().await?;
                fullnode_to_reboot.start().await?;
            }
            tokio::time::sleep(Duration::from_secs(WAIT_TIME_BETWEEN_REBOOTS_SECS)).await;
        }

        Ok(())
    }
}

#[async_trait]
impl NetworkTest for FullNodeRebootStressTest {
    async fn run<'a>(&self, ctx: NetworkContextSynchronizer<'a>) -> Result<()> {
        <dyn NetworkLoadTest>::run(self, ctx).await
    }
}
