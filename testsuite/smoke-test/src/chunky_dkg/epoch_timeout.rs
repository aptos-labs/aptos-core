// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use super::shadow_mode::create_swarm_with_dkg_only;
use crate::utils::get_on_chain_resource;
use aptos_forge::{Node, NodeExt, SwarmExt};
use aptos_logger::info;
use aptos_types::dkg::chunky_dkg::ChunkyDKGState;
use std::time::Duration;

/// Enable chunky DKG in real V1 mode (no shadow grace period) AND set the
/// general epoch-timeout watchdog with a short grace period — all in one
/// governance script.
async fn enable_chunky_v1_and_watchdog(
    cli: &aptos::test::CliTestFramework,
    root_idx: usize,
    watchdog_grace_period_secs: u64,
) {
    let script = format!(
        r#"
script {{
    use aptos_std::fixed_point64;
    use aptos_framework::aptos_governance;
    use aptos_framework::chunky_dkg_config;
    use aptos_framework::epoch_timeout_config;
    use aptos_framework::features;

    fun main(core_resources: &signer) {{
        let framework_signer = aptos_governance::get_signer_testnet_only(core_resources, @0x1);

        // Chunky DKG V1 (real, no shadow grace).
        let chunky_cfg = chunky_dkg_config::new_v1(
            fixed_point64::create_from_rational(1, 2),
            fixed_point64::create_from_rational(2, 3),
        );
        chunky_dkg_config::set_for_next_epoch(&framework_signer, chunky_cfg);

        // Epoch watchdog: force-end after `n` seconds of stalled reconfig.
        let timeout_cfg = epoch_timeout_config::new_with_grace_period({});
        epoch_timeout_config::set_for_next_epoch(&framework_signer, timeout_cfg);

        // ENCRYPTED_TRANSACTIONS feature flag (108) — required for chunky DKG path.
        features::change_feature_flags_for_next_epoch(&framework_signer, vector[108], vector[]);

        aptos_governance::reconfigure(&framework_signer);
    }}
}}
"#,
        watchdog_grace_period_secs
    );
    cli.run_script(root_idx, &script)
        .await
        .expect("Failed to enable chunky V1 + watchdog via governance");
}

/// Test that the general epoch-timeout watchdog force-ends the epoch when
/// chunky DKG is stalled and there is no chunky shadow grace period.
///
/// Strategy:
/// 1. Bring up a swarm with DKG enabled (chunky DKG OFF).
/// 2. Activate a failpoint that makes chunky DKG ignore its start event.
/// 3. Via governance, enable chunky V1 (no shadow grace) AND set the
///    epoch-timeout watchdog with a short grace period.
/// 4. Verify epochs continue advancing — only possible because the watchdog
///    force-finalizes the in-progress reconfig.
#[tokio::test]
async fn epoch_timeout_watchdog_force_ends_epoch() {
    let epoch_duration_secs = 20;
    let watchdog_grace_period_secs = 30;

    let (swarm, cli, root_idx) = create_swarm_with_dkg_only(4, epoch_duration_secs).await;
    let client_endpoint = swarm.validators().nth(1).unwrap().rest_api_endpoint();
    let client = aptos_rest_client::Client::new(client_endpoint);

    swarm
        .wait_for_all_nodes_to_catchup_to_epoch(2, Duration::from_secs(epoch_duration_secs * 3))
        .await
        .expect("Waited too long for epoch 2.");

    let epoch_before = client
        .get_ledger_information()
        .await
        .expect("ledger info")
        .into_inner()
        .epoch;
    info!("Network stable at epoch {}", epoch_before);

    // Stall chunky DKG on all validators before enabling it.
    info!("Activating chunky_dkg::process_dkg_start_event failpoint on all validators...");
    for validator in swarm.validators() {
        validator
            .rest_client()
            .set_failpoint(
                "chunky_dkg::process_dkg_start_event".to_string(),
                "return".to_string(),
            )
            .await
            .expect("Failed to set failpoint");
    }

    info!(
        "Enabling chunky V1 + epoch watchdog (grace={}s) via governance...",
        watchdog_grace_period_secs
    );
    enable_chunky_v1_and_watchdog(&cli, root_idx, watchdog_grace_period_secs).await;

    // Wait for the epoch to advance past at least one regular boundary that
    // crosses the watchdog deadline. The reconfig is stuck (chunky DKG never
    // completes); only the watchdog can finalize it.
    let target_epoch = epoch_before + 2;
    let time_limit_secs = epoch_duration_secs * 2 + watchdog_grace_period_secs + 60;
    let timer = tokio::time::Instant::now();
    let mut reached_target = false;

    info!(
        "Waiting for epoch to reach {} (stuck chunky DKG, watchdog should force-end)...",
        target_epoch
    );
    while timer.elapsed().as_secs() < time_limit_secs {
        let ledger = client
            .get_ledger_information()
            .await
            .expect("ledger info")
            .into_inner();
        let dkg_state = get_on_chain_resource::<ChunkyDKGState>(&client).await;
        info!(
            "epoch={} chunky_in_progress={} chunky_completed={} elapsed={}s",
            ledger.epoch,
            dkg_state.in_progress.is_some(),
            dkg_state.last_completed.is_some(),
            timer.elapsed().as_secs(),
        );
        if ledger.epoch >= target_epoch {
            reached_target = true;
            break;
        }
        tokio::time::sleep(Duration::from_secs(5)).await;
    }

    assert!(
        reached_target,
        "Epoch did not advance past {} within {}s — watchdog did not fire as expected",
        target_epoch, time_limit_secs,
    );

    let dkg_state = get_on_chain_resource::<ChunkyDKGState>(&client).await;
    assert!(
        dkg_state.last_completed.is_none(),
        "Chunky DKG should not have completed (failpoint blocks it)"
    );
    info!("Watchdog force-ended the epoch despite stalled chunky DKG");
}
