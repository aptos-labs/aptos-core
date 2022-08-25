// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use crate::smoke_test_environment::SwarmBuilder;
use aptos::account::create::DEFAULT_FUNDED_COINS;
use aptos_keygen::KeyGen;

#[tokio::test]
async fn test_account_flow() {
    let (_swarm, mut cli, _faucet) = SwarmBuilder::new_local(1)
        .with_aptos()
        .build_with_cli(2)
        .await;

    cli.assert_account_balance_now(0, DEFAULT_FUNDED_COINS)
        .await;
    cli.assert_account_balance_now(1, DEFAULT_FUNDED_COINS)
        .await;

    let transfer_amount = 100;
    let response = cli
        .transfer_coins(0, 1, transfer_amount, None)
        .await
        .unwrap();
    let expected_sender_amount =
        DEFAULT_FUNDED_COINS - (response.gas_used * response.gas_unit_price) - transfer_amount;
    let expected_receiver_amount = DEFAULT_FUNDED_COINS + transfer_amount;

    // transfer_coins already waits for transaction to be committed
    cli.assert_account_balance_now(0, expected_sender_amount)
        .await;
    cli.assert_account_balance_now(1, expected_receiver_amount)
        .await;

    let expected_sender_amount = expected_sender_amount + DEFAULT_FUNDED_COINS;
    let _ = cli.fund_account(0, None).await.unwrap();
    // fund_account already waits for transaction to be committed
    cli.assert_account_balance_now(0, expected_sender_amount)
        .await;

    // Create another cli account:
    cli.create_cli_account_from_faucet(KeyGen::from_os_rng().generate_ed25519_private_key(), None)
        .await
        .unwrap();
    cli.assert_account_balance_now(2, DEFAULT_FUNDED_COINS)
        .await;
}
