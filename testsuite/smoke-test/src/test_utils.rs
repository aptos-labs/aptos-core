// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use aptos_rest_client::Client as RestClient;
use aptos_sdk::{
    transaction_builder::TransactionFactory,
    types::{transaction::SignedTransaction, LocalAccount},
};
use aptos_transaction_builder::aptos_stdlib;
use forge::{LocalSwarm, NodeExt, Swarm};
use rand::random;

pub async fn create_and_fund_account(swarm: &'_ mut dyn Swarm, amount: u64) -> LocalAccount {
    let account = LocalAccount::generate(&mut rand::rngs::OsRng);
    let mut chain_info = swarm.chain_info().into_aptos_public_info();
    chain_info
        .create_user_account(account.public_key())
        .await
        .unwrap();
    chain_info.mint(account.address(), amount).await.unwrap();
    account
}

pub async fn transfer_coins_non_blocking(
    client: &RestClient,
    transaction_factory: &TransactionFactory,
    sender: &mut LocalAccount,
    receiver: &LocalAccount,
    amount: u64,
) -> SignedTransaction {
    let txn = sender.sign_with_transaction_builder(transaction_factory.payload(
        aptos_stdlib::encode_test_coin_transfer(receiver.address(), amount),
    ));

    client.submit(&txn).await.unwrap();
    txn
}

pub async fn transfer_coins(
    client: &RestClient,
    transaction_factory: &TransactionFactory,
    sender: &mut LocalAccount,
    receiver: &LocalAccount,
    amount: u64,
) -> SignedTransaction {
    let txn =
        transfer_coins_non_blocking(client, transaction_factory, sender, receiver, amount).await;

    client.wait_for_signed_transaction(&txn).await.unwrap();

    txn
}

pub async fn transfer_and_reconfig(
    client: &RestClient,
    transaction_factory: &TransactionFactory,
    root_account: &mut LocalAccount,
    sender: &mut LocalAccount,
    receiver: &LocalAccount,
    num_transfers: usize,
) {
    for _ in 0..num_transfers {
        // Reconfigurations have a 20% chance of being executed
        if random::<u16>() % 5 == 0 {
            let aptos_version = client.get_aptos_version().await.unwrap();
            let current_version = *aptos_version.into_inner().major.inner();
            let txn = root_account.sign_with_transaction_builder(transaction_factory.payload(
                aptos_stdlib::encode_version_set_version(current_version + 1),
            ));
            client.submit_and_wait(&txn).await.unwrap();

            println!("Changing aptos version to {}", current_version + 1,);
        }

        transfer_coins(client, transaction_factory, sender, receiver, 1).await;
    }
}

pub async fn assert_balance(client: &RestClient, account: &LocalAccount, balance: u64) {
    let on_chain_balance = client
        .get_account_balance(account.address())
        .await
        .unwrap()
        .into_inner();

    assert_eq!(on_chain_balance.get(), balance);
}

/// This module provides useful functions for operating, handling and managing
/// AptosSwarm instances. It is particularly useful for working with tests that
/// require a SmokeTestEnvironment, as it provides a generic interface across
/// AptosSwarms, regardless of if the swarm is a validator swarm, validator full
/// node swarm, or a public full node swarm.
pub mod swarm_utils {
    use aptos_config::config::{NodeConfig, SecureBackend, WaypointConfig};
    use aptos_secure_storage::{KVStorage, Storage};
    use aptos_types::waypoint::Waypoint;

    pub fn insert_waypoint(node_config: &mut NodeConfig, waypoint: Waypoint) {
        let f = |backend: &SecureBackend| {
            let mut storage: Storage = backend.into();
            storage
                .set(aptos_global_constants::WAYPOINT, waypoint)
                .expect("Unable to write waypoint");
            storage
                .set(aptos_global_constants::GENESIS_WAYPOINT, waypoint)
                .expect("Unable to write waypoint");
        };
        let backend = &node_config.consensus.safety_rules.backend;
        f(backend);
        match &node_config.base.waypoint {
            WaypointConfig::FromStorage(backend) => {
                f(backend);
            }
            _ => panic!("unexpected waypoint from node config"),
        }
    }
}

/// This helper function creates 3 new accounts, mints funds, transfers funds
/// between the accounts and verifies that these operations succeed.
pub async fn check_create_mint_transfer(swarm: &mut LocalSwarm) {
    let client = swarm.validators().next().unwrap().rest_client();

    // Create account 0, mint 10 coins and check balance
    let mut account_0 = create_and_fund_account(swarm, 10).await;
    assert_balance(&client, &account_0, 10).await;

    // Create account 1, mint 1 coin, transfer 3 coins from account 0 to 1, check balances
    let account_1 = create_and_fund_account(swarm, 1).await;
    transfer_coins(
        &client,
        &swarm.chain_info().transaction_factory(),
        &mut account_0,
        &account_1,
        3,
    )
    .await;

    assert_balance(&client, &account_0, 7).await;
    assert_balance(&client, &account_1, 4).await;

    // Create account 2, mint 15 coins and check balance
    let account_2 = create_and_fund_account(swarm, 15).await;
    assert_balance(&client, &account_2, 15).await;
}
