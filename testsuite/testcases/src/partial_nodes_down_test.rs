// Copyright (c) The Diem Core Contributors
// SPDX-License-Identifier: Apache-2.0

use crate::generate_traffic;
use forge::{NetworkContext, NetworkTest, Result, Test};
use std::thread;
use tokio::time::Duration;

pub struct PartialNodesDown;

impl Test for PartialNodesDown {
    fn name(&self) -> &'static str {
        "partialNodesDown::10%-down"
    }
}

impl NetworkTest for PartialNodesDown {
    fn run<'t>(&self, ctx: &mut NetworkContext<'t>) -> Result<()> {
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
            node.stop()?;
        }
        thread::sleep(Duration::from_secs(5));

        // Generate some traffic
        let txn_stat = generate_traffic(ctx, &up_nodes, duration)?;
        ctx.report
            .report_txn_stats(self.name().to_string(), txn_stat, duration);
        for n in &down_nodes {
            let node = ctx.swarm().validator_mut(*n).unwrap();
            println!("Node {} is going to restart", node.name());
            node.start()?;
        }
        thread::sleep(Duration::from_secs(5));

        Ok(())
    }
}
