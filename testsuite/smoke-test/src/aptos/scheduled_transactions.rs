// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{aptos::move_test_helpers, smoke_test_environment::new_local_swarm_with_aptos};
use aptos_forge::{AptosPublicInfo, LocalSwarm, Swarm};
use aptos_sdk::{transaction_builder::TransactionFactory, types::LocalAccount};
use aptos_types::transaction::{EntryFunction, TransactionPayload};
use move_core_types::{account_address::AccountAddress, ident_str, language_storage::ModuleId};
use std::time::Duration;

async fn test_setup() -> (
    LocalSwarm,
    AptosPublicInfo,
    TransactionFactory,
    LocalAccount,
) {
    let swarm = new_local_swarm_with_aptos(1).await;
    let mut chain_info = swarm.aptos_public_info();

    let base_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let path = base_dir.join("src/aptos/package_scheduled_txns_usage/");

    let transaction_factory = match move_test_helpers::publish_package(&mut chain_info, path).await
    {
        Ok(tf) => tf,
        Err(e) => panic!("Failed to publish package: {:?}", e),
    };

    let account = chain_info
        .create_and_fund_user_account(50_000_000_000)
        .await
        .unwrap();

    (swarm, chain_info, transaction_factory, account)
}

// TODO: Need to find a way to verify the scheduled transactions being run. For now they are
//       verified manually by checking the logs through the code and in the scheduled user fn.

#[tokio::test]
async fn test_schedule_txns_e2e() {
    let (_swarm, chain_info, transaction_factory, account) = test_setup().await;

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
        )))
        .max_gas_amount(100000000)
        .gas_unit_price(100)
        .expiration_timestamp_secs(10000000000);

    let insert_sched_txns = account.sign_with_transaction_builder(txn_builder);
    let response = chain_info
        .client()
        .submit_and_wait(&insert_sched_txns)
        .await
        .unwrap();
    assert!(
        response.inner().success(),
        "Transaction failed: {:?}",
        response.inner()
    );

    // Get the version where the scheduled transaction was inserted
    let schedule_insert_version = response.inner().version().unwrap();

    // Poll for scheduled transactions in subsequent blocks
    let mut scheduled_count = 0;
    let start_time = std::time::Instant::now();
    let timeout = Duration::from_secs(30);

    while start_time.elapsed() < timeout && scheduled_count < 3 {
        tokio::time::sleep(Duration::from_millis(500)).await;

        // Get latest transactions and look for scheduled transaction type
        let latest_version = chain_info
            .client()
            .get_ledger_information()
            .await
            .unwrap()
            .into_inner()
            .version;

        // Search recent transactions for ScheduledTransaction type
        let start_search = schedule_insert_version + 1;
        let limit = std::cmp::min(100, latest_version - start_search + 1);

        if limit > 0 {
            let txns = chain_info
                .client()
                .get_transactions(Some(start_search), Some(limit as u16))
                .await
                .unwrap()
                .into_inner();

            for txn in txns {
                if let aptos_api_types::Transaction::ScheduledTransaction(_) = txn {
                    scheduled_count += 1;
                }
            }
        }
    }

    assert!(
        scheduled_count >= 3,
        "Expected 3 scheduled transactions but found only {}",
        scheduled_count
    );
}

