// Copyright © Aptos Foundation
// Parts of the project are originally copyright © Meta Platforms, Inc.
// SPDX-License-Identifier: Apache-2.0

use super::new_test_context_with_orderless_flags;
use aptos_api_test_context::{current_function_name, TestContext};
use aptos_crypto::{
    bls12381::{PrivateKey, PublicKey},
    test_utils::KeyPair,
    SigningKey, Uniform,
};
use aptos_types::{
    function_info::FunctionInfo,
    transaction::{EntryFunction, TransactionStatus},
};
use move_core_types::{identifier::Identifier, language_storage::ModuleId, vm_status::StatusCode};
use rand::rngs::OsRng;
use rstest::rstest;
use serde_json::json;
use std::{path::PathBuf, sync::Arc};

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[rstest(
    use_txn_payload_v2_format,
    use_orderless_transactions,
    case(false, false),
    case(true, false),
    case(true, true)
)]
async fn test_account_abstraction_single_signer(
    use_txn_payload_v2_format: bool,
    use_orderless_transactions: bool,
) {
    let key_pair = Arc::new(KeyPair::<PrivateKey, PublicKey>::generate(&mut OsRng));

    let mut context = new_test_context_with_orderless_flags(
        current_function_name!(),
        use_txn_payload_v2_format,
        use_orderless_transactions,
    );
    let mut account = context.create_account().await;
    let user_addr = account.address();
    let other = context.create_account().await;

    // Publish packages
    let named_addresses = vec![("aa".to_string(), user_addr)];
    let txn = futures::executor::block_on(async move {
        let path = PathBuf::from(std::env!("CARGO_MANIFEST_DIR"))
            .join("../aptos-move/move-examples/account_abstraction/bls12381_single_key");
        TestContext::build_package(path, named_addresses)
    });
    context.publish_package(&mut account, txn).await;

    let txn0 = context.mint_user_account(&account).await;
    context.commit_block(&vec![txn0]).await;

    context
        .api_execute_entry_function(
            &mut account,
            &format!("0x{}::single_key::update_public_key", user_addr.to_hex()),
            json!([]),
            json!([hex::encode(key_pair.public_key.to_bytes())]),
        )
        .await;

    let func_info = FunctionInfo::new(
        user_addr,
        "single_key".to_string(),
        "authenticate".to_string(),
    );
    let txn3 = context
        .add_dispatchable_authentication_function(&account, func_info.clone())
        .await;
    context.commit_block(&vec![txn3]).await;

    let factory = context.transaction_factory();

    let fake_sign = Arc::new(|_: &[u8]| b"invalid_signature".to_vec());
    // case 1: invalid aa signature
    account.set_abstraction_auth(func_info.clone(), fake_sign);
    let aa_txn = account.sign_aa_transaction_with_transaction_builder(
        vec![],
        None,
        factory
            .account_transfer(other.address(), 1)
            .expiration_timestamp_secs(context.get_expiration_time())
            .upgrade_payload_with_rng(
                &mut context.rng,
                context.use_txn_payload_v2_format,
                context.use_orderless_transactions,
            ),
    );

    let txn_status = context.try_commit_block(&vec![aa_txn]).await;
    assert!(matches!(
        txn_status.last(),
        Some(TransactionStatus::Discard(StatusCode::ABORTED))
    ));
    // decrement seq num for aborted txn.
    account.decrement_sequence_number();

    // case 2: successful AA txn.
    let sign_func = Arc::new(move |x: &[u8]| {
        key_pair
            .private_key
            .sign_arbitrary_message(x)
            .to_bytes()
            .to_vec()
    });
    account.set_abstraction_auth(func_info.clone(), sign_func);
    let balance_start = context.get_apt_balance(other.address()).await;
    let aa_txn = account.sign_aa_transaction_with_transaction_builder(
        vec![],
        None,
        factory
            .account_transfer(other.address(), 4)
            .expiration_timestamp_secs(context.get_expiration_time())
            .upgrade_payload_with_rng(
                &mut context.rng,
                context.use_txn_payload_v2_format,
                context.use_orderless_transactions,
            ),
    );
    context
        .expect_status_code(202)
        .post_bcs_txn("/transactions", bcs::to_bytes(&aa_txn).unwrap())
        .await;
    context.commit_mempool_txns(1).await;
    assert_eq!(
        balance_start + 4,
        context.get_apt_balance(other.address()).await
    );
}

