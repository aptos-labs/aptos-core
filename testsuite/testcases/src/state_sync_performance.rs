// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use crate::generate_traffic;
use anyhow::bail;
use aptos_logger::info;
use forge::{get_highest_synced_version, NetworkContext, NetworkTest, Result, SwarmExt, Test};
use std::time::Instant;
use tokio::{runtime::Runtime, time::Duration};

const MAX_NODE_LAG_SECS: u64 = 10; // Max amount of lag (in seconds) that nodes should adhere to

/// A state sync performance test that measures fullnode sync performance.
/// In the test, all fullnodes are wiped, restarted and timed to synchronize.
pub struct StateSyncFullnodePerformance;

impl Test for StateSyncFullnodePerformance {
    fn name(&self) -> &'static str {
        "StateSyncFullnodePerformance"
    }
}

impl NetworkTest for StateSyncFullnodePerformance {
    fn run<'t>(&self, ctx: &mut NetworkContext<'t>) -> Result<()> {
        // Verify we have at least 1 fullnode
        let all_fullnodes = ctx
            .swarm()
            .full_nodes()
            .map(|v| v.peer_id())
            .collect::<Vec<_>>();
        if all_fullnodes.is_empty() {
            return Err(anyhow::format_err!(
                "Fullnode performance tests require at least 1 fullnode!"
            ));
        }

        // Log the test setup
        let all_validators = ctx
            .swarm()
            .validators()
            .map(|v| v.peer_id())
            .collect::<Vec<_>>();
        info!(
            "Running state sync test {:?} with {:?} validators and {:?} fullnodes.",
            self.name(),
            all_validators.len(),
            all_fullnodes.len()
        );

        // Generate some traffic through the fullnodes.
        // We do this for half the test time.
        let emit_txn_duration = ctx.global_duration.checked_div(2).unwrap();
        info!(
            "Generating the initial traffic for {:?} seconds.",
            emit_txn_duration.as_secs()
        );
        let _txn_stat = generate_traffic(
            ctx,
            &all_fullnodes,
            emit_txn_duration,
            aptos_global_constants::GAS_UNIT_PRICE,
        )?;

        // Wait for all nodes to synchronize. We time bound this to ensure
        // fullnodes don't fall too far behind the validators.
        info!("Waiting for the validators and fullnodes to be synchronized.");
        let runtime = Runtime::new().unwrap();
        runtime.block_on(async {
            ctx.swarm()
                .wait_for_all_nodes_to_catchup(Duration::from_secs(MAX_NODE_LAG_SECS))
                .await
        })?;

        // Stop and reset all fullnodes
        info!("Deleting all fullnode data!");
        for fullnode_id in &all_fullnodes {
            let fullnode = ctx.swarm().full_node_mut(*fullnode_id).unwrap();
            runtime.block_on(async { fullnode.clear_storage().await })?;
        }

        // Restart the nodes so they start syncing from a fresh state
        // and start the timer.
        for fullnode_id in &all_fullnodes {
            let fullnode = ctx.swarm().full_node_mut(*fullnode_id).unwrap();
            runtime.block_on(async { fullnode.start().await })?;
        }
        let timer = Instant::now();

        // Wait for all nodes to catch up to the highest synced version
        // then calculate and display the throughput results.
        ensure_state_sync_throughput(ctx, timer, self.name())
    }
}

/// A state sync performance test that measures validator sync performance.
/// In the test, 2 validators are wiped, restarted and timed to synchronize.
pub struct StateSyncValidatorPerformance;

impl Test for StateSyncValidatorPerformance {
    fn name(&self) -> &'static str {
        "StateSyncValidatorPerformance"
    }
}

impl NetworkTest for StateSyncValidatorPerformance {
    fn run<'t>(&self, ctx: &mut NetworkContext<'t>) -> Result<()> {
        // Verify we have at least 7 validators (i.e., 3f+1, where f is 2)
        // so we can kill 2 validators but still make progress.
        let all_validators = ctx
            .swarm()
            .validators()
            .map(|v| v.peer_id())
            .collect::<Vec<_>>();
        let num_validators = all_validators.len();
        if num_validators < 7 {
            return Err(anyhow::format_err!(
                "State sync validator performance tests require at least 7 validators! Given: {:?} \
                 This is to ensure the chain can still make progress when 2 validators are killed.",
                num_validators
            ));
        }

        // Log the test setup
        info!(
            "Running state sync test {:?} with {:?} validators.",
            self.name(),
            num_validators,
        );

        // Generate some traffic through the validators.
        // We do this for half the test time.
        let emit_txn_duration = ctx.global_duration.checked_div(2).unwrap();
        info!(
            "Generating the initial traffic for {:?} seconds.",
            emit_txn_duration.as_secs()
        );
        let _txn_stat = generate_traffic(
            ctx,
            &all_validators,
            emit_txn_duration,
            aptos_global_constants::GAS_UNIT_PRICE,
        )?;

        // Wait for all nodes to synchronize and stabilize.
        info!("Waiting for the validators to be synchronized.");
        let runtime = Runtime::new().unwrap();
        runtime.block_on(async {
            ctx.swarm()
                .wait_for_all_nodes_to_catchup(Duration::from_secs(MAX_NODE_LAG_SECS))
                .await
        })?;

        // Stop and reset two validators
        info!("Deleting data for two validators!");
        let validators_to_restart = &all_validators[0..2];
        for validator_id in validators_to_restart {
            let validator = ctx.swarm().validator_mut(*validator_id).unwrap();
            runtime.block_on(async { validator.clear_storage().await })?;
        }

        // Restart the validators so they start syncing from a fresh state
        // and start the timer.
        for validator_id in validators_to_restart {
            let validator = ctx.swarm().validator_mut(*validator_id).unwrap();
            runtime.block_on(async { validator.start().await })?;
        }
        let timer = Instant::now();

        // Wait for all nodes to catch up to the highest synced version
        // then calculate and display the throughput results.
        ensure_state_sync_throughput(ctx, timer, self.name())
    }
}

/// Calculates, enforces and displays the state sync throughput using
/// the synced version and sync duration.
fn ensure_state_sync_throughput(
    ctx: &mut NetworkContext<'_>,
    timer: Instant,
    test_name: &str,
) -> Result<()> {
    // Get the highest synced version for the chain
    let runtime = Runtime::new().unwrap();
    let highest_synced_version = runtime.block_on(async {
        get_highest_synced_version(&ctx.swarm().get_all_nodes_clients_with_names())
            .await
            .unwrap_or(0)
    });
    if highest_synced_version == 0 {
        return Err(anyhow::format_err!(
            "The swarm has synced 0 versions! Something has gone wrong!"
        ));
    }

    // Wait for all nodes to catch up to the same synced version.
    // We allow up to half the test time to do this.
    let node_sync_duration = ctx.global_duration.checked_div(2).unwrap();
    runtime.block_on(async {
        ctx.swarm()
            .wait_for_all_nodes_to_catchup(node_sync_duration)
            .await
    })?;

    // Calculate the state sync throughput
    let seconds_to_sync = timer.elapsed().as_secs();
    if seconds_to_sync == 0 {
        return Err(anyhow::format_err!(
            "The time taken to state sync was 0 seconds! Something has gone wrong!"
        ));
    }
    let state_sync_throughput = highest_synced_version as u64 / seconds_to_sync;

    // Report the state sync results
    let throughput_message = format!("State sync throughput : {} txn/sec", state_sync_throughput);
    info!("Measured state sync throughput: {:?}", throughput_message);
    ctx.report.report_text(throughput_message);
    ctx.report.report_metric(
        test_name,
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
