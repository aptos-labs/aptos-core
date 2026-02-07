// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Negative tests for public structs and enums as transaction arguments.
//!
//! These tests verify the API correctly rejects invalid struct/enum arguments.

use super::new_test_context_with_orderless_flags;
use aptos_api_test_context::{current_function_name, TestContext};
use rstest::rstest;
use serde_json::json;
use std::path::PathBuf;

/// Test that wrong variant name is rejected
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[rstest(
    use_txn_payload_v2_format,
    use_orderless_transactions,
    case(false, false),
    case(true, false),
    case(true, true)
)]
async fn test_wrong_variant_name_rejected(
    use_txn_payload_v2_format: bool,
    use_orderless_transactions: bool,
) {
    let mut context = new_test_context_with_orderless_flags(
        current_function_name!(),
        use_txn_payload_v2_format,
        use_orderless_transactions,
    );
    let mut account = context.create_account().await;
    let account_addr = account.address();

    // Publish valid package
    let named_addresses = vec![("account".to_string(), account_addr)];
    let txn = futures::executor::block_on(async move {
        let path = PathBuf::from(std::env!("CARGO_MANIFEST_DIR"))
            .join("src/tests/move/public_struct_enum_test");
        TestContext::build_package_with_latest_language(path, named_addresses)
    });
    context.publish_package(&mut account, txn).await;

    // Try to call with wrong variant name
    let req = json!({
        "sender": format!("0x{}", account_addr.to_hex()),
        "sequence_number": account.sequence_number().to_string(),
        "gas_unit_price": "100",
        "max_gas_amount": "1000000",
        "expiration_timestamp_secs": "9991638487317",
        "payload": {
            "type": "entry_function_payload",
            "function": format!("0x{}::public_struct_test::test_color", account_addr.to_hex()),
            "type_arguments": [],
            "arguments": [json!({ "InvalidVariant": {} })]
        }
    });

    let resp = context
        .expect_status_code(400)
        .post("/transactions/encode_submission", req)
        .await;

    assert!(resp["message"]
        .as_str()
        .unwrap()
        .contains("Variant InvalidVariant not found"));
}

/// Test that missing required field is rejected
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[rstest(
    use_txn_payload_v2_format,
    use_orderless_transactions,
    case(false, false),
    case(true, false),
    case(true, true)
)]
async fn test_missing_required_field_rejected(
    use_txn_payload_v2_format: bool,
    use_orderless_transactions: bool,
) {
    let mut context = new_test_context_with_orderless_flags(
        current_function_name!(),
        use_txn_payload_v2_format,
        use_orderless_transactions,
    );
    let mut account = context.create_account().await;
    let account_addr = account.address();

    // Publish valid package
    let named_addresses = vec![("account".to_string(), account_addr)];
    let txn = futures::executor::block_on(async move {
        let path = PathBuf::from(std::env!("CARGO_MANIFEST_DIR"))
            .join("src/tests/move/public_struct_enum_test");
        TestContext::build_package_with_latest_language(path, named_addresses)
    });
    context.publish_package(&mut account, txn).await;

    // Try to call with missing field in struct
    let req = json!({
        "sender": format!("0x{}", account_addr.to_hex()),
        "sequence_number": account.sequence_number().to_string(),
        "gas_unit_price": "100",
        "max_gas_amount": "1000000",
        "expiration_timestamp_secs": "9991638487317",
        "payload": {
            "type": "entry_function_payload",
            "function": format!("0x{}::public_struct_test::test_point", account_addr.to_hex()),
            "type_arguments": [],
            "arguments": [json!({ "x": "10" })] // Missing "y" field
        }
    });

    let resp = context
        .expect_status_code(400)
        .post("/transactions/encode_submission", req)
        .await;

    assert!(resp["message"]
        .as_str()
        .unwrap()
        .contains("field y not found"));
}

