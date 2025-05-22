// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use std::time::Duration;
use aptos_cached_packages::aptos_stdlib;
use aptos_forge::{AptosPublicInfo, LocalSwarm, Swarm};
use aptos_logger::info;
use aptos_sdk::transaction_builder::TransactionFactory;
use aptos_sdk::types::LocalAccount;
use aptos_types::transaction::{EntryFunction, TransactionPayload};
use aptos_vm::AptosVM;
use move_core_types::account_address::AccountAddress;
use move_core_types::ident_str;
use move_core_types::language_storage::ModuleId;
use crate::aptos::move_test_helpers;
use crate::smoke_test_environment::new_local_swarm_with_aptos;

async fn test_setup() -> (LocalSwarm, AptosPublicInfo, TransactionFactory, LocalAccount) {
    let mut swarm = new_local_swarm_with_aptos(1).await;
    let mut chain_info = swarm.aptos_public_info();

    let base_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let path = base_dir.join("src/aptos/package_scheduled_txns_usage/");

    let transaction_factory = match move_test_helpers::publish_package(&mut chain_info, path).await {
        Ok(tf) => { tf },
        Err(e) => panic!("Failed to publish package: {:?}", e),
    };

    let account = chain_info
        .create_and_fund_user_account(50_000_000_000)
        .await
        .unwrap();

    (swarm, chain_info, transaction_factory, account)
}

#[tokio::test]
async fn test_basic_schedule() {
    let (_swarm, mut chain_info, transaction_factory, account) = test_setup().await;

   // Get current timestamp for scheduling
    let current_time_ms = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_millis() as u64;

    let txn_builder = transaction_factory
        .payload(TransactionPayload::EntryFunction(EntryFunction::new(
            ModuleId::new(
                AccountAddress::from_hex_literal("0xA550C18").unwrap(),
                ident_str!("scheduled_txns_usage").to_owned(),
            ),
            ident_str!("test_insert_transactions").to_owned(),
            vec![],
            vec![bcs::to_bytes(&current_time_ms).unwrap()],
        ))).max_gas_amount(100000000)
        .gas_unit_price(100)
        .expiration_timestamp_secs(10000000000);

    let insert_sched_txns = account.sign_with_transaction_builder(txn_builder);
    info!("Gonna submit_and_wait: Account balance before {}", chain_info.get_balance(account.address()).await);
    let response = chain_info.client().submit_and_wait(&insert_sched_txns).await.unwrap();
    assert!(
        response.inner().success(),
        "Transaction failed: {:?}",
        response.inner()
    );

    // sleep for 5 seconds
    tokio::time::sleep(Duration::from_secs(5)).await;

    info!("Account balance after {}", chain_info.get_balance(account.address()).await);

    assert!(false);
}

#[tokio::test]
async fn test_schedule_with_transfer_txns() {
    let (_swarm, mut chain_info, transaction_factory, account) = test_setup().await;

    // Create two accounts for transfers
    let account1 = chain_info
        .create_and_fund_user_account(10_000_000_000)
        .await
        .unwrap();
    let account2 = chain_info
        .create_and_fund_user_account(1_000_000)
        .await
        .unwrap();

    // Get current timestamp for scheduling
    let current_time_ms = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_millis() as u64;

    let txn_builder = transaction_factory
        .payload(TransactionPayload::EntryFunction(EntryFunction::new(
            ModuleId::new(
                AccountAddress::from_hex_literal("0xA550C18").unwrap(),
                ident_str!("scheduled_txns_usage").to_owned(),
            ),
            ident_str!("test_insert_transactions").to_owned(),
            vec![],
            vec![bcs::to_bytes(&current_time_ms).unwrap()],
        ))).max_gas_amount(100000000)
        .gas_unit_price(100)
        .expiration_timestamp_secs(10000000000);

    let insert_sched_txns = account.sign_with_transaction_builder(txn_builder);
    info!("Gonna submit_and_wait: Account balance before {}", chain_info.get_balance(account.address()).await);
    let response = chain_info.client().submit(&insert_sched_txns).await.unwrap();
    /*assert!(
        response.inner().success(),
        "Transaction failed: {:?}",
        response.inner()
    );*/

    // sleep for 5 seconds
    //tokio::time::sleep(Duration::from_secs(5)).await;
    // Submit transfers from account1 to account2 for 5 seconds
    let start_time = std::time::Instant::now();
    let duration = Duration::from_secs(5);

    while start_time.elapsed() < duration {
        let transfer_txn = account1.sign_with_transaction_builder(
            transaction_factory
                .payload(aptos_stdlib::aptos_coin_transfer(account2.address(), 100)),
        );
        let _ = chain_info.client().submit(&transfer_txn).await.unwrap();
        //tokio::time::sleep(Duration::from_millis(2)).await;
    }

    tokio::time::sleep(Duration::from_secs(2)).await;
    info!("Account balance after {}", chain_info.get_balance(account.address()).await);

    assert!(false);
}
