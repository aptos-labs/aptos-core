// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::smoke_test_environment::SwarmBuilder;
use aptos::{common::types::GasOptions, move_tool::MemberId, test::CliTestFramework};
use aptos_forge::{NodeExt, Swarm, SwarmExt};
use aptos_logger::info;
use aptos_types::on_chain_config::{OnChainRandomnessConfig, StorageGasSchedule};
use serde::{Deserialize, Serialize};
use std::{collections::BTreeMap, str::FromStr, sync::Arc, time::Duration};
use rand::thread_rng;
use aptos_crypto::ed25519::{Ed25519PrivateKey, Ed25519PublicKey};
use aptos_crypto::Uniform;
use crate::randomness::get_on_chain_resource;

/// Publish the `on-chain-dice` example module,
/// run its function that consume on-chain randomness, and
/// print out the random results.
#[tokio::test]
async fn e2e_basic_consumption() {
    let epoch_duration_secs = 20;

    let (mut swarm, mut cli, faucet) = SwarmBuilder::new_local(4)
        .with_num_fullnodes(1)
        .with_aptos()
        .with_init_genesis_config(Arc::new(move |conf| {
            conf.epoch_duration_secs = epoch_duration_secs;

            // Ensure randomness is enabled.
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

    let mut rng = thread_rng();
    let new_sk = Ed25519PrivateKey::generate(&mut rng);
    let new_pk = Ed25519PublicKey::from(&new_sk);
    let user_idx = cli.add_account_to_cli(new_sk);
    let user_addr = cli.account_id(user_idx);
    let mut info = swarm.aptos_public_info();
    info.create_user_account(&new_pk).await.unwrap();
    info.mint(user_addr, 999999999).await.unwrap();

    info!("Publishing OnChainDice module.");
    publish_on_chain_dice_module(&mut cli, user_idx).await;

    let storage_gas = get_on_chain_resource::<StorageGasSchedule>(&rest_client).await;
    println!("storage_gas={:?}", storage_gas);

    info!("Rolling the dice.");
    let roll_func_id = MemberId::from_str(&format!("{}::dice::roll", user_addr)).unwrap();
    let mut dice_roll_history = vec![];
    for _ in 0..10 {
        let gas_options = GasOptions {
            gas_unit_price: Some(1),
            max_gas: Some(1_000_000),
            expiration_secs: 60,
        };
        let txn_summary = cli
            .run_function(user_idx, Some(gas_options), roll_func_id.clone(), vec![], vec![])
            .await
            .unwrap();
        info!("Roll txn summary: {:?}", txn_summary);

        let dice_roll_result = rest_client
            .get_account_resource_bcs::<DiceRollResult>(
                user_addr,
                format!("{}::dice::DiceRollResult", user_addr).as_str(),
            )
            .await
            .unwrap()
            .into_inner();
        dice_roll_history.push(dice_roll_result.roll);
    }


    info!("Roll history: {:?}", dice_roll_history);
    assert!(false);
}

#[derive(Deserialize, Serialize)]
struct DiceRollResult {
    roll: u64,
}

async fn publish_on_chain_dice_module(cli: &mut CliTestFramework, publisher_account_idx: usize) {
    cli.init_move_dir();
    let mut package_addresses = BTreeMap::new();
    package_addresses.insert("module_owner", "_");

    cli.init_package(
        "OnChainDice".to_string(),
        package_addresses,
        Some(CliTestFramework::aptos_framework_dir()),
    )
    .await
    .unwrap();

    let content =
        include_str!("../../../../aptos-move/move-examples/on_chain_dice/sources/dice.move")
            .to_string();
    cli.add_file_in_package("sources/dice.move", content);

    cli.wait_for_account(publisher_account_idx).await.unwrap();

    info!("Move package dir: {}", cli.move_dir().display());

    let mut named_addresses = BTreeMap::new();
    let account_str = cli.account_id(publisher_account_idx).to_string();
    named_addresses.insert("module_owner", account_str.as_str());
    cli.publish_package(0, None, named_addresses, None)
        .await
        .unwrap();
}
