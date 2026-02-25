// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Negative tests for public structs and enums as transaction arguments.
//!
//! These tests verify the API correctly rejects invalid struct/enum arguments.

use super::setup_public_struct_test;
use aptos_api_test_context::current_function_name;
use rstest::rstest;
use serde_json::json;

/// Test that malformed struct/enum arguments are rejected at encode time.
///
/// Each case specifies an entry function, a bad argument, and the substring
/// expected to appear in the 400 error message.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[rstest(
    use_txn_payload_v2_format,
    use_orderless_transactions,
    case(false, false),
    case(true, false),
    case(true, true)
)]
async fn test_invalid_args_rejected(
    use_txn_payload_v2_format: bool,
    use_orderless_transactions: bool,
) {
    let (context, account) = setup_public_struct_test(
        current_function_name!(),
        use_txn_payload_v2_format,
        use_orderless_transactions,
    )
    .await;
    let account_addr = account.address();
    let seq = account.sequence_number().to_string();

    // (entry function, bad arguments JSON, expected error substring)
    let cases: &[(&str, serde_json::Value, &str)] = &[
        (
            "test_color",
            json!([{ "InvalidVariant": {} }]),
            "Variant InvalidVariant not found",
        ),
        (
            "test_point",
            json!([{ "x": "10" }]), // missing y
            "field y not found",
        ),
        (
            "test_color",
            json!([{ "Custom": { "r": 100, "g": 50 } }]), // missing b
            "field b not found",
        ),
        (
            "test_color",
            json!([{ "Red": {}, "Blue": {} }]), // two variants
            "Enum value must have exactly one variant specified",
        ),
        (
            "test_option_point",
            json!([{ "vec": [{ "x": "10" }] }]), // missing y inside Option::Some
            "field y not found",
        ),
        (
            "test_option_color",
            json!([{ "vec": [{ "InvalidColor": {} }] }]),
            "Variant InvalidColor not found",
        ),
        // Unknown extra field in enum variant body → line 1119–1125 of convert.rs
        (
            "test_color",
            json!([{ "Custom": { "r": 100, "g": 50, "b": 25, "extra": 99 } }]),
            "Unknown fields",
        ),
        // Non-object value passed as enum → line 1072 of convert.rs
        (
            "test_color",
            json!(["Red"]),
            "Expecting a JSON Map for enum",
        ),
        // Non-object value for enum variant body → line 1104 of convert.rs
        (
            "test_color",
            json!([{ "Red": "not_an_object" }]),
            "Expecting a JSON Map for variant fields",
        ),
    ];

    for (func, args, expected_err) in cases {
        let req = json!({
            "sender": format!("0x{}", account_addr.to_hex()),
            "sequence_number": seq,
            "gas_unit_price": "100",
            "max_gas_amount": "1000000",
            "expiration_timestamp_secs": "9991638487317",
            "payload": {
                "type": "entry_function_payload",
                "function": format!("0x{}::public_struct_test::{}", account_addr.to_hex(), func),
                "type_arguments": [],
                "arguments": args
            }
        });

        let resp = context
            .expect_status_code(400)
            .post("/transactions/encode_submission", req)
            .await;

        assert!(
            resp["message"].as_str().unwrap().contains(expected_err),
            "function '{}': expected error '{}', got: {}",
            func,
            expected_err,
            resp["message"]
        );
    }
}

/// Test that Container<T> with non-public T is rejected.
/// Container has public visibility and copy ability, but T (PrivatePoint) is not public,
/// so the transaction fails with INVALID_MAIN_FUNCTION_SIGNATURE and is discarded.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[rstest(use_orderless_transactions, case(false), case(true))]
async fn test_generic_container_with_non_public_type_arg_rejected(
    use_orderless_transactions: bool,
) {
    let (mut context, mut account) =
        setup_public_struct_test(current_function_name!(), true, use_orderless_transactions).await;
    let account_addr = account.address();

    // Try to call test_generic_container<PrivatePoint> with Container<PrivatePoint>
    // The VM should reject this because PrivatePoint is not public (no public pack function)
    let payload = json!({
        "type": "entry_function_payload",
        "function": format!("0x{}::public_struct_test::test_generic_container", account_addr.to_hex()),
        "type_arguments": [format!("0x{}::public_struct_test::PrivatePoint", account_addr.to_hex())],
        "arguments": [{ "value": { "x": "10", "y": "20" } }]
    });

    // Submit the transaction — it fails with INVALID_MAIN_FUNCTION_SIGNATURE and is discarded.
    context.api_execute_txn(&mut account, payload).await;

    // Get all transactions to verify test_generic_container was NOT committed
    let ledger_version = context.get_latest_ledger_info().version();
    let limit = std::cmp::min(ledger_version + 1, u16::MAX as u64) as u16;
    let txns = context.get_transactions(0, limit);

    // Verify that NO transaction with test_generic_container exists in the ledger
    // The validation should have rejected it before execution
    let found_test_txn = txns.iter().any(|txn| {
        if let aptos_types::transaction::Transaction::UserTransaction(signed_txn) = &txn.transaction
        {
            match signed_txn.payload() {
                aptos_types::transaction::TransactionPayload::Payload(
                    aptos_types::transaction::TransactionPayloadInner::V1 {
                        executable:
                            aptos_types::transaction::TransactionExecutable::EntryFunction(ef),
                        ..
                    },
                ) => ef.function().as_str() == "test_generic_container",
                aptos_types::transaction::TransactionPayload::EntryFunction(ef) => {
                    ef.function().as_str() == "test_generic_container"
                },
                _ => false,
            }
        } else {
            false
        }
    });

    assert!(
        !found_test_txn,
        "Transaction with test_generic_container should NOT exist in ledger - validation should have rejected it"
    );
}
