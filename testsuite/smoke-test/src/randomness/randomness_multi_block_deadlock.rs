// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use super::e2e_basic_consumption::publish_on_chain_dice_module;
use crate::smoke_test_environment::SwarmBuilder;
use aptos::common::types::GasOptions;
use aptos_forge::{NodeExt, Swarm, SwarmExt};
use aptos_logger::info;
use aptos_move_cli::MemberId;
use aptos_types::on_chain_config::{
    ConsensusAlgorithmConfig, OnChainConsensusConfig, OnChainRandomnessConfig, ValidatorTxnConfig,
    DEFAULT_WINDOW_SIZE,
};
use std::{str::FromStr, sync::Arc, time::Duration};

/// Regression test for the multi-block batch deadlock introduced in PR #18699.
///
/// The deadlock occurs when:
/// 1. A block with `has_rand_txns_fut = true` appears as a non-last block
///    in a multi-block ordering batch.
/// 2. Later blocks' `has_rand_txns_fut` waits for earlier blocks' `execute_fut`,
///    which waits for `rand_rx`, which is only sent after the entire batch is
///    dequeued from the rand manager.
///
/// This test reproduces the conditions by:
/// - Enabling randomness and deploying a randomness-consuming module
/// - Submitting randomness transactions to make `has_rand_txns_fut = true`
/// - Stopping a validator to cause round timeouts, producing multi-block
///   ordering batches (skipped rounds → 2+ blocks per batch)
/// - Verifying the chain continues making progress (no deadlock)
#[tokio::test]
async fn randomness_multi_block_deadlock() {
    let epoch_duration_secs = 30;

    let (mut swarm, mut cli, _faucet) = SwarmBuilder::new_local(4)
        .with_num_fullnodes(1)
        .with_aptos()
        .with_init_genesis_config(Arc::new(move |conf| {
            conf.epoch_duration_secs = epoch_duration_secs;
            conf.consensus_config.enable_validator_txns();
            conf.randomness_config_override = Some(OnChainRandomnessConfig::default_enabled());
            // Disable order vote to use 3-chain commit rule instead of 2-chain.
            // This produces larger multi-block ordering batches (3 blocks per
            // batch instead of 2), making it easier to reproduce the deadlock.
            conf.consensus_config = OnChainConsensusConfig::V5 {
                alg: ConsensusAlgorithmConfig::JolteonV2 {
                    main: Default::default(),
                    quorum_store_enabled: true,
                    order_vote_enabled: false,
                },
                vtxn: ValidatorTxnConfig::default_for_genesis(),
                window_size: DEFAULT_WINDOW_SIZE,
                rand_check_enabled: true,
            };
        }))
        .build_with_cli(0)
        .await;

    let rest_client = swarm.validators().next().unwrap().rest_client();

    info!("[deadlock-test] Wait for epoch 2 (randomness activates).");
    swarm
        .wait_for_all_nodes_to_catchup_to_epoch(2, Duration::from_secs(epoch_duration_secs * 2))
        .await
        .expect("Epoch 2 taking too long to arrive!");

    // Deploy randomness-consuming module
    let root_address = swarm.chain_info().root_account().address();
    let _root_idx = cli.add_account_with_address_to_cli(swarm.root_key(), root_address);
    info!("[deadlock-test] Publishing OnChainDice module.");
    publish_on_chain_dice_module(&mut cli, 0).await;

    // Submit some randomness transactions so blocks have has_rand_txns_fut = true
    let account = cli.account_id(0).to_hex_literal();
    let roll_func_id = MemberId::from_str(&format!("{}::dice::roll", account)).unwrap();

    info!("[deadlock-test] Submitting randomness transactions (baseline).");
    for _ in 0..3 {
        let gas_options = GasOptions {
            gas_unit_price: Some(100),
            max_gas: Some(10_000),
            expiration_secs: 60,
        };
        cli.run_function(0, Some(gas_options), roll_func_id.clone(), vec![], vec![])
            .await
            .unwrap();
    }

    // Stop one validator to cause round timeouts.
    // With 4 validators and 1 down, consensus still works (3/4 > 2/3)
    // but rounds proposed by the stopped validator will timeout,
    // creating multi-block ordering batches (skipped rounds).
    let validator_to_stop = swarm.validators().last().unwrap().peer_id();
    info!(
        "[deadlock-test] Stopping validator {} to cause round timeouts.",
        validator_to_stop
    );
    swarm
        .validator_mut(validator_to_stop)
        .unwrap()
        .stop()
        .unwrap();

    // Now submit randomness transactions while a validator is down.
    // This creates the deadlock conditions: randomness txns + multi-block batches.
    info!("[deadlock-test] Submitting randomness transactions with validator down.");
    for i in 0..10 {
        let gas_options = GasOptions {
            gas_unit_price: Some(100),
            max_gas: Some(10_000),
            expiration_secs: 60,
        };
        match cli
            .run_function(0, Some(gas_options), roll_func_id.clone(), vec![], vec![])
            .await
        {
            Ok(summary) => info!("[deadlock-test] Roll {} succeeded: {:?}", i, summary),
            Err(e) => info!("[deadlock-test] Roll {} failed (may retry): {:?}", i, e),
        }
    }

    // Verify chain is still making progress (no deadlock)
    info!("[deadlock-test] Verifying chain progress after randomness txns with validator down.");
    let version_before = super::get_current_version(&rest_client).await;
    tokio::time::sleep(Duration::from_secs(10)).await;
    let version_after = super::get_current_version(&rest_client).await;
    assert!(
        version_after > version_before,
        "Chain stalled (deadlock)! Version stuck at {}. \
         This indicates the multi-block batch deadlock from PR #18699.",
        version_before,
    );
    info!(
        "[deadlock-test] Chain progressed from version {} to {} — no deadlock.",
        version_before, version_after
    );

    // Restart the stopped validator and verify recovery
    info!("[deadlock-test] Restarting stopped validator.");
    swarm
        .validator_mut(validator_to_stop)
        .unwrap()
        .start()
        .unwrap();
    swarm
        .validator_mut(validator_to_stop)
        .unwrap()
        .wait_until_healthy(Duration::from_secs(30))
        .await
        .unwrap();

    info!("[deadlock-test] Test passed — no multi-block batch deadlock.");
}
