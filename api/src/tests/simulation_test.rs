// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use super::new_test_context;
use aptos_api_test_context::{current_function_name, TestContext};
use aptos_crypto::ed25519::Ed25519Signature;
use aptos_types::transaction::authenticator::TransactionAuthenticator;
use serde_json::json;

async fn simulate_aptos_transfer(
    context: &mut TestContext,
    use_valid_signature: bool,
    transfer_amount: u64,
    expected_status: u16,
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
        context
            .expect_status_code(expected_status)
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
                }),
            )
            .await
    } else {
        unreachable!("Simulation uses Ed25519 authenticator.");
    }
}

const SMALL_TRANSFER_AMOUNT: u64 = 10;
const LARGE_TRANSFER_AMOUNT: u64 = 1_000_000_000;

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_simulate_transaction_with_valid_signature() {
    let mut context = new_test_context(current_function_name!());
    let resp = simulate_aptos_transfer(&mut context, true, SMALL_TRANSFER_AMOUNT, 400).await;
    context.check_golden_output(resp);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_simulate_transaction_with_not_valid_signature() {
    let mut context = new_test_context(current_function_name!());
    let resp = simulate_aptos_transfer(&mut context, false, SMALL_TRANSFER_AMOUNT, 200).await;
    assert!(resp[0]["success"].as_bool().is_some_and(|v| v));
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_simulate_transaction_with_insufficient_balance() {
    let mut context = new_test_context(current_function_name!());
    let resp = simulate_aptos_transfer(&mut context, false, LARGE_TRANSFER_AMOUNT, 200).await;
    assert!(!resp[0]["success"].as_bool().is_some_and(|v| v));
}
