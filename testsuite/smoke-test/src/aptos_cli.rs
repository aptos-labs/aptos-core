// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use crate::smoke_test_environment::new_local_swarm_with_aptos;
use aptos::{account::create::DEFAULT_FUNDED_COINS, test::CliTestFramework};
use aptos_config::keys::ConfigKey;
use aptos_crypto::ed25519::Ed25519PrivateKey;
use aptos_faucet::FaucetArgs;
use aptos_types::{account_config::aptos_root_address, chain_id::ChainId};
use forge::{LocalSwarm, Node};
use tokio::task::JoinHandle;

pub async fn setup_test(num_nodes: usize) -> (LocalSwarm, CliTestFramework) {
    let swarm = new_local_swarm_with_aptos(num_nodes).await;
    let chain_id = swarm.chain_id();
    let validator = swarm.validators().next().unwrap();
    let root_key = swarm.root_key();
    let _ = launch_faucet(validator.rest_api_endpoint(), root_key, chain_id);

    // Connect the operator tool to the node's JSON RPC API
    let tool = CliTestFramework::new(
        validator.rest_api_endpoint(),
        "http://localhost:9996".parse().unwrap(),
        2,
    )
    .await;

    (swarm, tool)
}

fn launch_faucet(
    endpoint: reqwest::Url,
    mint_key: Ed25519PrivateKey,
    chain_id: ChainId,
) -> JoinHandle<()> {
    let faucet = FaucetArgs {
        address: "127.0.0.1".to_string(),
        port: 9996,
        server_url: endpoint.to_string(),
        mint_key_file_path: "".to_string(),
        mint_key: Some(ConfigKey::new(mint_key)),
        mint_account_address: Some(aptos_root_address()),
        chain_id,
        maximum_amount: None,
        do_not_delegate: true,
    };
    tokio::spawn(faucet.run())
}

#[tokio::test]
async fn test_account_flow() {
    let (_swarm, cli) = setup_test(1).await;

    assert_eq!(DEFAULT_FUNDED_COINS, cli.account_balance(0).await.unwrap());
    assert_eq!(DEFAULT_FUNDED_COINS, cli.account_balance(1).await.unwrap());

    // Transfer an amount between the accounts
    let transfer_amount = 100;
    let response = cli.transfer_coins(0, 1, transfer_amount).await.unwrap();
    let expected_sender_amount =
        DEFAULT_FUNDED_COINS - response.gas_used.unwrap() - transfer_amount;
    let expected_receiver_amount = DEFAULT_FUNDED_COINS + transfer_amount;

    assert_eq!(
        expected_sender_amount,
        cli.wait_for_balance(0, expected_sender_amount)
            .await
            .unwrap()
    );
    assert_eq!(
        expected_receiver_amount,
        cli.wait_for_balance(1, expected_receiver_amount)
            .await
            .unwrap()
    );

    // Wait for faucet amount to be updated
    let expected_sender_amount = expected_sender_amount + DEFAULT_FUNDED_COINS;
    let _ = cli.fund_account(0).await.unwrap();
    assert_eq!(
        expected_sender_amount,
        cli.wait_for_balance(0, expected_sender_amount)
            .await
            .unwrap()
    );
}
