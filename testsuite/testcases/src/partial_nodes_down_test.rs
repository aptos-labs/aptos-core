// Copyright © Aptos Foundation
// Parts of the project are originally copyright © Meta Platforms, Inc.
// SPDX-License-Identifier: Apache-2.0

use crate::generate_traffic;
use aptos_forge::{NetworkContext, NetworkTest, Result, Test};
use std::thread;
use tokio::{runtime::Runtime, time::Duration};

pub struct PartialNodesDown;

impl Test for PartialNodesDown {
    fn name(&self) -> &'static str {
        "10%-down"
    }
}

impl NetworkTest for PartialNodesDown {
    fn run(&self, ctx: &mut NetworkContext<'_>) -> Result<()> {
        let runtime = Runtime::new()?;
        let duration = Duration::from_secs(120);
        let all_validators = ctx
            .swarm()
            .validators()
            .map(|v| v.peer_id())
            .collect::<Vec<_>>();
        let mut down_nodes = all_validators.clone();
        let up_nodes = down_nodes.split_off(all_validators.len() / 10);
        for n in &down_nodes {
            let node = ctx.swarm().validator_mut(*n).unwrap();
            println!("Node {} is going to stop", node.name());
            runtime.block_on(node.stop())?;
        }
        thread::sleep(Duration::from_secs(5));

        // Generate some traffic
        let txn_stat = generate_traffic(ctx, &up_nodes, duration)?;
        ctx.report
            .report_txn_stats(self.name().to_string(), &txn_stat);
        for n in &down_nodes {
            let node = ctx.swarm().validator_mut(*n).unwrap();
            println!("Node {} is going to restart", node.name());
            runtime.block_on(node.start())?;
        }

        Ok(())
    }
}
