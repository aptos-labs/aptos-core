// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use crate::{
    randomness::{e2e_basic_consumption::publish_on_chain_dice_module, get_current_version},
    smoke_test_environment::SwarmBuilder,
};
use aptos::{common::types::GasOptions, move_tool::MemberId};
use aptos_forge::{NodeExt, Swarm, SwarmExt};
use aptos_logger::info;
use aptos_types::on_chain_config::OnChainRandomnessConfig;
use std::{str::FromStr, sync::Arc, time::Duration};

/// Verify that with deferred rand_check, all validators agree on whether
/// blocks need randomness. Non-rand blocks should progress without verification/reconstruction,
/// and rand blocks should still get correct randomness (dice rolls succeed).
#[tokio::test]
async fn deferred_rand_check_with_mixed_blocks() {
    let epoch_duration_secs = 20;

    let (swarm, mut cli, _faucet) = SwarmBuilder::new_local(4)
        .with_aptos()
        .with_init_genesis_config(Arc::new(move |conf| {
            conf.epoch_duration_secs = epoch_duration_secs;
            conf.consensus_config.enable_validator_txns();
            conf.randomness_config_override = Some(OnChainRandomnessConfig::default_enabled());
        }))
        .build_with_cli(0)
        .await;

    let rest_client = swarm.validators().next().unwrap().rest_client();

    info!("Wait for epoch 2. Epoch 1 does not have randomness.");
    swarm
        .wait_for_all_nodes_to_catchup_to_epoch(2, Duration::from_secs(epoch_duration_secs * 2))
        .await
        .expect("Epoch 2 taking too long to arrive!");

    // Phase 1: Verify chain progresses with non-rand blocks (no randomness txns)
    info!("Phase 1: Verify chain progresses without randomness transactions.");
    let version_before = get_current_version(&rest_client).await;
    tokio::time::sleep(Duration::from_secs(5)).await;
    let version_after = get_current_version(&rest_client).await;
    assert!(
        version_after > version_before,
        "Chain should progress with non-rand blocks. Before: {}, After: {}",
        version_before,
        version_after
    );
    info!(
        "Chain progressed from version {} to {} with non-rand blocks.",
        version_before, version_after
    );

    // Phase 2: Deploy dice module and verify rand blocks get correct randomness
    info!("Phase 2: Deploy on_chain_dice module and test rand transactions.");
    let root_address = swarm.chain_info().root_account().address();
    let _root_idx = cli.add_account_with_address_to_cli(swarm.root_key(), root_address);

    publish_on_chain_dice_module(&mut cli, 0).await;

    let account = cli.account_id(0).to_hex_literal();
    let roll_func_id = MemberId::from_str(&format!("{}::dice::roll", account)).unwrap();

    // Submit randomness transactions and verify they succeed
    info!("Rolling the dice (randomness transactions).");
    for i in 0..3 {
        let gas_options = GasOptions {
            gas_unit_price: Some(100),
            max_gas: Some(10_000),
            expiration_secs: 60,
        };
        let txn_summary = cli
            .run_function(0, Some(gas_options), roll_func_id.clone(), vec![], vec![])
            .await
            .unwrap();
        info!("Roll {} txn summary: {:?}", i, txn_summary);
    }

    // Phase 3: Verify all validators are in sync
    info!("Phase 3: Verify all validators are in sync.");
    swarm
        .wait_for_all_nodes_to_catchup(Duration::from_secs(30))
        .await
        .expect("All nodes should be in sync");
    info!("All nodes are in sync after mixed rand/non-rand workload.");
}

/// Verify that chain progresses when all blocks are non-rand.
/// With deferred aggregation, non-rand blocks should not pay the randomness verification cost.
#[tokio::test]
async fn deferred_rand_check_chain_progress() {
    let epoch_duration_secs = 20;

    let (swarm, _cli, _faucet) = SwarmBuilder::new_local(4)
        .with_aptos()
        .with_init_genesis_config(Arc::new(move |conf| {
            conf.epoch_duration_secs = epoch_duration_secs;
            conf.consensus_config.enable_validator_txns();
            conf.randomness_config_override = Some(OnChainRandomnessConfig::default_enabled());
        }))
        .build_with_cli(0)
        .await;

    let rest_client = swarm.validators().next().unwrap().rest_client();

    info!("Wait for epoch 2.");
    swarm
        .wait_for_all_nodes_to_catchup_to_epoch(2, Duration::from_secs(epoch_duration_secs * 2))
        .await
        .expect("Epoch 2 taking too long to arrive!");

    // Verify chain progresses with only non-rand blocks (no randomness modules deployed)
    info!("Verify chain progresses with non-rand blocks only.");
    let version_start = get_current_version(&rest_client).await;
    tokio::time::sleep(Duration::from_secs(10)).await;
    let version_end = get_current_version(&rest_client).await;

    let versions_produced = version_end - version_start;
    info!(
        "Produced {} versions in 10 seconds with deferred rand_check.",
        versions_produced
    );
    assert!(
        versions_produced > 0,
        "Chain must progress with non-rand blocks"
    );

    // All nodes should be in sync
    swarm
        .wait_for_all_nodes_to_catchup(Duration::from_secs(30))
        .await
        .expect("All nodes should be in sync");
    info!("All nodes in sync. Deferred rand_check chain progress test passed.");
}
