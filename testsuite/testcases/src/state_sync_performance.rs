// Copyright © Aptos Foundation
// Parts of the project are originally copyright © Meta Platforms, Inc.
// SPDX-License-Identifier: Apache-2.0

use crate::generate_traffic;
use anyhow::bail;
use aptos_forge::{
    get_highest_synced_epoch, get_highest_synced_version, NetworkContext, NetworkTest, Result,
    SwarmExt, Test,
};
use aptos_logger::info;
use aptos_sdk::move_types::account_address::AccountAddress;
use std::time::Instant;
use tokio::{runtime::Runtime, time::Duration};

const MAX_EPOCH_CHANGE_SECS: u64 = 300; // Max amount of time (in seconds) to wait for an epoch change
const MAX_NODE_LAG_SECS: u64 = 30; // Max amount of lag (in seconds) that nodes should adhere to
const NUM_STATE_VALUE_COUNTER_NAME: &str = "aptos_jellyfish_leaf_count"; // The metric to fetch for the number of state values

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
        let all_fullnodes = get_fullnodes_and_check_setup(ctx, self.name())?;

        // Emit a lot of traffic and ensure the fullnodes can all sync
        emit_traffic_and_ensure_bounded_sync(ctx, &all_fullnodes)?;

        // Stop and reset the fullnodes so they start syncing from genesis
        stop_and_reset_nodes(ctx, &all_fullnodes, &[])?;

        // Wait for all nodes to catch up to the highest synced version
        // then calculate and display the throughput results.
        ensure_state_sync_transaction_throughput(ctx, self.name())
    }
}

/// A state sync performance test that measures fast sync performance.
/// In the test, all fullnodes are wiped, restarted and timed to synchronize.
pub struct StateSyncFullnodeFastSyncPerformance;

impl Test for StateSyncFullnodeFastSyncPerformance {
    fn name(&self) -> &'static str {
        "StateSyncFullnodeFastSyncPerformance"
    }
}

impl NetworkTest for StateSyncFullnodeFastSyncPerformance {
    fn run<'t>(&self, ctx: &mut NetworkContext<'t>) -> Result<()> {
        let all_fullnodes = get_fullnodes_and_check_setup(ctx, self.name())?;

        // Emit a lot of traffic and ensure the fullnodes can all sync
        emit_traffic_and_ensure_bounded_sync(ctx, &all_fullnodes)?;

        // Wait for an epoch change to ensure fast sync can download all the latest states
        info!("Waiting for an epoch change.");
        let runtime = Runtime::new().unwrap();
        runtime.block_on(async {
            ctx.swarm()
                .wait_for_all_nodes_to_change_epoch(Duration::from_secs(MAX_EPOCH_CHANGE_SECS))
                .await
        })?;

        // Get the highest known epoch in the chain
        let highest_synced_epoch = runtime.block_on(async {
            get_highest_synced_epoch(&ctx.swarm().get_all_nodes_clients_with_names())
                .await
                .unwrap_or(0)
        });
        if highest_synced_epoch == 0 {
            return Err(anyhow::format_err!(
                "The swarm has synced 0 epochs! Something has gone wrong!"
            ));
        }

        // Fetch the number of state values held on-chain
        let fullnode_name = ctx.swarm().full_nodes().next().unwrap().name();
        let prom_query = format!(
            "{}{{instance=\"{}\"}}",
            NUM_STATE_VALUE_COUNTER_NAME, &fullnode_name
        );
        let promql_result = runtime.block_on(ctx.swarm().query_metrics(&prom_query, None, None))?;
        let number_of_state_values = match promql_result.as_instant().unwrap().first() {
            Some(instant_vector) => instant_vector.sample().value() as u64,
            None => {
                return Err(anyhow::format_err!(
                    "No instant vectors found for prom query {}",
                    prom_query
                ));
            },
        };
        info!(
            "Number of reported state values found on-chain is: {}",
            number_of_state_values
        );

        // Stop and reset the fullnodes so they start syncing from genesis
        stop_and_reset_nodes(ctx, &all_fullnodes, &[])?;

        // Wait for all nodes to catch up to the highest synced epoch
        // then calculate and display the throughput results.
        display_state_sync_state_throughput(
            ctx,
            self.name(),
            highest_synced_epoch,
            number_of_state_values,
        )?;

        // TODO: add a minimum expected throughput that could fail the test
        Ok(())
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
        emit_traffic_and_ensure_bounded_sync(ctx, &all_validators)?;

        // Stop and reset two validators so they start syncing from genesis
        info!("Deleting data for two validators!");
        let validators_to_reset = &all_validators[0..2];
        stop_and_reset_nodes(ctx, &[], validators_to_reset)?;

        // Wait for all nodes to catch up to the highest synced version
        // then calculate and display the throughput results.
        ensure_state_sync_transaction_throughput(ctx, self.name())
    }
}

