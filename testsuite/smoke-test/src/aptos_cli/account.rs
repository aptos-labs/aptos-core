// Copyright © Velor Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::smoke_test_environment::SwarmBuilder;
use velor::{account::create::DEFAULT_FUNDED_COINS, common::types::GasOptions};
use velor_crypto::{PrivateKey, ValidCryptoMaterialStringExt};
use velor_keygen::KeyGen;

#[tokio::test]
async fn test_account_flow() {
    let (_swarm, mut cli, _faucet) = SwarmBuilder::new_local(1)
        .with_velor()
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
    let expected_sender_amount = DEFAULT_FUNDED_COINS - (response.octa_spent()) - transfer_amount;
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

    // Test gas options
    // Override gas unit price should use it instead of the estimated one
    let summary = cli
        .transfer_coins(
            2,
            1,
            5,
            Some(GasOptions {
                gas_unit_price: Some(2),
                max_gas: None,
                expiration_secs: 30,
            }),
        )
        .await
        .unwrap();
    assert_eq!(2, summary.gas_unit_price);

    let new_expected_balance = DEFAULT_FUNDED_COINS - summary.octa_spent() - 5;

    cli.assert_account_balance_now(2, new_expected_balance)
        .await;

    // Setting max gas skips simulation (this should fail for too little gas units, but be charged gas)
    // If it was simulated, it wouldn't charge gas, and it would need to be caught by the VM.  Mempool
    // submission doesn't check max gas is correct, just that the user has enough to pay it
    cli.transfer_coins(
        2,
        1,
        5,
        Some(GasOptions {
            gas_unit_price: None,
            // NOTE(Gas): This should be equal to the min gas amount allowed.
            //            Read the comment above to understand why.
            max_gas: Some(2),
            expiration_secs: 30,
        }),
    )
    .await
    .unwrap_err();

    // The previous transaction resulted in gas spent, so the balance of the account
    // should be lower than it was before (<), unless the gas price is zero, in which
    // case it will be the same (hence <=).
    assert!(cli.account_balance_now(2).await.unwrap() <= new_expected_balance);
}

#[tokio::test]
async fn test_account_key_rotation() {
    let (_swarm, mut cli, _faucet) = SwarmBuilder::new_local(1)
        .with_velor()
        .build_with_cli(2)
        .await;
    let account_id = cli.account_id(0);
    let original_public_key = cli.private_key(0).public_key();
    assert_eq!(
        cli.lookup_address(&original_public_key).await.unwrap(),
        account_id
    );

    let mut keygen = KeyGen::from_seed([9u8; 32]);
    let new_private_key = keygen.generate_ed25519_private_key();
    cli.rotate_key(0, new_private_key.to_encoded_string().unwrap(), None)
        .await
        .unwrap();
    // Ensure account id in framework is still the same
    assert_eq!(account_id, cli.account_id(0));

    // Original should still work
    assert_eq!(
        cli.lookup_address(&original_public_key).await.unwrap(),
        account_id
    );
    // And new one should work
    assert_eq!(
        cli.lookup_address(&new_private_key.public_key())
            .await
            .unwrap(),
        account_id
    );

    // And now a transfer with the old key should not work
    cli.transfer_coins(0, 1, 5, None)
        .await
        .expect_err("Old key should not be able to transfer");

    // But the new one should
    cli.set_private_key(0, new_private_key);
    cli.transfer_coins(0, 1, 5, None)
        .await
        .expect("New key should be able to transfer");
}
