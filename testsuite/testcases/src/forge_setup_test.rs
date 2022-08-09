// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use std::thread;

use aptos_logger::info;
use forge::{FullNode, NetworkContext, NetworkTest, Result, Test};
use rand::{
    rngs::{OsRng, StdRng},
    seq::IteratorRandom,
    Rng, SeedableRng,
};
use tokio::runtime::Runtime;

const STATE_SYNC_VERSION_COUNTER_NAME: &str = "aptos_state_sync_version";

pub struct ForgeSetupTest;

impl Test for ForgeSetupTest {
    fn name(&self) -> &'static str {
        "verify_forge_setup"
    }
}

async fn wipe_fullnode(fullnode: &mut dyn FullNode) -> Result<()> {
    fullnode.stop().await?;
    info!("Clear its storage");
    fullnode.clear_storage()?;
    info!("Start it up again");
    if let Err(e) = fullnode.start().await {
        info!("Error on fullnode startup: {}", e);
    } else {
        info!("Fullnode started successfully");
    }
    Ok(())
}

impl NetworkTest for ForgeSetupTest {
    fn run<'t>(&self, ctx: &mut NetworkContext<'t>) -> Result<()> {
        let mut rng = StdRng::from_seed(OsRng.gen());
        let runtime = Runtime::new().unwrap();

        let swarm = ctx.swarm();

        let all_fullnodes = swarm.full_nodes().map(|v| v.peer_id()).collect::<Vec<_>>();
        let fullnode_id = all_fullnodes.iter().choose(&mut rng).unwrap();

        info!("Pick one fullnode to stop");
        let fullnode = swarm.full_node_mut(*fullnode_id).unwrap();
        runtime.block_on(wipe_fullnode(fullnode))?;

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

        Ok(())
    }
}