/// This tests a function with params (signer_a, signer_b, signer_c, d) works for the AA authentication flow.
/// a, c are AA; b, d are normal ed25519 accounts.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[rstest(
    use_txn_payload_v2_format,
    use_orderless_transactions,
    case(false, false),
    case(true, false),
    case(true, true)
)]
async fn test_account_abstraction_multi_agent_with_abstracted_sender(
    use_txn_payload_v2_format: bool,
    use_orderless_transactions: bool,
) {
    let key_pair = Arc::new(KeyPair::<PrivateKey, PublicKey>::generate(&mut OsRng));
    let mut context = new_test_context_with_orderless_flags(
        current_function_name!(),
        use_txn_payload_v2_format,
        use_orderless_transactions,
    );

    let mut a = context.create_account().await;
    let b = context.create_account().await;
    let mut c = context.create_account().await;
    let d = context.create_account().await;
    let a_addr = a.address();

    // Publish packages
    let named_addresses = vec![("aa".to_string(), a_addr)];
    let txn = futures::executor::block_on(async move {
        let path = PathBuf::from(std::env!("CARGO_MANIFEST_DIR"))
            .join("../aptos-move/move-examples/account_abstraction/bls12381_single_key");
        TestContext::build_package(path, named_addresses)
    });
    context.publish_package(&mut a, txn).await;

    let txn1 = context.mint_user_account(&a).await;
    context.commit_block(&vec![txn1]).await;
    let txn2 = context.mint_user_account(&b).await;
    context.commit_block(&vec![txn2]).await;
    let txn3 = context.mint_user_account(&c).await;
    context.commit_block(&vec![txn3]).await;

    // Convert c to aa
    context
        .api_execute_entry_function(
            &mut c,
            &format!("0x{}::single_key::update_public_key", a_addr.to_hex()),
            json!([]),
            json!([hex::encode(key_pair.public_key.to_bytes())]),
        )
        .await;
    let func_info = FunctionInfo::new(a_addr, "single_key".to_string(), "authenticate".to_string());
    let txn = context
        .add_dispatchable_authentication_function(&c, func_info.clone())
        .await;
    context.commit_block(&vec![txn]).await;

    let sign_func = Arc::new(move |x: &[u8]| {
        key_pair
            .private_key
            .sign_arbitrary_message(x)
            .to_bytes()
            .to_vec()
    });
    c.set_abstraction_auth(func_info, sign_func);

    let factory = context.transaction_factory();
    let balance_start = context.get_apt_balance(d.address()).await;
    let aa_txn = a.sign_aa_transaction_with_transaction_builder(
        vec![&b, &c],
        None,
        factory
            .entry_function(EntryFunction::new(
                ModuleId::new(a_addr, Identifier::new("test_functions").unwrap()),
                Identifier::new("transfer_to_the_last").unwrap(),
                vec![],
                vec![bcs::to_bytes(&d.address()).unwrap()],
            ))
            .expiration_timestamp_secs(context.get_expiration_time())
            .upgrade_payload_with_rng(
                &mut context.rng,
                context.use_txn_payload_v2_format,
                context.use_orderless_transactions,
            ),
    );
    context
        .expect_status_code(202)
        .post_bcs_txn("/transactions", bcs::to_bytes(&aa_txn).unwrap())
        .await;
    context.commit_mempool_txns(1).await;
    assert_eq!(
        balance_start + 3,
        context.get_apt_balance(d.address()).await
    );
}