/// Test that missing required field in enum variant is rejected
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[rstest(
    use_txn_payload_v2_format,
    use_orderless_transactions,
    case(false, false),
    case(true, false),
    case(true, true)
)]
async fn test_enum_variant_missing_field_rejected(
    use_txn_payload_v2_format: bool,
    use_orderless_transactions: bool,
) {
    let mut context = new_test_context_with_orderless_flags(
        current_function_name!(),
        use_txn_payload_v2_format,
        use_orderless_transactions,
    );
    let mut account = context.create_account().await;
    let account_addr = account.address();

    // Publish valid package
    let named_addresses = vec![("account".to_string(), account_addr)];
    let txn = futures::executor::block_on(async move {
        let path = PathBuf::from(std::env!("CARGO_MANIFEST_DIR"))
            .join("src/tests/move/public_struct_enum_test");
        TestContext::build_package_with_latest_language(path, named_addresses)
    });
    context.publish_package(&mut account, txn).await;

    // Try to call with missing field in enum variant
    let req = json!({
        "sender": format!("0x{}", account_addr.to_hex()),
        "sequence_number": account.sequence_number().to_string(),
        "gas_unit_price": "100",
        "max_gas_amount": "1000000",
        "expiration_timestamp_secs": "9991638487317",
        "payload": {
            "type": "entry_function_payload",
            "function": format!("0x{}::public_struct_test::test_color", account_addr.to_hex()),
            "type_arguments": [],
            "arguments": [json!({ "Custom": { "r": 100, "g": 50 } })] // Missing "b" field
        }
    });

    let resp = context
        .expect_status_code(400)
        .post("/transactions/encode_submission", req)
        .await;

    assert!(resp["message"]
        .as_str()
        .unwrap()
        .contains("field b not found"));
}

/// Test that multiple variants in enum value is rejected
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[rstest(
    use_txn_payload_v2_format,
    use_orderless_transactions,
    case(false, false),
    case(true, false),
    case(true, true)
)]
async fn test_multiple_enum_variants_rejected(
    use_txn_payload_v2_format: bool,
    use_orderless_transactions: bool,
) {
    let mut context = new_test_context_with_orderless_flags(
        current_function_name!(),
        use_txn_payload_v2_format,
        use_orderless_transactions,
    );
    let mut account = context.create_account().await;
    let account_addr = account.address();

    // Publish valid package
    let named_addresses = vec![("account".to_string(), account_addr)];
    let txn = futures::executor::block_on(async move {
        let path = PathBuf::from(std::env!("CARGO_MANIFEST_DIR"))
            .join("src/tests/move/public_struct_enum_test");
        TestContext::build_package_with_latest_language(path, named_addresses)
    });
    context.publish_package(&mut account, txn).await;

    // Try to call with multiple variants specified
    let req = json!({
        "sender": format!("0x{}", account_addr.to_hex()),
        "sequence_number": account.sequence_number().to_string(),
        "gas_unit_price": "100",
        "max_gas_amount": "1000000",
        "expiration_timestamp_secs": "9991638487317",
        "payload": {
            "type": "entry_function_payload",
            "function": format!("0x{}::public_struct_test::test_color", account_addr.to_hex()),
            "type_arguments": [],
            "arguments": [json!({ "Red": {}, "Blue": {} })] // Two variants
        }
    });

    let resp = context
        .expect_status_code(400)
        .post("/transactions/encode_submission", req)
        .await;

    assert!(resp["message"]
        .as_str()
        .unwrap()
        .contains("Enum value must have exactly one variant specified"));
}

/// Test that Option::Some with missing field in inner struct is rejected
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[rstest(
    use_txn_payload_v2_format,
    use_orderless_transactions,
    case(false, false),
    case(true, false),
    case(true, true)
)]
async fn test_option_some_missing_field_rejected(
    use_txn_payload_v2_format: bool,
    use_orderless_transactions: bool,
) {
    let mut context = new_test_context_with_orderless_flags(
        current_function_name!(),
        use_txn_payload_v2_format,
        use_orderless_transactions,
    );
    let mut account = context.create_account().await;
    let account_addr = account.address();

    // Publish valid package
    let named_addresses = vec![("account".to_string(), account_addr)];
    let txn = futures::executor::block_on(async move {
        let path = PathBuf::from(std::env!("CARGO_MANIFEST_DIR"))
            .join("src/tests/move/public_struct_enum_test");
        TestContext::build_package_with_latest_language(path, named_addresses)
    });
    context.publish_package(&mut account, txn).await;

    // Try to call with Option::Some but missing field in Point
    let req = json!({
        "sender": format!("0x{}", account_addr.to_hex()),
        "sequence_number": account.sequence_number().to_string(),
        "gas_unit_price": "100",
        "max_gas_amount": "1000000",
        "expiration_timestamp_secs": "9991638487317",
        "payload": {
            "type": "entry_function_payload",
            "function": format!("0x{}::public_struct_test::test_option_point", account_addr.to_hex()),
            "type_arguments": [],
            "arguments": [{ "vec": [{ "x": "10" }] }] // Missing y field in Point
        }
    });

    let resp = context
        .expect_status_code(400)
        .post("/transactions/encode_submission", req)
        .await;

    assert!(resp["message"]
        .as_str()
        .unwrap()
        .contains("field y not found"));
}

