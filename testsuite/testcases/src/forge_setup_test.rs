// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::generate_traffic;
use anyhow::Context;
use aptos_config::config::OverrideNodeConfig;
use aptos_forge::{NetworkContextSynchronizer, NetworkTest, Result, Test};
use async_trait::async_trait;
use log::info;
use rand::{
    rngs::{OsRng, StdRng},
    seq::IteratorRandom,
    Rng, SeedableRng,
};
use std::{ops::DerefMut, thread, time::Duration};

const STATE_SYNC_VERSION_COUNTER_NAME: &str = "aptos_state_sync_version";

pub struct ForgeSetupTest;

impl Test for ForgeSetupTest {
    fn name(&self) -> &'static str {
        "verify_forge_setup"
    }
}

#[async_trait]
impl NetworkTest for ForgeSetupTest {
    async fn run<'a>(&self, ctx: NetworkContextSynchronizer<'a>) -> Result<()> {
        let mut rng = StdRng::from_seed(OsRng.gen());
        let mut ctx_locker = ctx.ctx.lock().await;
        let ctx = ctx_locker.deref_mut();

        // TODO: decrease lock shadow on swarm for this test
        {
            let swarm = ctx.swarm.read().await;

            let all_fullnodes = swarm.full_nodes().map(|v| v.peer_id()).collect::<Vec<_>>();
            let fullnode_id = all_fullnodes.iter().choose(&mut rng).unwrap();

            info!("Pick one fullnode to stop and wipe");
            let fullnode = swarm.full_node(*fullnode_id).unwrap();
            fullnode.clear_storage().await?;
            fullnode.start().await?;

            let fullnode = swarm.full_node(*fullnode_id).unwrap();
            let fullnode_name = fullnode.name();

            for _ in 0..10 {
                let query = format!(
                    "{}{{instance=\"{}\",type=\"synced\"}}",
                    STATE_SYNC_VERSION_COUNTER_NAME, &fullnode_name
                );
                info!("PromQL Query {}", query);
                let r = swarm.query_metrics(&query, None, None).await?;
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
        }

        // add some PFNs and send load to them
        let mut pfns = Vec::new();
        let num_pfns = 5;
        {
            let mut swarm = ctx.swarm.write().await;
            for _ in 0..num_pfns {
                let pfn_version = swarm.versions().max().unwrap();
                let pfn_node_config =
                    OverrideNodeConfig::new_with_default_base(swarm.get_default_pfn_node_config());
                let pfn_peer_id = swarm.add_full_node(&pfn_version, pfn_node_config).await?;

                let _pfn = swarm.full_node(pfn_peer_id).context("pfn not found")?;
                pfns.push(pfn_peer_id);
            }
        }

        let duration = Duration::from_secs(10 * num_pfns);
        let txn_stat = generate_traffic(ctx, &pfns, duration).await?;

        ctx.report
            .report_txn_stats(self.name().to_string(), &txn_stat);

        Ok(())
    }
}
