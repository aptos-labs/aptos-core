// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::smoke_test_environment::SwarmBuilder;
use aptos::{common::types::GasOptions, move_tool::MemberId, test::CliTestFramework};
use aptos_forge::{NodeExt, Swarm, SwarmExt};
use aptos_logger::info;
use aptos_types::on_chain_config::OnChainRandomnessConfig;
use serde::{Deserialize, Serialize};
use std::{collections::BTreeMap, str::FromStr, sync::Arc, time::Duration};
use std::collections::btree_map::BTreeMap;
use std::time::Instant;
use rand::thread_rng;
use tokio::time::sleep;
use aptos_crypto::bls12381;

/// Publish the `on-chain-dice` example module,
/// run its function that consume on-chain randomness, and
/// print out the random results.
#[tokio::test]
async fn raise_by_secret() {
    let epoch_duration_secs = 20;

    let (swarm, mut cli, _faucet) = SwarmBuilder::new_local(4)
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

    let root_address = swarm.chain_info().root_account().address();
    info!("Root account: {}", root_address);
    let _root_idx = cli.add_account_with_address_to_cli(swarm.root_key(), root_address);

    info!("Publishing OnChainDice module.");
    publish_module(&mut cli, 0).await;

    info!("Trigger raise.");
    let account = cli.account_id(0).to_hex_literal();
    let trigger_func_id = MemberId::from_str(&format!("{}::mpc_example::trigger_raise", account)).unwrap();
    let mut rng = thread_rng();

    let gas_options = GasOptions {
        gas_unit_price: Some(100),
        max_gas: Some(10_000),
        expiration_secs: 60,
    };
    let element = aptos_crypto::rand_bls12381_g1(&mut rng);

    let txn_summary = cli
        .run_function(0, Some(gas_options), trigger_func_id.clone(), vec![], vec![])
        .await
        .unwrap();
    println!("txn_summary={:?}", txn_summary);

    info!("Wait for correct result.");
    let timer = Instant::now();
    let time_limit = Duration::from_secs(10);
    while timer.elapsed() < time_limit {
        let vrfy_func_id = MemberId::from_str(&format!("{}::mpc_example::fetch_and_verify", account)).unwrap();
        let gas_options = GasOptions {
            gas_unit_price: Some(100),
            max_gas: Some(10_000),
            expiration_secs: 60,
        };
        let txn_summary = cli
            .run_function(0, Some(gas_options), vrfy_func_id.clone(), vec![], vec![])
            .await
            .unwrap();
        println!("txn_summary={:?}", txn_summary);

        sleep(Duration::from_secs(1)).await;
    }
}

#[derive(Deserialize, Serialize)]
struct DiceRollHistory {
    rolls: Vec<u64>,
}

async fn publish_module(
    cli: &mut CliTestFramework,
    publisher_account_idx: usize,
) {
    cli.init_move_dir();
    let mut package_addresses = BTreeMap::new();
    package_addresses.insert("module_owner", "_");

    cli.init_package(
        "MpcExample".to_string(),
        package_addresses,
        Some(CliTestFramework::aptos_framework_dir()),
    )
        .await
        .unwrap();

    let content =
        include_str!("../../../../aptos-move/move-examples/mpc/sources/mpc_example.move")
            .to_string();
    cli.add_file_in_package("sources/mpc_example.move", content);

    cli.wait_for_account(publisher_account_idx).await.unwrap();

    info!("Move package dir: {}", cli.move_dir().display());

    let mut named_addresses = BTreeMap::new();
    let account_str = cli.account_id(publisher_account_idx).to_string();
    named_addresses.insert("module_owner", account_str.as_str());
    cli.publish_package(0, None, named_addresses, None)
        .await
        .unwrap();
}
