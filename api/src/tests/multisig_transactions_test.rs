// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

// Note[Orderless]: Done
use super::new_test_context_with_orderless_flags;
use aptos_api_test_context::{current_function_name, TestContext};
use aptos_types::{
    account_address::AccountAddress,
    transaction::{EntryFunction, MultisigTransactionPayload},
};
use move_core_types::{
    ident_str,
    language_storage::{ModuleId, CORE_CODE_ADDRESS},
    value::{serialize_values, MoveValue},
};
use proptest::num::usize;
use rstest::rstest;

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[rstest(
    use_txn_payload_v2_format,
    use_orderless_transactions,
    case(false, false),
    case(true, false),
    case(true, true)
)]
async fn test_multisig_transaction_with_payload_succeeds(
    use_txn_payload_v2_format: bool,
    use_orderless_transactions: bool,
) {
    let mut context = new_test_context_with_orderless_flags(
        current_function_name!(),
        use_txn_payload_v2_format,
        use_orderless_transactions,
    )
    .await;
    let owner_account_1 = &mut context.create_account().await;
    let owner_account_2 = &mut context.create_account().await;
    let owner_account_3 = &mut context.create_account().await;
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
        .create_multisig_transaction(owner_account_1, multisig_account, multisig_payload.clone())
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
#[rstest(
    use_txn_payload_v2_format,
    use_orderless_transactions,
    case(false, false),
    case(true, false),
    case(true, true)
)]
async fn test_multisig_transaction_with_existing_account(
    use_txn_payload_v2_format: bool,
    use_orderless_transactions: bool,
) {
    let mut context = new_test_context_with_orderless_flags(
        current_function_name!(),
        use_txn_payload_v2_format,
        use_orderless_transactions,
    )
    .await;
    let multisig_account = &mut context.create_account().await;
    let owner_account_1 = &mut context.create_account().await;
    let owner_account_2 = &mut context.create_account().await;
    let owner_account_3 = &mut context.create_account().await;
    let owners = vec![
        owner_account_1.address(),
        owner_account_2.address(),
        owner_account_3.address(),
    ];
    context
        .create_multisig_account_with_existing_account(multisig_account, owners.clone(), 2, 1000)
        .await;
    assert_owners(&context, multisig_account.address(), owners).await;
    assert_signature_threshold(&context, multisig_account.address(), 2).await;

    let multisig_payload = construct_multisig_txn_transfer_payload(owner_account_1.address(), 1000);
    context
        .create_multisig_transaction(
            owner_account_1,
            multisig_account.address(),
            multisig_payload.clone(),
        )
        .await;
    // Owner 2 approves and owner 3 rejects. There are still 2 approvals total (owners 1 and 2) so
    // the transaction can still be executed.
    context
        .approve_multisig_transaction(owner_account_2, multisig_account.address(), 1)
        .await;
    context
        .reject_multisig_transaction(owner_account_3, multisig_account.address(), 1)
        .await;

    let org_multisig_balance = context.get_apt_balance(multisig_account.address()).await;
    let org_owner_1_balance = context.get_apt_balance(owner_account_1.address()).await;

    context
        .execute_multisig_transaction(owner_account_2, multisig_account.address(), 202)
        .await;

    // The multisig tx that transfers away 1000 APT should have succeeded.
    assert_eq!(
        org_multisig_balance - 1000,
        context.get_apt_balance(multisig_account.address()).await
    );
    assert_eq!(
        org_owner_1_balance + 1000,
        context.get_apt_balance(owner_account_1.address()).await
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[rstest(
    use_txn_payload_v2_format,
    use_orderless_transactions,
    case(false, false),
    case(true, false),
    case(true, true)
)]
async fn test_multisig_transaction_to_update_owners(
    use_txn_payload_v2_format: bool,
    use_orderless_transactions: bool,
) {
    let mut context = new_test_context_with_orderless_flags(
        current_function_name!(),
        use_txn_payload_v2_format,
        use_orderless_transactions,
    )
    .await;
    let owner_account_1 = &mut context.create_account().await;
    let owner_account_2 = &mut context.create_account().await;
    let owner_account_3 = &mut context.create_account().await;
    let owner_account_4 = &mut context.create_account().await;
    let multisig_account = context
        .create_multisig_account(
            owner_account_1,
            vec![owner_account_2.address()],
            2,
            0, /* initial balance */
        )
        .await;
    assert_eq!(0, context.get_apt_balance(multisig_account).await);

    // Add owners 3 and 4.
    let add_owners_payload = bcs::to_bytes(&MultisigTransactionPayload::EntryFunction(
        EntryFunction::new(
            ModuleId::new(CORE_CODE_ADDRESS, ident_str!("multisig_account").to_owned()),
            ident_str!("add_owners").to_owned(),
            vec![],
            serialize_values(&vec![MoveValue::vector_address(vec![
                owner_account_3.address(),
                owner_account_4.address(),
            ])]),
        ),
    ))
    .unwrap();
    context
        .create_multisig_transaction(
            owner_account_1,
            multisig_account,
            add_owners_payload.clone(),
        )
        .await;
    context
        .approve_multisig_transaction(owner_account_2, multisig_account, 1)
        .await;
    context
        .execute_multisig_transaction(owner_account_1, multisig_account, 202)
        .await;

    // There should be 4 owners now.
    assert_owners(&context, multisig_account, vec![
        owner_account_1.address(),
        owner_account_2.address(),
        owner_account_3.address(),
        owner_account_4.address(),
    ])
    .await;

    let remove_owners_payload = bcs::to_bytes(&MultisigTransactionPayload::EntryFunction(
        EntryFunction::new(
            ModuleId::new(CORE_CODE_ADDRESS, ident_str!("multisig_account").to_owned()),
            ident_str!("remove_owners").to_owned(),
            vec![],
            serialize_values(&vec![MoveValue::vector_address(vec![
                owner_account_4.address()
            ])]),
        ),
    ))
    .unwrap();
    context
        .create_multisig_transaction(
            owner_account_1,
            multisig_account,
            remove_owners_payload.clone(),
        )
        .await;
    context
        .approve_multisig_transaction(owner_account_3, multisig_account, 2)
        .await;
    context
        .execute_multisig_transaction(owner_account_1, multisig_account, 202)
        .await;
    // There should be 3 owners now that owner 4 has been kicked out.
    assert_owners(&context, multisig_account, vec![
        owner_account_1.address(),
        owner_account_2.address(),
        owner_account_3.address(),
    ])
    .await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[rstest(
    use_txn_payload_v2_format,
    use_orderless_transactions,
    case(false, false),
    case(true, false),
    case(true, true)
)]
async fn test_multisig_transaction_update_signature_threshold(
    use_txn_payload_v2_format: bool,
    use_orderless_transactions: bool,
) {
    let mut context = new_test_context_with_orderless_flags(
        current_function_name!(),
        use_txn_payload_v2_format,
        use_orderless_transactions,
    )
    .await;
    let owner_account_1 = &mut context.create_account().await;
    let owner_account_2 = &mut context.create_account().await;
    let multisig_account = context
        .create_multisig_account(
            owner_account_1,
            vec![owner_account_2.address()],
            2,    /* 2-of-2 */
            1000, /* initial balance */
        )
        .await;

    // Change the signature threshold from 2-of-2 to 1-of-2
    let signature_threshold_payload = bcs::to_bytes(&MultisigTransactionPayload::EntryFunction(
        EntryFunction::new(
            ModuleId::new(CORE_CODE_ADDRESS, ident_str!("multisig_account").to_owned()),
            ident_str!("update_signatures_required").to_owned(),
            vec![],
            serialize_values(&vec![MoveValue::U64(1)]),
        ),
    ))
    .unwrap();
    context
        .create_multisig_transaction(
            owner_account_1,
            multisig_account,
            signature_threshold_payload.clone(),
        )
        .await;
    context
        .approve_multisig_transaction(owner_account_2, multisig_account, 1)
        .await;
    context
        .execute_multisig_transaction(owner_account_1, multisig_account, 202)
        .await;

    // The signature threshold should be 1-of-2 now.
    assert_signature_threshold(&context, multisig_account, 1).await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[rstest(
    use_txn_payload_v2_format,
    use_orderless_transactions,
    case(false, false),
    case(true, false),
    case(true, true)
)]
async fn test_multisig_transaction_with_insufficient_balance_to_cover_gas(
    use_txn_payload_v2_format: bool,
    use_orderless_transactions: bool,
) {
    let mut context = new_test_context_with_orderless_flags(
        current_function_name!(),
        use_txn_payload_v2_format,
        use_orderless_transactions,
    )
    .await;
    let owner_account_1 = &mut context.create_account().await;
    // Owner 2 has no APT balance.
    let owner_account_2 = &mut context.gen_account();
    let multisig_account = context
        .create_multisig_account(
            owner_account_1,
            vec![owner_account_2.address()],
            1,
            1000, /* initial balance */
        )
        .await;

    let multisig_payload = construct_multisig_txn_transfer_payload(owner_account_1.address(), 1000);
    context
        .create_multisig_transaction(owner_account_1, multisig_account, multisig_payload)
        .await;
    // Target transaction execution should fail because the owner 2 account has no balance for gas.
    context
        .execute_multisig_transaction(owner_account_2, multisig_account, 400)
        .await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[rstest(
    use_txn_payload_v2_format,
    use_orderless_transactions,
    case(false, false),
    case(true, false),
    case(true, true)
)]
async fn test_multisig_transaction_with_payload_and_failing_execution(
    use_txn_payload_v2_format: bool,
    use_orderless_transactions: bool,
) {
    let mut context = new_test_context_with_orderless_flags(
        current_function_name!(),
        use_txn_payload_v2_format,
        use_orderless_transactions,
    )
    .await;
    let owner_account = &mut context.create_account().await;
    let multisig_account = context
        .create_multisig_account(owner_account, vec![], 1, 1000)
        .await;
    assert_eq!(1000, context.get_apt_balance(multisig_account).await);
    let multisig_payload = construct_multisig_txn_transfer_payload(owner_account.address(), 2000);
    context
        .create_multisig_transaction(owner_account, multisig_account, multisig_payload.clone())
        .await;
    // Target transaction execution should fail because the multisig account only has 1000 APT but
    // is requested to send 2000.
    // The transaction should still succeed with the failure tracked on chain.
    context
        .execute_multisig_transaction(owner_account, multisig_account, 202)
        .await;

    // Balance didn't change since the target transaction failed.
    assert_eq!(1000, context.get_apt_balance(multisig_account).await);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[rstest(
    use_txn_payload_v2_format,
    use_orderless_transactions,
    case(false, false),
    case(true, false),
    case(true, true)
)]
async fn test_multisig_transaction_with_payload_hash(
    use_txn_payload_v2_format: bool,
    use_orderless_transactions: bool,
) {
    let mut context = new_test_context_with_orderless_flags(
        current_function_name!(),
        use_txn_payload_v2_format,
        use_orderless_transactions,
    )
    .await;
    let owner_account = &mut context.create_account().await;
    let multisig_account = context
        .create_multisig_account(owner_account, vec![], 1, 1000)
        .await;
    assert_eq!(1000, context.get_apt_balance(multisig_account).await);
    let multisig_payload = construct_multisig_txn_transfer_payload(owner_account.address(), 1000);
    context
        .create_multisig_transaction_with_payload_hash(
            owner_account,
            multisig_account,
            multisig_payload.clone(),
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
#[rstest(
    use_txn_payload_v2_format,
    use_orderless_transactions,
    case(false, false),
    case(true, false),
    case(true, true)
)]
async fn test_multisig_transaction_with_payload_hash_and_failing_execution(
    use_txn_payload_v2_format: bool,
    use_orderless_transactions: bool,
) {
    let mut context = new_test_context_with_orderless_flags(
        current_function_name!(),
        use_txn_payload_v2_format,
        use_orderless_transactions,
    )
    .await;
    let owner_account = &mut context.create_account().await;
    let multisig_account = context
        .create_multisig_account(owner_account, vec![], 1, 1000)
        .await;
    assert_eq!(1000, context.get_apt_balance(multisig_account).await);
    let multisig_payload = construct_multisig_txn_transfer_payload(owner_account.address(), 2000);
    context
        .create_multisig_transaction_with_payload_hash(
            owner_account,
            multisig_account,
            multisig_payload.clone(),
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
    // Balance didn't change since the target transaction failed.
    assert_eq!(1000, context.get_apt_balance(multisig_account).await);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[rstest(
    use_txn_payload_v2_format,
    use_orderless_transactions,
    case(false, false),
    case(true, false),
    case(true, true)
)]
async fn test_multisig_transaction_with_payload_not_matching_hash(
    use_txn_payload_v2_format: bool,
    use_orderless_transactions: bool,
) {
    let mut context = new_test_context_with_orderless_flags(
        current_function_name!(),
        use_txn_payload_v2_format,
        use_orderless_transactions,
    )
    .await;
    let owner_account = &mut context.create_account().await;
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

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[rstest(
    use_txn_payload_v2_format,
    use_orderless_transactions,
    case(false, false),
    case(true, false),
    case(true, true)
)]

async fn test_multisig_transaction_with_matching_payload(
    use_txn_payload_v2_format: bool,
    use_orderless_transactions: bool,
) {
    let mut context = new_test_context_with_orderless_flags(
        current_function_name!(),
        use_txn_payload_v2_format,
        use_orderless_transactions,
    )
    .await;
    let owner_account = &mut context.create_account().await;
    let multisig_account = context
        .create_multisig_account(owner_account, vec![], 1, 1000)
        .await;
    assert_eq!(1000, context.get_apt_balance(multisig_account).await);
    let multisig_payload = construct_multisig_txn_transfer_payload(owner_account.address(), 1000);
    context
        .create_multisig_transaction(owner_account, multisig_account, multisig_payload.clone())
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
#[rstest(
    use_txn_payload_v2_format,
    use_orderless_transactions,
    case(false, false),
    case(true, false),
    case(true, true)
)]
async fn test_multisig_transaction_with_mismatching_payload(
    use_txn_payload_v2_format: bool,
    use_orderless_transactions: bool,
) {
    let mut context = new_test_context_with_orderless_flags(
        current_function_name!(),
        use_txn_payload_v2_format,
        use_orderless_transactions,
    )
    .await;
    let owner_account = &mut context.create_account().await;
    let multisig_account = context
        .create_multisig_account(owner_account, vec![], 1, 1000)
        .await;
    let multisig_payload = construct_multisig_txn_transfer_payload(owner_account.address(), 1000);
    context
        .create_multisig_transaction(owner_account, multisig_account, multisig_payload.clone())
        .await;

    // The multisig transaction execution should fail due to the payload mismatch
    // amount being different (1000 vs 2000).
    context
        .execute_multisig_transaction_with_payload(
            owner_account,
            multisig_account,
            "0x1::aptos_account::transfer",
            &[],
            &[&owner_account.address().to_hex_literal(), "2000"],
            400,
        )
        .await;
    // Balance didn't change since the target transaction failed.
    assert_eq!(1000, context.get_apt_balance(multisig_account).await);

    // Excuting the transaction with the correct payload should succeed.
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
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[rstest(
    use_txn_payload_v2_format,
    use_orderless_transactions,
    case(false, false),
    case(true, false),
    case(true, true)
)]
async fn test_multisig_transaction_simulation(
    use_txn_payload_v2_format: bool,
    use_orderless_transactions: bool,
) {
    let mut context = new_test_context_with_orderless_flags(
        current_function_name!(),
        use_txn_payload_v2_format,
        use_orderless_transactions,
    )
    .await;
    let owner_account_1 = &mut context.create_account().await;
    let owner_account_2 = &mut context.create_account().await;
    let owner_account_3 = &mut context.create_account().await;
    let multisig_account = context
        .create_multisig_account(
            owner_account_1,
            vec![owner_account_2.address(), owner_account_3.address()],
            1,    /* 1-of-3 */
            1000, /* initial balance */
        )
        .await;

    let multisig_payload = construct_multisig_txn_transfer_payload(owner_account_1.address(), 1000);
    context
        .create_multisig_transaction(owner_account_1, multisig_account, multisig_payload.clone())
        .await;

    // Simulate the multisig tx
    let simulation_resp = context
        .simulate_multisig_transaction(
            owner_account_1,
            multisig_account,
            "0x1::aptos_account::transfer",
            &[],
            &[&owner_account_1.address().to_hex_literal(), "1000"],
            200,
        )
        .await;
    // Validate that the simulation did successfully execute a transfer of 1000 coins from the
    // multisig account.
    let simulation_resp = &simulation_resp.as_array().unwrap()[0];
    assert!(simulation_resp["success"].as_bool().unwrap());
    let withdraw_event = &simulation_resp["events"].as_array().unwrap()[0];
    assert_eq!(
        withdraw_event["type"].as_str().unwrap(),
        "0x1::coin::WithdrawEvent"
    );
    let withdraw_from_account = AccountAddress::from_hex_literal(
        withdraw_event["guid"]["account_address"].as_str().unwrap(),
    )
    .unwrap();
    let withdrawn_amount = withdraw_event["data"]["amount"].as_str().unwrap();
    assert_eq!(withdraw_from_account, multisig_account);
    assert_eq!(withdrawn_amount, "1000");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[rstest(
    use_txn_payload_v2_format,
    use_orderless_transactions,
    case(false, false),
    case(true, false),
    case(true, true)
)]
async fn test_multisig_transaction_simulation_2_of_3(
    use_txn_payload_v2_format: bool,
    use_orderless_transactions: bool,
) {
    let mut context = new_test_context_with_orderless_flags(
        current_function_name!(),
        use_txn_payload_v2_format,
        use_orderless_transactions,
    )
    .await;
    let owner_account_1 = &mut context.create_account().await;
    let owner_account_2 = &mut context.create_account().await;
    let owner_account_3 = &mut context.create_account().await;
    let multisig_account = context
        .create_multisig_account(
            owner_account_1,
            vec![owner_account_2.address(), owner_account_3.address()],
            2,    /* 2-of-3 */
            1000, /* initial balance */
        )
        .await;

    let multisig_payload = construct_multisig_txn_transfer_payload(owner_account_1.address(), 1000);
    context
        .create_multisig_transaction(owner_account_1, multisig_account, multisig_payload.clone())
        .await;

    context
        .approve_multisig_transaction(owner_account_2, multisig_account, 1)
        .await;

    // Simulate the multisig transaction
    let simulation_resp = context
        .simulate_multisig_transaction(
            owner_account_1,
            multisig_account,
            "0x1::aptos_account::transfer",
            &[],
            &[&owner_account_1.address().to_hex_literal(), "1000"],
            200,
        )
        .await;
    // Validate that the simulation did successfully execute a transfer of 1000 coins from the
    // multisig account.
    let simulation_resp = &simulation_resp.as_array().unwrap()[0];
    assert!(simulation_resp["success"].as_bool().unwrap());
    let withdraw_event = &simulation_resp["events"].as_array().unwrap()[0];
    assert_eq!(
        withdraw_event["type"].as_str().unwrap(),
        "0x1::coin::WithdrawEvent"
    );
    let withdraw_from_account = AccountAddress::from_hex_literal(
        withdraw_event["guid"]["account_address"].as_str().unwrap(),
    )
    .unwrap();
    let withdrawn_amount = withdraw_event["data"]["amount"].as_str().unwrap();
    assert_eq!(withdraw_from_account, multisig_account);
    assert_eq!(withdrawn_amount, "1000");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[rstest(
    use_txn_payload_v2_format,
    use_orderless_transactions,
    case(false, false),
    case(true, false),
    case(true, true)
)]
async fn test_multisig_transaction_simulation_fail(
    use_txn_payload_v2_format: bool,
    use_orderless_transactions: bool,
) {
    let mut context = new_test_context_with_orderless_flags(
        current_function_name!(),
        use_txn_payload_v2_format,
        use_orderless_transactions,
    )
    .await;
    let owner_account_1 = &mut context.create_account().await;
    let owner_account_2 = &mut context.create_account().await;
    let owner_account_3 = &mut context.create_account().await;
    let multisig_account = context
        .create_multisig_account(
            owner_account_1,
            vec![owner_account_2.address(), owner_account_3.address()],
            1,    /* 1-of-3 */
            1000, /* initial balance */
        )
        .await;

    let multisig_payload = construct_multisig_txn_transfer_payload(owner_account_1.address(), 2000);
    context
        .create_multisig_transaction(owner_account_1, multisig_account, multisig_payload.clone())
        .await;

    // Simulating transferring more than what the multisig account has should fail.
    let simulation_resp = context
        .simulate_multisig_transaction(
            owner_account_1,
            multisig_account,
            "0x1::aptos_account::transfer",
            &[],
            &[&owner_account_1.address().to_hex_literal(), "2000"],
            200,
        )
        .await;
    let simulation_resp = &simulation_resp.as_array().unwrap()[0];
    let transaction_failed = &simulation_resp["events"]
        .as_array()
        .unwrap()
        .iter()
        .any(|event| {
            event["type"]
                .as_str()
                .unwrap()
                .contains("TransactionExecutionFailed")
        });
    assert!(transaction_failed);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[rstest(
    use_txn_payload_v2_format,
    use_orderless_transactions,
    case(false, false),
    case(true, false),
    case(true, true)
)]
async fn test_multisig_transaction_simulation_fail_2_of_3_insufficient_approvals(
    use_txn_payload_v2_format: bool,
    use_orderless_transactions: bool,
) {
    let mut context = new_test_context_with_orderless_flags(
        current_function_name!(),
        use_txn_payload_v2_format,
        use_orderless_transactions,
    )
    .await;
    let owner_account_1 = &mut context.create_account().await;
    let owner_account_2 = &mut context.create_account().await;
    let owner_account_3 = &mut context.create_account().await;
    let multisig_account = context
        .create_multisig_account(
            owner_account_1,
            vec![owner_account_2.address(), owner_account_3.address()],
            2,    /* 2-of-3 */
            1000, /* initial balance */
        )
        .await;

    let multisig_payload = construct_multisig_txn_transfer_payload(owner_account_1.address(), 2000);
    context
        .create_multisig_transaction(owner_account_1, multisig_account, multisig_payload.clone())
        .await;

    // Simulating without sufficient approvals has should fail.
    let simulation_resp = context
        .simulate_multisig_transaction(
            owner_account_1,
            multisig_account,
            "0x1::aptos_account::transfer",
            &[],
            &[&owner_account_1.address().to_hex_literal(), "1000"],
            200,
        )
        .await;
    let simulation_resp = &simulation_resp.as_array().unwrap()[0];
    assert!(!simulation_resp["success"].as_bool().unwrap());
    assert!(simulation_resp["vm_status"]
        .as_str()
        .unwrap()
        .contains("MULTISIG_TRANSACTION_INSUFFICIENT_APPROVALS"));
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[rstest(
    use_txn_payload_v2_format,
    use_orderless_transactions,
    case(false, false),
    case(true, false),
    case(true, true)
)]
async fn test_simulate_multisig_transaction_should_charge_gas_against_sender(
    use_txn_payload_v2_format: bool,
    use_orderless_transactions: bool,
) {
    let mut context = new_test_context_with_orderless_flags(
        current_function_name!(),
        use_txn_payload_v2_format,
        use_orderless_transactions,
    )
    .await;
    let owner_account = &mut context.create_account().await;
    let multisig_account = context
        .create_multisig_account(
            owner_account,
            vec![],
            1,  /* 1-of-1 */
            10, /* initial balance */
        )
        .await;
    assert_eq!(10, context.get_apt_balance(multisig_account).await);

    let multisig_payload = construct_multisig_txn_transfer_payload(owner_account.address(), 10);
    context
        .create_multisig_transaction(owner_account, multisig_account, multisig_payload.clone())
        .await;

    // This simulation should succeed because gas should be paid out of the sender account (owner),
    // not the multisig account itself.
    let simulation_resp = context
        .simulate_multisig_transaction(
            owner_account,
            multisig_account,
            "0x1::aptos_account::transfer",
            &[],
            &[&owner_account.address().to_hex_literal(), "10"],
            200,
        )
        .await;
    let simulation_resp = &simulation_resp.as_array().unwrap()[0];
    assert!(simulation_resp["success"].as_bool().unwrap());
}

async fn assert_owners(
    context: &TestContext,
    multisig_account: AccountAddress,
    mut expected_owners: Vec<AccountAddress>,
) {
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
    expected_owners.sort();
    assert_eq!(expected_owners, owners);
}

async fn assert_signature_threshold(
    context: &TestContext,
    multisig_account: AccountAddress,
    expected_signature_threshold: u64,
) {
    let multisig_account_resource = context
        .api_get_account_resource(
            multisig_account,
            "0x1",
            "multisig_account",
            "MultisigAccount",
        )
        .await;
    assert_eq!(
        expected_signature_threshold.to_string(),
        multisig_account_resource["data"]["num_signatures_required"]
            .as_str()
            .unwrap()
    );
}

fn construct_multisig_txn_transfer_payload(recipient: AccountAddress, amount: u64) -> Vec<u8> {
    bcs::to_bytes(&MultisigTransactionPayload::EntryFunction(
        EntryFunction::new(
            ModuleId::new(CORE_CODE_ADDRESS, ident_str!("aptos_account").to_owned()),
            ident_str!("transfer").to_owned(),
            vec![],
            serialize_values(&vec![MoveValue::Address(recipient), MoveValue::U64(amount)]),
        ),
    ))
    .unwrap()
}