/// Test that Option::Some with invalid enum variant is rejected
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[rstest(
    use_txn_payload_v2_format,
    use_orderless_transactions,
    case(false, false),
    case(true, false),
    case(true, true)
)]
async fn test_option_some_invalid_enum_variant_rejected(
    use_txn_payload_v2_format: bool,
    use_orderless_transactions: bool,
) {
    let mut context = new_test_context_with_orderless_flags(
        current_function_name!(),
        use_txn_payload_v2_format,
        use_orderless_transactions,
    );
    let mut account = context.create_account().await;
    let account_addr = account.address();

    // Publish valid package
    let named_addresses = vec![("account".to_string(), account_addr)];
    let txn = futures::executor::block_on(async move {
        let path = PathBuf::from(std::env!("CARGO_MANIFEST_DIR"))
            .join("src/tests/move/public_struct_enum_test");
        TestContext::build_package_with_latest_language(path, named_addresses)
    });
    context.publish_package(&mut account, txn).await;

    // Try to call with Option::Some but invalid Color variant
    let req = json!({
        "sender": format!("0x{}", account_addr.to_hex()),
        "sequence_number": account.sequence_number().to_string(),
        "gas_unit_price": "100",
        "max_gas_amount": "1000000",
        "expiration_timestamp_secs": "9991638487317",
        "payload": {
            "type": "entry_function_payload",
            "function": format!("0x{}::public_struct_test::test_option_color", account_addr.to_hex()),
            "type_arguments": [],
            "arguments": [{ "vec": [{ "InvalidColor": {} }] }]
        }
    });

    let resp = context
        .expect_status_code(400)
        .post("/transactions/encode_submission", req)
        .await;

    assert!(resp["message"]
        .as_str()
        .unwrap()
        .contains("Variant InvalidColor not found"));
}

/// Test that Container<T> with non-public T is rejected
/// Container has public visibility and copy ability, but T (PrivatePoint) is not public
///
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[rstest(use_orderless_transactions, case(false), case(true))]
async fn test_generic_container_with_non_public_type_arg_rejected(
    use_orderless_transactions: bool,
) {
    let use_txn_payload_v2_format = true;
    let mut context = new_test_context_with_orderless_flags(
        current_function_name!(),
        use_txn_payload_v2_format,
        use_orderless_transactions,
    );
    let mut account = context.create_account().await;
    let account_addr = account.address();

    // Publish valid package
    let named_addresses = vec![("account".to_string(), account_addr)];
    let txn = futures::executor::block_on(async move {
        let path = PathBuf::from(std::env!("CARGO_MANIFEST_DIR"))
            .join("src/tests/move/public_struct_enum_test");
        TestContext::build_package_with_latest_language(path, named_addresses)
    });
    context.publish_package(&mut account, txn).await;

    // Try to call test_generic_container<PrivatePoint> with Container<PrivatePoint>
    // The VM should reject this during execution because PrivatePoint is not public
    let payload = json!({
        "type": "entry_function_payload",
        "function": format!("0x{}::public_struct_test::test_generic_container", account_addr.to_hex()),
        "type_arguments": [format!("0x{}::public_struct_test::PrivatePoint", account_addr.to_hex())],
        "arguments": [{ "value": { "x": "10", "y": "20" } }]
    });

    // Submit the transaction - it should be rejected during validation
    // and NOT appear in the ledger (validation errors currently discard the transaction)
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
