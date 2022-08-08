// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use crate::smoke_test_environment::SwarmBuilder;
use aptos::account::create::DEFAULT_FUNDED_COINS;
use aptos::common::types::{GasOptions, DEFAULT_GAS_UNIT_PRICE, DEFAULT_MAX_GAS};

#[tokio::test]
async fn test_account_flow() {
    let (_swarm, cli, _faucet) = SwarmBuilder::new_local(1)
        .with_aptos()
        .build_with_cli(2)
        .await;

    assert_eq!(DEFAULT_FUNDED_COINS, cli.account_balance(0).await.unwrap());
    assert_eq!(DEFAULT_FUNDED_COINS, cli.account_balance(1).await.unwrap());

    // Transfer an amount between the accounts
    let transfer_amount = 100;
    let response = cli
        .transfer_coins(
            0,
            1,
            transfer_amount,
            Some(GasOptions {
                gas_unit_price: DEFAULT_GAS_UNIT_PRICE * 2,
                max_gas: DEFAULT_MAX_GAS,
            }),
        )
        .await
        .unwrap();
    let expected_sender_amount =
        DEFAULT_FUNDED_COINS - (response.gas_used * response.gas_unit_price) - transfer_amount;
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
    let _ = cli.fund_account(0, None).await.unwrap();
    assert_eq!(
        expected_sender_amount,
        cli.wait_for_balance(0, expected_sender_amount)
            .await
            .unwrap()
    );
}
