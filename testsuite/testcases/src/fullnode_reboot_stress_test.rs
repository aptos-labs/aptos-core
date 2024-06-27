// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{LoadDestination, NetworkLoadTest};
use aptos_forge::{
    NetworkContext, NetworkContextSynchronizer, NetworkTest, Result, Swarm, Test, TestReport,
};
use async_trait::async_trait;
use rand::{seq::SliceRandom, thread_rng};
use std::{sync::Arc, time::Duration};
use tokio::time::Instant;

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
        let start = Instant::now();

        let all_fullnodes = {
            swarm
                .read()
                .await
                .full_nodes()
                .map(|v| v.peer_id())
                .collect::<Vec<_>>()
        };

        while start.elapsed() < duration {
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
            tokio::time::sleep(Duration::from_secs(10)).await;
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