/// Verifies the setup for the given fullnode test and returns the
/// set of fullnodes.
fn get_fullnodes_and_check_setup(
    ctx: &mut NetworkContext,
    test_name: &'static str,
) -> Result<Vec<AccountAddress>> {
    // Verify we have at least 1 fullnode
    let all_fullnodes = ctx
        .swarm()
        .full_nodes()
        .map(|v| v.peer_id())
        .collect::<Vec<_>>();
    if all_fullnodes.is_empty() {
        return Err(anyhow::format_err!(
            "Fullnode test {} requires at least 1 fullnode!",
            test_name
        ));
    }

    // Log the test setup
    info!(
        "Running state sync test {:?} with {:?} validators and {:?} fullnodes.",
        test_name,
        ctx.swarm().validators().count(),
        all_fullnodes.len()
    );

    Ok(all_fullnodes)
}

/// Emits traffic through all specified nodes and ensures all nodes can
/// sync within a reasonable time bound.
fn emit_traffic_and_ensure_bounded_sync(
    ctx: &mut NetworkContext,
    nodes_to_send_traffic: &[AccountAddress],
) -> Result<()> {
    // Generate some traffic through the specified nodes.
    // We do this for half the test time.
    let emit_txn_duration = ctx.global_duration.checked_div(2).unwrap();
    info!(
        "Generating the initial traffic for {:?} seconds.",
        emit_txn_duration.as_secs()
    );
    let _txn_stat = generate_traffic(ctx, nodes_to_send_traffic, emit_txn_duration)?;

    // Wait for all nodes to synchronize. We time bound this to ensure
    // nodes don't fall too far behind.
    info!("Waiting for the validators and fullnodes to be synchronized.");
    Runtime::new().unwrap().block_on(async {
        ctx.swarm()
            .wait_for_all_nodes_to_catchup(Duration::from_secs(MAX_NODE_LAG_SECS))
            .await
    })?;

    Ok(())
}

/// Stops and resets all specified nodes
fn stop_and_reset_nodes(
    ctx: &mut NetworkContext,
    fullnodes_to_reset: &[AccountAddress],
    validators_to_reset: &[AccountAddress],
) -> Result<()> {
    let runtime = Runtime::new().unwrap();

    // Stop and reset all fullnodes
    info!("Deleting all fullnode data!");
    for fullnode_id in fullnodes_to_reset {
        let fullnode = ctx.swarm().full_node_mut(*fullnode_id).unwrap();
        runtime.block_on(async { fullnode.clear_storage().await })?;
    }

    // Stop and reset all validators
    info!("Deleting all validator data!");
    for valdiator_id in validators_to_reset {
        let validator = ctx.swarm().validator_mut(*valdiator_id).unwrap();
        runtime.block_on(async { validator.clear_storage().await })?;
    }

    // Restart the fullnodes so they start syncing from a fresh state
    for fullnode_id in fullnodes_to_reset {
        let fullnode = ctx.swarm().full_node_mut(*fullnode_id).unwrap();
        runtime.block_on(async { fullnode.start().await })?;
    }

    // Restart the validators so they start syncing from a fresh state
    for valdiator_id in validators_to_reset {
        let validator = ctx.swarm().validator_mut(*valdiator_id).unwrap();
        runtime.block_on(async { validator.start().await })?;
    }

    Ok(())
}

/// Calculates and displays the state sync state value throughput
/// when fast syncing to the latest epoch.
fn display_state_sync_state_throughput(
    ctx: &mut NetworkContext<'_>,
    test_name: &str,
    highest_synced_epoch: u64,
    number_of_state_values: u64,
) -> Result<()> {
    // Start the timer
    let timer = Instant::now();
    let runtime = Runtime::new().unwrap();

    // Wait for all nodes to catch up to the same epoch (that is when fast sync completes).
    // We allow up to half the test time to do this.
    let node_sync_duration = ctx.global_duration.checked_div(2).unwrap();
    runtime.block_on(async {
        ctx.swarm()
            .wait_for_all_nodes_to_catchup_to_epoch(highest_synced_epoch, node_sync_duration)
            .await
    })?;

    // Stop the syncing timer
    let seconds_to_sync = timer.elapsed().as_secs();
    if seconds_to_sync == 0 {
        return Err(anyhow::format_err!(
            "The time taken to state sync was 0 seconds! Something has gone wrong!"
        ));
    }

    // Calculate and report the syncing throughput
    let state_sync_throughput = number_of_state_values / seconds_to_sync;
    let throughput_message = format!(
        "State sync throughput : {} state values / sec",
        state_sync_throughput
    );
    info!("Measured state sync throughput: {:?}", throughput_message);
    ctx.report.report_text(throughput_message);
    ctx.report.report_metric(
        test_name,
        "state_sync_throughput",
        state_sync_throughput as f64,
    );

    Ok(())
}

/// Calculates, enforces and displays the state sync transaction
/// throughput using the synced version and sync duration.
fn ensure_state_sync_transaction_throughput(
    ctx: &mut NetworkContext<'_>,
    test_name: &str,
) -> Result<()> {
    // Start the timer
    let timer = Instant::now();

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
    let state_sync_throughput = highest_synced_version / seconds_to_sync;

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
    let min_expected_tps = ctx.success_criteria.min_avg_tps as u64;
    if state_sync_throughput < min_expected_tps {
        let error_message = format!(
            "State sync TPS requirement failed. Average TPS: {}, minimum required TPS: {}",
            state_sync_throughput, min_expected_tps
        );
        bail!(error_message)
    }

    Ok(())
}
