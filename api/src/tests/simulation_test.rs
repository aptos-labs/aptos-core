// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use super::new_test_context;
use aptos_api_test_context::{current_function_name, pretty, TestContext};
use aptos_crypto::ed25519::Ed25519Signature;
use aptos_types::{
    account_address::AccountAddress,
    transaction::{
        authenticator::{AccountAuthenticator, TransactionAuthenticator},
        EntryFunction, RawTransaction, SignedTransaction, TransactionPayload,
    },
};
use move_core_types::{ident_str, language_storage::ModuleId};
use serde_json::json;
use std::path::PathBuf;
const ACCOUNT_ABSTRACTION: u64 = 85;

async fn simulate_aptos_transfer(
    context: &mut TestContext,
    use_valid_signature: bool,
    transfer_amount: u64,
    expected_status: u16,
    assert_gas_used: bool,
) -> serde_json::Value {
    let alice = &mut context.gen_account();
    let bob = &mut context.gen_account();
    let txn = context.mint_user_account(alice).await;
    context.commit_block(&vec![txn]).await;

    let txn = context.account_transfer_to(alice, bob.address(), transfer_amount);

    if let TransactionAuthenticator::Ed25519 {
        public_key,
        signature,
    } = txn.authenticator_ref()
    {
        let signature = use_valid_signature
            .then(|| signature.to_string())
            .unwrap_or(Ed25519Signature::dummy_signature().to_string());
        let req = warp::test::request()
            .method("POST")
            .path("/v1/transactions/simulate")
            .json(&json!({
                "sender": txn.sender().to_string(),
                "sequence_number": txn.sequence_number().to_string(),
                "max_gas_amount": txn.max_gas_amount().to_string(),
                "gas_unit_price": txn.gas_unit_price().to_string(),
                "expiration_timestamp_secs": txn.expiration_timestamp_secs().to_string(),
                "payload": {
                    "type": "entry_function_payload",
                    "function": "0x1::aptos_account::transfer",
                    "type_arguments": [],
                    "arguments": [
                        bob.address().to_standard_string(), transfer_amount.to_string(),
                    ]
                },
                "signature": {
                    "type": "ed25519_signature",
                    "public_key": public_key.to_string(),
                    "signature": signature,
                }
            }));
        let resp = context.expect_status_code(expected_status).reply(req).await;
        // Assert the gas used header is present if expected.
        if assert_gas_used {
            assert!(
                resp.headers()
                    .get("X-Aptos-Gas-Used")
                    .unwrap()
                    .to_str()
                    .unwrap()
                    .parse::<u64>()
                    .unwrap()
                    > 0
            );
        }
        serde_json::from_slice(resp.body()).unwrap()
    } else {
        unreachable!("Simulation uses Ed25519 authenticator.");
    }
}

