// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use crate::generate_traffic;
use aptos_logger::info;
use forge::{NetworkContext, NetworkTest, Result, SwarmExt, Test};
use std::time::Instant;
use tokio::{runtime::Runtime, time::Duration};

const MAX_FULLNODE_LAG_SECS: u64 = 10; // Max amount of lag (in seconds) that fullnodes should adhere to
const MAX_FULLNODE_SYNC_SECS: u64 = TRANSACTION_EMIT_SECS * 2; // Max time for all fullnodes to sync up from a fresh state
const TRANSACTION_EMIT_SECS: u64 = 300; // The duration to emit transactions for syncing

pub struct StateSyncPerformance;

impl Test for StateSyncPerformance {
    fn name(&self) -> &'static str {
        "StateSyncPerformance"
    }
}

impl NetworkTest for StateSyncPerformance {
    fn run<'t>(&self, ctx: &mut NetworkContext<'t>) -> Result<()> {
        // Generate some traffic through the fullnodes
        let all_fullnodes = ctx
            .swarm()
            .full_nodes()
            .map(|v| v.peer_id())
            .collect::<Vec<_>>();
        let _txn_stat = generate_traffic(
            ctx,
            &all_fullnodes,
            Duration::from_secs(TRANSACTION_EMIT_SECS),
            1,
        )?;
        info!("Finished generating the initial traffic.");

        // Wait for all nodes to catch up. We time bound this to ensure
        // fullnodes don't fall too far behind the validators.
        let runtime = Runtime::new().unwrap();
        runtime.block_on(async {
            ctx.swarm()
                .wait_for_all_nodes_to_catchup(
                    Instant::now() + Duration::from_secs(MAX_FULLNODE_LAG_SECS),
                )
                .await
        })?;
        info!("Validators and fullnodes are all synchronized.");

        // Stop and reset all fullnodes
        info!("Deleting all fullnode data!");
        for fullnode_id in &all_fullnodes {
            let fullnode = ctx.swarm().full_node_mut(*fullnode_id).unwrap();
            runtime.block_on(async { fullnode.stop().await })?;
            fullnode.clear_storage()?;
        }

        // Fetch the highest synced version from the swarm
        let highest_synced_version =
            runtime.block_on(async { ctx.swarm().get_highest_synced_version().await.unwrap_or(0) });
        if highest_synced_version == 0 {
            return Err(anyhow::format_err!(
                "The swarm has synced 0 versions! Something has gone wrong!"
            ));
        }
        info!(
            "Syncing to target version at : {:?}",
            highest_synced_version
        );

        // Restart the fullnodes so they start syncing from a fresh state
        for fullnode_id in &all_fullnodes {
            let fullnode = ctx.swarm().full_node_mut(*fullnode_id).unwrap();
            runtime.block_on(async { fullnode.start().await })?;
        }
        info!("Restarting all the fullnodes and waiting for them to catchup.");

        // Wait for all fullnodes to catch up to the highest synced version
        let timer = Instant::now();
        runtime.block_on(async {
            ctx.swarm()
                .wait_for_all_nodes_to_catchup(
                    Instant::now() + Duration::from_secs(MAX_FULLNODE_SYNC_SECS),
                )
                .await
        })?;
        let time_to_state_sync = timer.elapsed().as_secs();

        // Calculate the state sync throughput
        if time_to_state_sync == 0 {
            return Err(anyhow::format_err!(
                "The time taken to state sync was 0 seconds! Something has gone wrong!"
            ));
        }
        let state_sync_throughput = highest_synced_version as u64 / time_to_state_sync;

        // Report the state sync results
        let state_sync_throughput_message =
            format!("State sync throughput : {} txn/sec", state_sync_throughput);
        info!(
            "Measured state sync throughput: {:?}",
            state_sync_throughput_message
        );
        ctx.report.report_text(state_sync_throughput_message);
        ctx.report.report_metric(
            self.name(),
            "state_sync_throughput",
            state_sync_throughput as f64,
        );

        Ok(())
    }
}
