// Copyright © Velor Foundation
// Parts of the project are originally copyright © Meta Platforms, Inc.
// SPDX-License-Identifier: Apache-2.0

use crate::generate_traffic;
use velor_forge::{NetworkContextSynchronizer, NetworkTest, Result, Test};
use async_trait::async_trait;
use std::{ops::DerefMut, thread};
use tokio::{runtime::Runtime, time::Duration};

pub struct PartialNodesDown;

impl Test for PartialNodesDown {
    fn name(&self) -> &'static str {
        "10%-down"
    }
}

#[async_trait]
impl NetworkTest for PartialNodesDown {
    async fn run<'a>(&self, ctx: NetworkContextSynchronizer<'a>) -> Result<()> {
        let mut ctx_locker = ctx.ctx.lock().await;
        let ctx = ctx_locker.deref_mut();
        let runtime = Runtime::new()?;
        let duration = Duration::from_secs(120);
        let all_validators = ctx
            .swarm
            .read()
            .await
            .validators()
            .map(|v| v.peer_id())
            .collect::<Vec<_>>();
        let mut down_nodes = all_validators.clone();
        let up_nodes = down_nodes.split_off(all_validators.len() / 10);
        for n in &down_nodes {
            let swarm = ctx.swarm.read().await;
            let node = swarm.validator(*n).unwrap();
            println!("Node {} is going to stop", node.name());
            runtime.block_on(node.stop())?;
        }
        thread::sleep(Duration::from_secs(5));

        // Generate some traffic
        let txn_stat = generate_traffic(ctx, &up_nodes, duration).await?;
        ctx.report
            .report_txn_stats(self.name().to_string(), &txn_stat);
        for n in &down_nodes {
            let swarm = ctx.swarm.read().await;
            let node = swarm.validator(*n).unwrap();
            println!("Node {} is going to restart", node.name());
            runtime.block_on(node.start())?;
        }

        Ok(())
    }
}
