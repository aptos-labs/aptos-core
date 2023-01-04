// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use super::new_test_context;
use aptos_api_test_context::current_function_name;
use aptos_types::{account_address::AccountAddress, transaction::EntryFunction};
use move_core_types::{
    ident_str,
    language_storage::{ModuleId, CORE_CODE_ADDRESS},
    value::{serialize_values, MoveValue},
};

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_multisig_transaction_with_payload() {
    let mut context = new_test_context(current_function_name!());
    let owner_account = &mut context.create_and_fund_account().await;
    let multisig_account = context
        .create_multisig_account(owner_account, vec![], 1, 1000)
        .await;
    assert_eq!(1000, context.get_apt_balance(multisig_account).await);
    let multisig_payload = construct_multisig_txn_transfer_payload(owner_account.address(), 1000);
    context
        .create_multisig_transaction(owner_account, multisig_account, multisig_payload)
        .await;
    context
        .execute_multisig_transaction(owner_account, multisig_account, 202)
        .await;

    // The multisig tx that transfers away 1000 APT should have succeeded.
    assert_eq!(0, context.get_apt_balance(multisig_account).await);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_multisig_transaction_with_payload_and_failing_execution() {
    let mut context = new_test_context(current_function_name!());
    let owner_account = &mut context.create_and_fund_account().await;
    let multisig_account = context
        .create_multisig_account(owner_account, vec![], 1, 1000)
        .await;
    assert_eq!(1000, context.get_apt_balance(multisig_account).await);
    let multisig_payload = construct_multisig_txn_transfer_payload(owner_account.address(), 2000);
    context
        .create_multisig_transaction(owner_account, multisig_account, multisig_payload)
        .await;
    // Target transaction execution should fail because the multisig account only has 1000 APT but
    // is requested to send 2000.
    // The transaction should still succeed with the failure tracked on chain.
    context
        .execute_multisig_transaction(owner_account, multisig_account, 202)
        .await;
    let transaction_execution_failed_events = context
        .get(format!("/accounts/{}/events/0x1::multisig_account::MultisigAccount/transaction_execution_failed_events", multisig_account).as_str())
        .await;
    let transaction_execution_failed_events =
        transaction_execution_failed_events.as_array().unwrap();
    assert_eq!(1, transaction_execution_failed_events.len());
    assert_eq!(
        "65542",
        transaction_execution_failed_events[0]["data"]["execution_error"]["error_code"]
    );
    // Balance didn't change since the target transaction failed.
    assert_eq!(1000, context.get_apt_balance(multisig_account).await);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_multisig_transaction_with_payload_hash() {
    let mut context = new_test_context(current_function_name!());
    let owner_account = &mut context.create_and_fund_account().await;
    let multisig_account = context
        .create_multisig_account(owner_account, vec![], 1, 1000)
        .await;
    assert_eq!(1000, context.get_apt_balance(multisig_account).await);
    let multisig_payload = construct_multisig_txn_transfer_payload(owner_account.address(), 1000);
    context
        .create_multisig_transaction_with_payload_hash(
            owner_account,
            multisig_account,
            multisig_payload,
        )
        .await;
    context
        .execute_multisig_transaction_with_payload(
            owner_account,
            multisig_account,
            "0x1::aptos_account::transfer",
            &[],
            &[&owner_account.address().to_hex_literal(), "1000"],
            202,
        )
        .await;

    // The multisig tx that transfers away 1000 APT should have succeeded.
    assert_eq!(0, context.get_apt_balance(multisig_account).await);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_multisig_transaction_with_payload_hash_and_failing_execution() {
    let mut context = new_test_context(current_function_name!());
    let owner_account = &mut context.create_and_fund_account().await;
    let multisig_account = context
        .create_multisig_account(owner_account, vec![], 1, 1000)
        .await;
    assert_eq!(1000, context.get_apt_balance(multisig_account).await);
    let multisig_payload = construct_multisig_txn_transfer_payload(owner_account.address(), 2000);
    context
        .create_multisig_transaction_with_payload_hash(
            owner_account,
            multisig_account,
            multisig_payload,
        )
        .await; // Target transaction execution should fail because the multisig account only has 1000 APT but
                // Target transaction execution should fail because the multisig account only has 1000 APT but
                // is requested to send 2000.
                // The transaction should still succeed with the failure tracked on chain.
    context
        .execute_multisig_transaction_with_payload(
            owner_account,
            multisig_account,
            "0x1::aptos_account::transfer",
            &[],
            &[&owner_account.address().to_hex_literal(), "2000"],
            202,
        )
        .await;
    let transaction_execution_failed_events = context
        .get(format!("/accounts/{}/events/0x1::multisig_account::MultisigAccount/transaction_execution_failed_events", multisig_account).as_str())
        .await;
    let transaction_execution_failed_events =
        transaction_execution_failed_events.as_array().unwrap();
    assert_eq!(1, transaction_execution_failed_events.len());
    assert_eq!(
        "65542",
        transaction_execution_failed_events[0]["data"]["execution_error"]["error_code"]
    );
    // Balance didn't change since the target transaction failed.
    assert_eq!(1000, context.get_apt_balance(multisig_account).await);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_multisig_transaction_with_payload_not_matching_hash() {
    let mut context = new_test_context(current_function_name!());
    let owner_account = &mut context.create_and_fund_account().await;
    let multisig_account = context
        .create_multisig_account(owner_account, vec![], 1, 1000)
        .await;
    let multisig_payload = construct_multisig_txn_transfer_payload(owner_account.address(), 500);
    context
        .create_multisig_transaction_with_payload_hash(
            owner_account,
            multisig_account,
            multisig_payload,
        )
        .await;

    // The multisig transaction execution should fail due to the amount being different
    // (1000 vs 500).
    context
        .execute_multisig_transaction_with_payload(
            owner_account,
            multisig_account,
            "0x1::aptos_account::transfer",
            &[],
            &[&owner_account.address().to_hex_literal(), "1000"],
            400,
        )
        .await;
}

fn construct_multisig_txn_transfer_payload(recipient: AccountAddress, amount: u64) -> Vec<u8> {
    bcs::to_bytes(&EntryFunction {
        module: ModuleId::new(CORE_CODE_ADDRESS, ident_str!("aptos_account").to_owned()),
        function: ident_str!("transfer").to_owned(),
        ty_args: vec![],
        args: serialize_values(&vec![MoveValue::Address(recipient), MoveValue::U64(amount)]),
    })
    .unwrap()
}