const SMALL_TRANSFER_AMOUNT: u64 = 10;
const LARGE_TRANSFER_AMOUNT: u64 = 1_000_000_000;

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_simulate_transaction_with_valid_signature() {
    let mut context = new_test_context(current_function_name!());
    let resp = simulate_aptos_transfer(&mut context, true, SMALL_TRANSFER_AMOUNT, 400, false).await;
    context.check_golden_output(resp);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_simulate_transaction_with_not_valid_signature() {
    let mut context = new_test_context(current_function_name!());
    let resp = simulate_aptos_transfer(&mut context, false, SMALL_TRANSFER_AMOUNT, 200, true).await;
    assert!(resp[0]["success"].as_bool().is_some_and(|v| v));
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_simulate_transaction_with_insufficient_balance() {
    let mut context = new_test_context(current_function_name!());
    let resp = simulate_aptos_transfer(&mut context, false, LARGE_TRANSFER_AMOUNT, 200, true).await;
    assert!(!resp[0]["success"].as_bool().is_some_and(|v| v));
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_bcs_simulate_fee_payer_transaction_without_gas_fee_check_with_aa_disabled() {
    let mut context = new_test_context(current_function_name!());
    context.disable_feature(ACCOUNT_ABSTRACTION).await;
    bcs_simulate_fee_payer_transaction_without_gas_fee_check(&mut context).await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_bcs_simulate_fee_payer_transaction_without_gas_fee_check() {
    let mut context = new_test_context(current_function_name!());
    context.disable_feature(ACCOUNT_ABSTRACTION).await;
    bcs_simulate_fee_payer_transaction_without_gas_fee_check(&mut context).await;
}

// Enable the MODULE_EVENT_MIGRATION feature
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_simulate_txn_with_aggregator() {
    let mut context = new_test_context(current_function_name!());
    let account = context.root_account().await;

    let named_addresses = vec![("addr".to_string(), account.address())];
    let path = PathBuf::from(std::env!("CARGO_MANIFEST_DIR")).join("src/tests/move/pack_counter");
    let payload = TestContext::build_package(path, named_addresses);
    let txn = account.sign_with_transaction_builder(context.transaction_factory().payload(payload));
    context.commit_block(&vec![txn]).await;

    let payload = TransactionPayload::EntryFunction(EntryFunction::new(
        ModuleId::new(account.address(), ident_str!("counter").to_owned()),
        ident_str!("increment_counter").to_owned(),
        vec![],
        vec![],
    ));
    let txn = account.sign_with_transaction_builder(context.transaction_factory().payload(payload));
    if let TransactionAuthenticator::Ed25519 {
        public_key,
        signature: _,
    } = txn.authenticator_ref()
    {
        let function = format!("{}::counter::increment_counter", account.address());
        let resp = context
            .expect_status_code(200)
            .post(
                "/transactions/simulate",
                json!({
                    "sender": txn.sender().to_string(),
                    "sequence_number": txn.sequence_number().to_string(),
                    "max_gas_amount": txn.max_gas_amount().to_string(),
                    "gas_unit_price": txn.gas_unit_price().to_string(),
                    "expiration_timestamp_secs": txn.expiration_timestamp_secs().to_string(),
                    "payload": {
                        "type": "entry_function_payload",
                        "function": function,
                        "type_arguments": [],
                        "arguments": []
                    },
                    "signature": {
                        "type": "ed25519_signature",
                        "public_key": public_key.to_string(),
                        "signature": Ed25519Signature::dummy_signature().to_string(),
                    }
                }),
            )
            .await;
        assert!(resp[0]["success"].as_bool().is_some_and(|v| v));
    } else {
        unreachable!("Simulation uses Ed25519 authenticator.");
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_bcs_simulate_simple() {
    let transfer_amount: u64 = SMALL_TRANSFER_AMOUNT;

    let mut context = new_test_context(current_function_name!());
    let alice = &mut context.gen_account();
    let bob = &mut context.gen_account();
    let txn = context.mint_user_account(alice).await;
    context.commit_block(&vec![txn]).await;

    let txn = context.account_transfer_to(alice, bob.address(), transfer_amount);
    let body = bcs::to_bytes(&txn).unwrap();

    // expected to fail due to using a valid signature.
    let _resp = context
        .expect_status_code(400)
        .post_bcs_txn("/transactions/simulate", body)
        .await;

    if let TransactionAuthenticator::Ed25519 {
        public_key,
        signature: _,
    } = txn.authenticator_ref()
    {
        let txn = SignedTransaction::new_signed_transaction(
            txn.clone().into_raw_transaction(),
            TransactionAuthenticator::Ed25519 {
                public_key: public_key.clone(),
                signature: Ed25519Signature::dummy_signature(),
            },
        );

        let body = bcs::to_bytes(&txn).unwrap();

        // expected to succeed
        let resp = context
            .expect_status_code(200)
            .post_bcs_txn("/transactions/simulate", body)
            .await;

        assert!(resp[0]["success"].as_bool().unwrap(), "{}", pretty(&resp));
    } else {
        unreachable!("Simulation uses Ed25519 authenticator.");
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_bcs_simulate_without_auth_key_check() {
    let transfer_amount: u64 = SMALL_TRANSFER_AMOUNT;

    let mut context = new_test_context(current_function_name!());
    let alice = &mut context.gen_account();
    let bob = &mut context.gen_account();
    let txn = context.mint_user_account(alice).await;
    context.commit_block(&vec![txn]).await;

    // Construct a signed transaction.
    let txn = context.account_transfer_to(alice, bob.address(), transfer_amount);
    // Replace the authenticator with a NoAccountAuthenticator in the transaction.
    let txn = SignedTransaction::new_signed_transaction(
        txn.clone().into_raw_transaction(),
        TransactionAuthenticator::SingleSender {
            sender: AccountAuthenticator::NoAccountAuthenticator,
        },
    );

    let body = bcs::to_bytes(&txn).unwrap();

    // expected to succeed
    let resp = context
        .expect_status_code(200)
        .post_bcs_txn("/transactions/simulate", body)
        .await;
    assert!(resp[0]["success"].as_bool().unwrap(), "{}", pretty(&resp));
}

async fn bcs_simulate_fee_payer_transaction_without_gas_fee_check(context: &mut TestContext) {
    let alice = &mut context.gen_account();
    let bob = &mut context.gen_account();
    let txn = context.mint_user_account(alice).await;
    context.commit_block(&vec![txn]).await;

    let transfer_amount: u64 = SMALL_TRANSFER_AMOUNT;
    let txn = context.account_transfer_to(alice, bob.address(), transfer_amount);
    let raw_txn = RawTransaction::new(
        txn.sender(),
        txn.sequence_number(),
        txn.payload().clone(),
        txn.max_gas_amount(),
        100,
        txn.expiration_timestamp_secs(),
        txn.chain_id(),
    );
    let txn = SignedTransaction::new_signed_transaction(
        raw_txn.clone(),
        TransactionAuthenticator::FeePayer {
            sender: AccountAuthenticator::NoAccountAuthenticator,
            secondary_signer_addresses: vec![],
            secondary_signers: vec![],
            fee_payer_address: AccountAddress::ONE,
            fee_payer_signer: AccountAuthenticator::NoAccountAuthenticator,
        },
    );
    let body = bcs::to_bytes(&txn).unwrap();
    let resp = context
        .expect_status_code(200)
        .post_bcs_txn("/transactions/simulate", body)
        .await;
    assert!(!resp[0]["success"].as_bool().unwrap(), "{}", pretty(&resp));
    assert!(
        resp[0]["vm_status"]
            .as_str()
            .unwrap()
            .contains("INSUFFICIENT_BALANCE_FOR_TRANSACTION_FEE"),
        "{}",
        pretty(&resp)
    );

    let txn = SignedTransaction::new_signed_transaction(
        raw_txn.clone(),
        TransactionAuthenticator::FeePayer {
            sender: AccountAuthenticator::NoAccountAuthenticator,
            secondary_signer_addresses: vec![],
            secondary_signers: vec![],
            fee_payer_address: AccountAddress::ZERO,
            fee_payer_signer: AccountAuthenticator::NoAccountAuthenticator,
        },
    );
    let body = bcs::to_bytes(&txn).unwrap();
    let resp = context
        .expect_status_code(200)
        .post_bcs_txn("/transactions/simulate", body)
        .await;
    assert!(resp[0]["success"].as_bool().unwrap(), "{}", pretty(&resp));
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_bcs_simulate_automated_account_creation() {
    let mut context = new_test_context(current_function_name!());
    let alice = &mut context.gen_account();
    let bob = &mut context.gen_account();

    let transfer_amount: u64 = 0;
    let txn = context.account_transfer_to(alice, bob.address(), transfer_amount);
    let raw_txn = RawTransaction::new(
        txn.sender(),
        txn.sequence_number(),
        txn.payload().clone(),
        txn.max_gas_amount(),
        100,
        txn.expiration_timestamp_secs(),
        txn.chain_id(),
    );
    // Replace the authenticator with a NoAccountAuthenticator in the transaction.
    let txn = SignedTransaction::new_signed_transaction(
        raw_txn.clone(),
        TransactionAuthenticator::SingleSender {
            sender: AccountAuthenticator::NoAccountAuthenticator,
        },
    );

    let body = bcs::to_bytes(&txn).unwrap();

    let resp = context
        .expect_status_code(200)
        .post_bcs_txn("/transactions/simulate", body)
        .await;
    assert!(!resp[0]["success"].as_bool().unwrap(), "{}", pretty(&resp));
    assert!(
        resp[0]["vm_status"]
            .as_str()
            .unwrap()
            .contains("SENDING_ACCOUNT_DOES_NOT_EXIST"),
        "{}",
        pretty(&resp)
    );

    let txn =
        SignedTransaction::new_signed_transaction(raw_txn, TransactionAuthenticator::FeePayer {
            sender: AccountAuthenticator::NoAccountAuthenticator,
            secondary_signer_addresses: vec![],
            secondary_signers: vec![],
            fee_payer_address: AccountAddress::ZERO,
            fee_payer_signer: AccountAuthenticator::NoAccountAuthenticator,
        });
    let body = bcs::to_bytes(&txn).unwrap();
    let resp = context
        .expect_status_code(200)
        .post_bcs_txn("/transactions/simulate", body)
        .await;
    assert!(resp[0]["success"].as_bool().unwrap(), "{}", pretty(&resp));
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_bcs_execute_simple_no_authenticator_fail() {
    let transfer_amount: u64 = SMALL_TRANSFER_AMOUNT;

    let mut context = new_test_context(current_function_name!());
    let alice = &mut context.gen_account();
    let bob = &mut context.gen_account();
    let txn = context.mint_user_account(alice).await;
    context.commit_block(&vec![txn]).await;

    // Construct a signed transaction.
    let txn = context.account_transfer_to(alice, bob.address(), transfer_amount);
    // Replace the authenticator with a NoAccountAuthenticator in the transaction.
    let txn = SignedTransaction::new_signed_transaction(
        txn.clone().into_raw_transaction(),
        TransactionAuthenticator::SingleSender {
            sender: AccountAuthenticator::NoAccountAuthenticator,
        },
    );

    let body = bcs::to_bytes(&txn).unwrap();

    // expected to fail due to the use of NoAccountAuthenticator in an actual execution
    let resp = context
        .expect_status_code(400)
        .post_bcs_txn("/transactions", body)
        .await;
    assert!(resp["message"]
        .as_str()
        .unwrap()
        .contains("INVALID_SIGNATURE"));
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_bcs_execute_fee_payer_transaction_no_authenticator_fail() {
    let mut context = new_test_context(current_function_name!());
    let alice = &mut context.gen_account();
    let bob = &mut context.gen_account();
    let txn = context.mint_user_account(alice).await;
    context.commit_block(&vec![txn]).await;

    let transfer_amount: u64 = SMALL_TRANSFER_AMOUNT;
    let txn = context.account_transfer_to(alice, bob.address(), transfer_amount);
    let raw_txn = RawTransaction::new(
        txn.sender(),
        txn.sequence_number(),
        txn.payload().clone(),
        txn.max_gas_amount(),
        100,
        txn.expiration_timestamp_secs(),
        txn.chain_id(),
    );

    let txn = SignedTransaction::new_signed_transaction(
        raw_txn.clone(),
        TransactionAuthenticator::FeePayer {
            sender: AccountAuthenticator::NoAccountAuthenticator,
            secondary_signer_addresses: vec![],
            secondary_signers: vec![],
            fee_payer_address: AccountAddress::ZERO,
            fee_payer_signer: AccountAuthenticator::NoAccountAuthenticator,
        },
    );
    let body = bcs::to_bytes(&txn).unwrap();
    let resp = context
        .expect_status_code(400)
        .post_bcs_txn("/transactions", body)
        .await;
    assert!(resp["message"]
        .as_str()
        .unwrap()
        .contains("INVALID_SIGNATURE"));
}
