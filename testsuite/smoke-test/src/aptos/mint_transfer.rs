// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use std::time::Duration;
use crate::smoke_test_environment::new_local_swarm_with_aptos;
use aptos_cached_packages::aptos_stdlib;
use aptos_forge::{Swarm, SwarmExt};
use aptos_logger::info;
use aptos_move_debugger::aptos_debugger::AptosDebugger;
use aptos_types::transaction::{EntryFunction, ExecutionStatus, TransactionPayload, TransactionStatus};
use move_core_types::account_address::AccountAddress;
use move_core_types::ident_str;
use move_core_types::language_storage::ModuleId;
use crate::aptos::move_test_helpers;

#[tokio::test(flavor = "multi_thread", worker_threads = 1)]
async fn test_mint_transfer() {
    let swarm = new_local_swarm_with_aptos(1).await;
    let mut info = swarm.aptos_public_info();

    let account1 = info.random_account();
    info.create_user_account(account1.public_key())
        .await
        .unwrap();
    let account2 = info.random_account();
    info.create_user_account(account2.public_key())
        .await
        .unwrap();

    // NOTE(Gas): For some reason, there needs to be a lot of funds in the account in order for the
    //            test to pass.
    //            Is this caused by us increasing the default max gas amount in
    //            testsuite/forge/src/interface/aptos.rs?
    info.mint(account1.address(), 100_000_000_000)
        .await
        .unwrap();

    let transfer_txn = account1.sign_with_transaction_builder(
        info.transaction_factory()
            .payload(aptos_stdlib::aptos_coin_transfer(account2.address(), 40000)),
    );
    info.client().submit_and_wait(&transfer_txn).await.unwrap();
    assert_eq!(
        info.client()
            .view_apt_account_balance(account2.address())
            .await
            .unwrap()
            .into_inner(),
        40000
    );

    // test delegation
    let txn_factory = info.transaction_factory();
    let delegate_txn1 = info
        .root_account()
        .sign_with_transaction_builder(txn_factory.payload(
            aptos_stdlib::aptos_coin_delegate_mint_capability(account1.address()),
        ));
    info.client().submit_and_wait(&delegate_txn1).await.unwrap();

    // Test delegating more than one at a time: faucet startup stampeding herd
    let delegate_txn2 = info
        .root_account()
        .sign_with_transaction_builder(txn_factory.payload(
            aptos_stdlib::aptos_coin_delegate_mint_capability(account2.address()),
        ));
    info.client().submit_and_wait(&delegate_txn2).await.unwrap();

    let claim_txn = account1.sign_with_transaction_builder(
        txn_factory.payload(aptos_stdlib::aptos_coin_claim_mint_capability()),
    );
    info.client().submit_and_wait(&claim_txn).await.unwrap();
    let mint_txn = account1.sign_with_transaction_builder(
        txn_factory.payload(aptos_stdlib::aptos_coin_mint(account1.address(), 10000)),
    );
    info.client().submit_and_wait(&mint_txn).await.unwrap();

    // Testing the AptosDebugger by reexecuting the transaction that has been published.
    println!("Testing....");
    let debugger = AptosDebugger::rest_client(info.client().clone()).unwrap();

    let txn_ver = debugger
        .get_version_by_account_sequence(account1.address(), 0)
        .await
        .unwrap()
        .unwrap();

    let output = debugger
        .execute_past_transactions(txn_ver, 1, false, 1, &[1])
        .await
        .unwrap()
        .pop()
        .unwrap();

    assert_eq!(
        output.status(),
        &TransactionStatus::Keep(ExecutionStatus::Success)
    );
}

#[tokio::test]
async fn test_sched() {
    let mut swarm = new_local_swarm_with_aptos(1).await;
    let mut chain_info = swarm.aptos_public_info();

    let base_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let path = base_dir.join("src/aptos/package_scheduled_txns_usage/");

    let transaction_factory = match move_test_helpers::publish_package(&mut chain_info, path).await {
        Ok(tf) => { tf },
        Err(e) => panic!("Failed to publish package: {:?}", e),
    };

    let mut account1 = chain_info
        .create_and_fund_user_account(50_000_000_000)
        .await
        .unwrap();

    // Get current timestamp for scheduling
    let current_time_ms = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_millis() as u64;

    //let transaction_factory = chain_info.transaction_factory();
    let txn_builder = transaction_factory
        .payload(TransactionPayload::EntryFunction(EntryFunction::new(
            ModuleId::new(
                AccountAddress::from_hex_literal("0xA550C18").unwrap(),
                ident_str!("scheduled_txns_usage").to_owned(),
            ),
            ident_str!("test_insert_transactions").to_owned(),
            vec![],
            vec![bcs::to_bytes(&current_time_ms).unwrap()],
        )))
        .max_gas_amount(200000)
        .gas_unit_price(100)
        .expiration_timestamp_secs(10000000000);

    let mutate_txn = account1.sign_with_transaction_builder(txn_builder);
    info!("Gonna submit_and_wait..........");

    let response = chain_info.client().submit_and_wait(&mutate_txn).await.unwrap();
    assert!(
        response.inner().success(),
        "Transaction failed: {:?}",
        response.inner()
    );

    info!("Going again for submit_and_wait..........");

    // sleep for 5 seconds
    tokio::time::sleep(Duration::from_secs(5)).await;

    assert!(false);
}
