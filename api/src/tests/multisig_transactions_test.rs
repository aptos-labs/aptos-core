// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use super::new_test_context;
use aptos_api_test_context::{current_function_name, TestContext};
use aptos_types::{account_address::AccountAddress, transaction::EntryFunction};
use move_core_types::{
    ident_str,
    language_storage::{ModuleId, CORE_CODE_ADDRESS},
    value::{serialize_values, MoveValue},
};

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_multisig_transaction_with_payload_succeeds() {
    let mut context = new_test_context(current_function_name!());
    let owner_account_1 = &mut context.create_and_fund_account().await;
    let owner_account_2 = &mut context.create_and_fund_account().await;
    let owner_account_3 = &mut context.create_and_fund_account().await;
    let multisig_account = context
        .create_multisig_account(
            owner_account_1,
            vec![owner_account_2.address(), owner_account_3.address()],
            2,    /* 2-of-3 */
            1000, /* initial balance */
        )
        .await;
    assert_eq!(1000, context.get_apt_balance(multisig_account).await);

    let multisig_payload = construct_multisig_txn_transfer_payload(owner_account_1.address(), 1000);
    context
        .create_multisig_transaction(owner_account_1, multisig_account, multisig_payload)
        .await;
    // Owner 2 approves and owner 3 rejects. There are still 2 approvals total (owners 1 and 2) so
    // the transaction can still be executed.
    context
        .approve_multisig_transaction(owner_account_2, multisig_account, 1)
        .await;
    context
        .reject_multisig_transaction(owner_account_3, multisig_account, 1)
        .await;
    context
        .execute_multisig_transaction(owner_account_1, multisig_account, 202)
        .await;

    // The multisig tx that transfers away 1000 APT should have succeeded.
    assert_eq!(0, context.get_apt_balance(multisig_account).await);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_multisig_transaction_to_update_owners() {
    let mut context = new_test_context(current_function_name!());
    let owner_account_1 = &mut context.create_and_fund_account().await;
    let owner_account_2 = &mut context.create_and_fund_account().await;
    let owner_account_3 = &mut context.create_and_fund_account().await;
    let owner_account_4 = &mut context.create_and_fund_account().await;
    let multisig_account = context
        .create_multisig_account(
            owner_account_1,
            vec![owner_account_2.address()],
            2,
            1000, /* initial balance */
        )
        .await;

    // Add owners 3 and 4.
    let add_owners_payload = bcs::to_bytes(&EntryFunction {
        module: ModuleId::new(CORE_CODE_ADDRESS, ident_str!("multisig_account").to_owned()),
        function: ident_str!("add_owners").to_owned(),
        ty_args: vec![],
        args: serialize_values(&vec![MoveValue::vector_address(vec![
            owner_account_3.address(),
            owner_account_4.address(),
        ])]),
    })
    .unwrap();
    context
        .create_multisig_transaction(owner_account_1, multisig_account, add_owners_payload)
        .await;
    context
        .approve_multisig_transaction(owner_account_2, multisig_account, 1)
        .await;
    context
        .execute_multisig_transaction(owner_account_1, multisig_account, 202)
        .await;

    // There should be 4 owners now.
    let owners = get_owners(&context, multisig_account).await;
    let mut expected_owners = vec![
        owner_account_1.address(),
        owner_account_2.address(),
        owner_account_3.address(),
        owner_account_4.address(),
    ];
    expected_owners.sort();
    assert_eq!(expected_owners, owners);

    let remove_owners_payload = bcs::to_bytes(&EntryFunction {
        module: ModuleId::new(CORE_CODE_ADDRESS, ident_str!("multisig_account").to_owned()),
        function: ident_str!("remove_owners").to_owned(),
        ty_args: vec![],
        args: serialize_values(&vec![MoveValue::vector_address(vec![
            owner_account_4.address()
        ])]),
    })
    .unwrap();
    context
        .create_multisig_transaction(owner_account_1, multisig_account, remove_owners_payload)
        .await;
    context
        .approve_multisig_transaction(owner_account_3, multisig_account, 2)
        .await;
    context
        .execute_multisig_transaction(owner_account_1, multisig_account, 202)
        .await;
    // There should be 3 owners now that owner 4 has been kicked out.
    let owners = get_owners(&context, multisig_account).await;
    let mut expected_owners = vec![
        owner_account_1.address(),
        owner_account_2.address(),
        owner_account_3.address(),
    ];
    expected_owners.sort();
    assert_eq!(expected_owners, owners);
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
        .await;

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

async fn get_owners(
    context: &TestContext,
    multisig_account: AccountAddress,
) -> Vec<AccountAddress> {
    let multisig_account_resource = context
        .api_get_account_resource(
            multisig_account,
            "0x1",
            "multisig_account",
            "MultisigAccount",
        )
        .await;
    let mut owners = multisig_account_resource["data"]["owners"]
        .as_array()
        .unwrap()
        .iter()
        .cloned()
        .map(|address| AccountAddress::from_hex_literal(address.as_str().unwrap()).unwrap())
        .collect::<Vec<_>>();
    owners.sort();
    owners
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