#[tokio::test]
async fn test_cancel_and_shutdown_e2e() {
    let (_swarm, mut chain_info, transaction_factory, account) = test_setup().await;

    // Get current timestamp and schedule transactions far in the future (1 hour from now)
    let current_time_ms = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_millis() as u64;
    let far_future_time = current_time_ms + 3600000; // 1 hour in the future

    // Insert 3 scheduled transactions at far future time
    let insert_txn = transaction_factory
        .payload(TransactionPayload::EntryFunction(EntryFunction::new(
            ModuleId::new(
                AccountAddress::from_hex_literal("0xA550C18").unwrap(),
                ident_str!("scheduled_txns_usage").to_owned(),
            ),
            ident_str!("test_insert_transactions").to_owned(),
            vec![],
            vec![bcs::to_bytes(&far_future_time).unwrap()],
        )))
        .max_gas_amount(100000000)
        .gas_unit_price(100)
        .expiration_timestamp_secs(10000000000);

    let insert_response = chain_info
        .client()
        .submit_and_wait(&account.sign_with_transaction_builder(insert_txn))
        .await
        .unwrap();
    assert!(
        insert_response.inner().success(),
        "Insert transaction failed: {:?}",
        insert_response.inner()
    );

    // Cancel 1 transaction
    let cancel_txn = transaction_factory
        .payload(TransactionPayload::EntryFunction(EntryFunction::new(
            ModuleId::new(
                AccountAddress::from_hex_literal("0xA550C18").unwrap(),
                ident_str!("scheduled_txns_usage").to_owned(),
            ),
            ident_str!("test_cancel_transaction").to_owned(),
            vec![],
            vec![],
        )))
        .max_gas_amount(100000000)
        .gas_unit_price(100)
        .expiration_timestamp_secs(10000000000);

    let cancel_response = chain_info
        .client()
        .submit_and_wait(&account.sign_with_transaction_builder(cancel_txn))
        .await
        .unwrap();
    assert!(
        cancel_response.inner().success(),
        "Cancel transaction failed: {:?}",
        cancel_response.inner()
    );

    // Get the version where the cancel transaction was executed
    let cancel_version = cancel_response.inner().version().unwrap();

    // Wait for TransactionCancelledEvent to be emitted
    let start_time = std::time::Instant::now();
    let timeout = Duration::from_secs(10);
    let mut cancelled_event_found = false;

    while start_time.elapsed() < timeout && !cancelled_event_found {
        tokio::time::sleep(Duration::from_millis(200)).await;

        // Get latest transactions and look for events
        let latest_version = chain_info
            .client()
            .get_ledger_information()
            .await
            .unwrap()
            .into_inner()
            .version;

        if latest_version >= cancel_version {
            // Get transactions from cancel version onwards to look for events
            let search_limit = std::cmp::min(20, latest_version - cancel_version + 1);
            let txns = chain_info
                .client()
                .get_transactions(Some(cancel_version), Some(search_limit as u16))
                .await
                .unwrap()
                .into_inner();

            for txn in txns {
                if let aptos_api_types::Transaction::UserTransaction(user_txn) = &txn {
                    // Look for TransactionCancelledEvent in the transaction events
                    for event in &user_txn.events {
                        if event.typ.to_string().contains("TransactionCancelledEvent") {
                            cancelled_event_found = true;
                            break;
                        }
                    }
                    if cancelled_event_found {
                        break;
                    }
                }
            }
        }
    }

    assert!(
        cancelled_event_found,
        "TransactionCancelledEvent was not emitted within timeout"
    );

    // Start shutdown using the user module with governance approach
    let shutdown_txn = transaction_factory
        .payload(TransactionPayload::EntryFunction(EntryFunction::new(
            ModuleId::new(
                AccountAddress::from_hex_literal("0xA550C18").unwrap(),
                ident_str!("scheduled_txns_usage").to_owned(),
            ),
            ident_str!("test_shutdown").to_owned(),
            vec![],
            vec![],
        )))
        .max_gas_amount(100000000)
        .gas_unit_price(100)
        .expiration_timestamp_secs(10000000000);

    let root_account = chain_info.root_account();
    let shutdown_response = chain_info
        .client()
        .submit_and_wait(&root_account.sign_with_transaction_builder(shutdown_txn))
        .await
        .unwrap();
    assert!(
        shutdown_response.inner().success(),
        "Shutdown transaction failed: {:?}",
        shutdown_response.inner()
    );

    // Continue shutdown with a separate transaction calling continue_shutdown directly
    let continue_shutdown_txn = transaction_factory
        .payload(TransactionPayload::EntryFunction(EntryFunction::new(
            ModuleId::new(AccountAddress::ONE, ident_str!("scheduled_txns").to_owned()),
            ident_str!("continue_shutdown").to_owned(),
            vec![],
            vec![bcs::to_bytes(&100u64).unwrap()], // cancel_batch_size = 100
        )))
        .max_gas_amount(100000000)
        .gas_unit_price(100)
        .expiration_timestamp_secs(10000000000);

    let continue_shutdown_response = chain_info
        .client()
        .submit_and_wait(&root_account.sign_with_transaction_builder(continue_shutdown_txn))
        .await
        .unwrap();
    assert!(
        continue_shutdown_response.inner().success(),
        "Continue shutdown transaction failed: {:?}",
        continue_shutdown_response.inner()
    );

    // Get the version where continue_shutdown was executed to monitor for events
    let continue_shutdown_version = continue_shutdown_response.inner().version().unwrap();

    // Look for TransactionFailedEvent events emitted during shutdown
    // The events should be in the continue_shutdown transaction
    let mut failed_events_count = 0;

    // Check the continue_shutdown transaction for events
    let continue_shutdown_txn = chain_info
        .client()
        .get_transaction_by_version(continue_shutdown_version)
        .await
        .unwrap()
        .into_inner();

    if let aptos_api_types::Transaction::UserTransaction(user_txn) = &continue_shutdown_txn {
        for event in &user_txn.events {
            if event.typ.to_string().contains("TransactionFailedEvent") {
                failed_events_count += 1;
            }
        }
    }

    assert_eq!(
        failed_events_count, 2,
        "Expected at least 2 TransactionFailedEvent events but found only {}",
        failed_events_count
    );
}
