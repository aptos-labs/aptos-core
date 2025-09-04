// Copyright © Velor Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::smoke_test_environment::SwarmBuilder;
use velor::{common::types::GasOptions, move_tool::MemberId, test::CliTestFramework};
use velor_forge::{NodeExt, Swarm, SwarmExt};
use velor_logger::info;
use velor_types::on_chain_config::OnChainRandomnessConfig;
use serde::{Deserialize, Serialize};
use std::{collections::BTreeMap, str::FromStr, sync::Arc, time::Duration};

/// Publish the `on-chain-dice` example module,
/// run its function that consume on-chain randomness, and
/// print out the random results.
#[tokio::test]
async fn e2e_basic_consumption() {
    let epoch_duration_secs = 20;

    let (swarm, mut cli, _faucet) = SwarmBuilder::new_local(4)
        .with_num_fullnodes(1)
        .with_velor()
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

    let root_address = swarm.chain_info().root_account().address();
    info!("Root account: {}", root_address);
    let _root_idx = cli.add_account_with_address_to_cli(swarm.root_key(), root_address);

    info!("Publishing OnChainDice module.");
    publish_on_chain_dice_module(&mut cli, 0).await;

    info!("Rolling the dice.");
    let account = cli.account_id(0).to_hex_literal();
    let roll_func_id = MemberId::from_str(&format!("{}::dice::roll", account)).unwrap();
    for _ in 0..10 {
        let gas_options = GasOptions {
            gas_unit_price: Some(100),
            max_gas: Some(10_000), // should match the default required gas deposit.
            expiration_secs: 60,
        };
        let txn_summary = cli
            .run_function(0, Some(gas_options), roll_func_id.clone(), vec![], vec![])
            .await
            .unwrap();
        info!("Roll txn summary: {:?}", txn_summary);
    }

    info!("Collecting roll history.");
    let dice_roll_history = rest_client
        .get_account_resource_bcs::<DiceRollHistory>(
            root_address,
            format!("{}::dice::DiceRollHistory", account).as_str(),
        )
        .await
        .unwrap()
        .into_inner();

    info!("Roll history: {:?}", dice_roll_history.rolls);
}

#[derive(Deserialize, Serialize)]
struct DiceRollHistory {
    rolls: Vec<u64>,
}

pub async fn publish_on_chain_dice_module(
    cli: &mut CliTestFramework,
    publisher_account_idx: usize,
) {
    cli.init_move_dir();
    let mut package_addresses = BTreeMap::new();
    package_addresses.insert("module_owner", "_");

    cli.init_package(
        "OnChainDice".to_string(),
        package_addresses,
        Some(CliTestFramework::velor_framework_dir()),
    )
    .await
    .unwrap();

    let content =
        include_str!("../../../../velor-move/move-examples/on_chain_dice/sources/dice.move")
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
