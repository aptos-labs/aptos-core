// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use crate::generate_traffic;
use anyhow::bail;
use aptos_logger::info;
use forge::{get_highest_synced_version, NetworkContext, NetworkTest, Result, SwarmExt, Test};
use std::time::Instant;
use tokio::{runtime::Runtime, time::Duration};

const MAX_FULLNODE_LAG_SECS: u64 = 10; // Max amount of lag (in seconds) that fullnodes should adhere to

pub struct StateSyncPerformance;

impl Test for StateSyncPerformance {
    fn name(&self) -> &'static str {
        "StateSyncPerformance"
    }
}

impl NetworkTest for StateSyncPerformance {
    fn run<'t>(&self, ctx: &mut NetworkContext<'t>) -> Result<()> {
        let emit_txn_duration = ctx.global_duration; // How long we'll emit txns for
        let fullnode_sync_duration = emit_txn_duration.saturating_mul(2); // Limits state sync to half txn throughput

        // Generate some traffic through the fullnodes
        info!(
            "Generating the initial traffic for {:?} seconds.",
            emit_txn_duration.as_secs()
        );
        let all_fullnodes = ctx
            .swarm()
            .full_nodes()
            .map(|v| v.peer_id())
            .collect::<Vec<_>>();
        let _txn_stat = generate_traffic(ctx, &all_fullnodes, emit_txn_duration, 1)?;

        // Wait for all nodes to catch up. We time bound this to ensure
        // fullnodes don't fall too far behind the validators.
        info!("Waiting for the validators and fullnodes to be synchronized.");
        let runtime = Runtime::new().unwrap();
        runtime.block_on(async {
            ctx.swarm()
                .wait_for_all_nodes_to_catchup(Duration::from_secs(MAX_FULLNODE_LAG_SECS))
                .await
        })?;

        // Stop and reset all fullnodes
        info!("Deleting all fullnode data!");
        for fullnode_id in &all_fullnodes {
            let fullnode = ctx.swarm().full_node_mut(*fullnode_id).unwrap();
            runtime.block_on(async { fullnode.clear_storage().await })?;
        }

        // Fetch the highest synced version from the swarm
        let highest_synced_version = runtime.block_on(async {
            get_highest_synced_version(&ctx.swarm().get_clients_with_names())
                .await
                .unwrap_or(0)
        });
        if highest_synced_version == 0 {
            return Err(anyhow::format_err!(
                "The swarm has synced 0 versions! Something has gone wrong!"
            ));
        }
        info!("Syncing to target version at: {:?}", highest_synced_version);

        // Restart the fullnodes so they start syncing from a fresh state
        for fullnode_id in &all_fullnodes {
            let fullnode = ctx.swarm().full_node_mut(*fullnode_id).unwrap();
            runtime.block_on(async { fullnode.start().await })?;
        }

        // Wait for all fullnodes to catch up to the highest synced version
        info!("Restarting all the fullnodes and waiting for them to catchup.");
        let timer = Instant::now();
        runtime.block_on(async {
            ctx.swarm()
                .wait_for_all_nodes_to_catchup(fullnode_sync_duration)
                .await
        })?;
        let duration_to_state_sync = timer.elapsed();
        let seconds_to_state_sync = duration_to_state_sync.as_secs();

        // Calculate the state sync throughput
        if seconds_to_state_sync == 0 {
            return Err(anyhow::format_err!(
                "The time taken to state sync was 0 seconds! Something has gone wrong!"
            ));
        }
        let state_sync_throughput = highest_synced_version as u64 / seconds_to_state_sync;

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

        // TODO: we fetch the TPS requirement from the given success criteria.
        // But, we should probably make it more generic to avoid this.
        // Ensure we meet the success criteria.
        let min_expected_tps = ctx.success_criteria.avg_tps as u64;
        if state_sync_throughput < min_expected_tps {
            let error_message = format!(
                "State sync TPS requirement failed. Average TPS: {}, minimum required TPS: {}",
                state_sync_throughput, min_expected_tps
            );
            bail!(error_message)
        }

        Ok(())
    }
}
