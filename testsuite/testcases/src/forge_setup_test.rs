// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::generate_traffic;
use anyhow::Context;
use aptos_config::config::OverrideNodeConfig;
use aptos_forge::{NetworkContext, NetworkTest, Result, Test};
use aptos_logger::info;
use rand::{
    rngs::{OsRng, StdRng},
    seq::IteratorRandom,
    Rng, SeedableRng,
};
use std::{thread, time::Duration};
use tokio::runtime::Runtime;

const STATE_SYNC_VERSION_COUNTER_NAME: &str = "aptos_state_sync_version";

pub struct ForgeSetupTest;

impl Test for ForgeSetupTest {
    fn name(&self) -> &'static str {
        "verify_forge_setup"
    }
}

impl NetworkTest for ForgeSetupTest {
    fn run(&self, ctx: &mut NetworkContext<'_>) -> Result<()> {
        let mut rng = StdRng::from_seed(OsRng.gen());
        let runtime = Runtime::new().unwrap();

        let swarm = ctx.swarm();

        let all_fullnodes = swarm.full_nodes().map(|v| v.peer_id()).collect::<Vec<_>>();
        let fullnode_id = all_fullnodes.iter().choose(&mut rng).unwrap();

        info!("Pick one fullnode to stop and wipe");
        let fullnode = swarm.full_node_mut(*fullnode_id).unwrap();
        runtime.block_on(fullnode.clear_storage())?;
        runtime.block_on(fullnode.start())?;

        let fullnode = swarm.full_node(*fullnode_id).unwrap();
        let fullnode_name = fullnode.name();

        for _ in 0..10 {
            let query = format!(
                "{}{{instance=\"{}\",type=\"synced\"}}",
                STATE_SYNC_VERSION_COUNTER_NAME, &fullnode_name
            );
            info!("PromQL Query {}", query);
            let r = runtime.block_on(swarm.query_metrics(&query, None, None))?;
            let ivs = r.as_instant().unwrap();
            for iv in ivs {
                info!(
                    "{}: {}",
                    STATE_SYNC_VERSION_COUNTER_NAME,
                    iv.sample().value()
                );
            }
            thread::sleep(std::time::Duration::from_secs(5));
        }

        // add some PFNs and send load to them
        let mut pfns = Vec::new();
        let num_pfns = 5;
        for _ in 0..num_pfns {
            let pfn_version = swarm.versions().max().unwrap();
            let pfn_node_config =
                OverrideNodeConfig::new_with_default_base(swarm.get_default_pfn_node_config());
            let pfn_peer_id =
                runtime.block_on(swarm.add_full_node(&pfn_version, pfn_node_config))?;

            let _pfn = swarm.full_node(pfn_peer_id).context("pfn not found")?;
            pfns.push(pfn_peer_id);
        }

        let duration = Duration::from_secs(10 * num_pfns);
        let txn_stat = generate_traffic(ctx, &pfns, duration)?;

        ctx.report
            .report_txn_stats(self.name().to_string(), &txn_stat);

        Ok(())
    }
}
