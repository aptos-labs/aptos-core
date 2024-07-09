// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::smoke_test_environment::SwarmBuilder;
use aptos::{common::types::GasOptions, move_tool::MemberId, test::CliTestFramework};
use aptos_forge::{NodeExt, Swarm, SwarmExt};
use aptos_logger::info;
use aptos_types::on_chain_config::OnChainRandomnessConfig;
use serde::{Deserialize, Serialize};
use std::{collections::BTreeMap, str::FromStr, sync::Arc, time::Duration};
use std::ops::Mul;
use std::time::Instant;
use diesel::CombineDsl;
use rand::rngs::OsRng;
use rand::thread_rng;
use tokio::time::sleep;
use aptos_crypto::bls12381;
use aptos_types::mpc::MPCState;
use crate::utils::get_on_chain_resource;
use ff::Field;
use group::{Curve, Group};
use group::prime::PrimeCurveAffine;
use aptos::common::types::{CliTypedResult, TransactionSummary};

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

    let rest_cli = swarm.validators().next().unwrap().rest_client();

    info!("Wait for epoch 2.");
    swarm
        .wait_for_all_nodes_to_catchup_to_epoch(2, Duration::from_secs(epoch_duration_secs * 2))
        .await
        .expect("Epoch 2 taking too long to arrive!");

    let root_address = swarm.chain_info().root_account().address();
    info!("Root account: {}", root_address);
    let _root_idx = cli.add_account_with_address_to_cli(swarm.root_key(), root_address);

    info!("Publishing a testing module.");
    publish_module(&mut cli, 0).await;

    info!("Trigger raises for P and P^2.");
    let my_esk_0 = blstrs::Scalar::from(55555);
    let my_epk_0 = blstrs::G1Affine::generator().mul(&my_esk_0);
    let my_epk_1 = my_epk_0.double();
    let my_epk_0_bytes = my_epk_0.to_compressed().to_vec();
    let my_epk_1_bytes = my_epk_1.to_compressed().to_vec();
    trigger_raise(&cli, 0, my_epk_0_bytes).await.unwrap();
    trigger_raise(&cli, 0, my_epk_1_bytes).await.unwrap();
    let account = cli.account_id(0).to_hex_literal();

    let tasks = rest_cli
        .get_account_resource_bcs::<PendingResults>(
            root_address,
            format!("{}::mpc_example::PendingResults", account).as_str(),
        )
        .await
        .unwrap()
        .into_inner();

    let t0 = tasks.tasks[0] as usize;
    let t1 = tasks.tasks[1] as usize;

    tokio::time::sleep(Duration::from_secs(10)).await;
    let mpc_state = get_on_chain_resource::<MPCState>(&rest_cli).await;
    let result_0_bytes = <[u8; 48]>::try_from(mpc_state.tasks[t0].result.clone().unwrap()).unwrap();
    let result_1_bytes = <[u8; 48]>::try_from(mpc_state.tasks[t1].result.clone().unwrap()).unwrap();
    let result_0 = blstrs::G1Affine::from_compressed(&result_0_bytes).unwrap();
    let result_1 = blstrs::G1Affine::from_compressed(&result_1_bytes).unwrap();
    let two = blstrs::Scalar::from(2);
    let result_0_doubled = result_0.mul(two).to_affine();
    assert_eq!(result_0_doubled, result_1);
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
        include_str!("../../../aptos-move/move-examples/mpc/sources/mpc_example.move")
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

async fn trigger_raise(aptos_cli: &CliTestFramework, user_idx: usize, arg: Vec<u8>) -> CliTypedResult<TransactionSummary> {
    let account = aptos_cli.account_id(user_idx).to_hex_literal();
    let trigger_func_id = MemberId::from_str(&format!("{}::mpc_example::trigger_raise", account)).unwrap();

    let gas_options = GasOptions {
        gas_unit_price: Some(100),
        max_gas: Some(10_000),
        expiration_secs: 60,
    };

    let arg_str = format!("hex:0x{}", hex::encode(arg));
    aptos_cli
        .run_function(user_idx, Some(gas_options), trigger_func_id.clone(), vec![arg_str.as_str()], vec![])
        .await
}

#[derive(Deserialize, Serialize)]
struct PendingResults {
    tasks: Vec<u64>,
}
