// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! API tests for public structs and enums with copy ability as transaction arguments.
//!
//! These tests verify that the API correctly handles JSON to BCS conversion
//! for public structs and enums passed as entry function arguments.

use super::new_test_context_with_orderless_flags;
use aptos_api_test_context::{current_function_name, TestContext};
use rstest::rstest;
use serde_json::json;
use std::path::PathBuf;

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[rstest(
    use_txn_payload_v2_format,
    use_orderless_transactions,
    case(false, false),
    case(true, false),
    case(true, true)
)]
async fn test_public_struct_point(
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

    // Publish the test package
    let named_addresses = vec![("account".to_string(), account_addr)];
    let txn = futures::executor::block_on(async move {
        let path = PathBuf::from(std::env!("CARGO_MANIFEST_DIR"))
            .join("src/tests/move/public_struct_enum_test");
        TestContext::build_package_with_latest_language(path, named_addresses)
    });
    context.publish_package(&mut account, txn).await;

    // Call entry function with a Point struct: { x: 10, y: 20 }
    context
        .api_execute_entry_function(
            &mut account,
            &format!(
                "0x{}::public_struct_test::test_point",
                account_addr.to_hex()
            ),
            json!([]),
            json!([{ "x": "10", "y": "20" }]),
        )
        .await;

    // Verify the result
    let resource = format!("{}::public_struct_test::TestResult", account_addr);
    let response = &context
        .gen_resource(&account_addr, &resource)
        .await
        .unwrap();

    assert_eq!(
        response["data"],
        json!({
            "value": "30",
            "message": "point_received",
        })
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
async fn test_public_struct_nested(
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

    // Publish the test package
    let named_addresses = vec![("account".to_string(), account_addr)];
    let txn = futures::executor::block_on(async move {
        let path = PathBuf::from(std::env!("CARGO_MANIFEST_DIR"))
            .join("src/tests/move/public_struct_enum_test");
        TestContext::build_package_with_latest_language(path, named_addresses)
    });
    context.publish_package(&mut account, txn).await;

    // Call entry function with a Rectangle struct with nested Points
    context
        .api_execute_entry_function(
            &mut account,
            &format!(
                "0x{}::public_struct_test::test_rectangle",
                account_addr.to_hex()
            ),
            json!([]),
            json!([{
                "top_left": { "x": "1", "y": "2" },
                "bottom_right": { "x": "3", "y": "4" }
            }]),
        )
        .await;

    // Verify the result
    let resource = format!("{}::public_struct_test::TestResult", account_addr);
    let response = &context
        .gen_resource(&account_addr, &resource)
        .await
        .unwrap();

    assert_eq!(
        response["data"],
        json!({
            "value": "10",
            "message": "rectangle_received",
        })
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
async fn test_public_struct_with_string(
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

    // Publish the test package
    let named_addresses = vec![("account".to_string(), account_addr)];
    let txn = futures::executor::block_on(async move {
        let path = PathBuf::from(std::env!("CARGO_MANIFEST_DIR"))
            .join("src/tests/move/public_struct_enum_test");
        TestContext::build_package_with_latest_language(path, named_addresses)
    });
    context.publish_package(&mut account, txn).await;

    // Call entry function with a Data struct: { values: [5, 10, 15], name: "test_data" }
    context
        .api_execute_entry_function(
            &mut account,
            &format!("0x{}::public_struct_test::test_data", account_addr.to_hex()),
            json!([]),
            json!([{
                "values": ["5", "10", "15"],
                "name": "test_data"
            }]),
        )
        .await;

    // Verify the result
    let resource = format!("{}::public_struct_test::TestResult", account_addr);
    let response = &context
        .gen_resource(&account_addr, &resource)
        .await
        .unwrap();

    assert_eq!(
        response["data"],
        json!({
            "value": "30",
            "message": "test_data",
        })
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
async fn test_public_enum_unit_variant(
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

    // Publish the test package
    let named_addresses = vec![("account".to_string(), account_addr)];
    let txn = futures::executor::block_on(async move {
        let path = PathBuf::from(std::env!("CARGO_MANIFEST_DIR"))
            .join("src/tests/move/public_struct_enum_test");
        TestContext::build_package_with_latest_language(path, named_addresses)
    });
    context.publish_package(&mut account, txn).await;

    // Call entry function with Color::Red enum variant
    context
        .api_execute_entry_function(
            &mut account,
            &format!(
                "0x{}::public_struct_test::test_color",
                account_addr.to_hex()
            ),
            json!([]),
            json!([{ "Red": {} }]),
        )
        .await;

    // Verify the result
    let resource = format!("{}::public_struct_test::TestResult", account_addr);
    let response = &context
        .gen_resource(&account_addr, &resource)
        .await
        .unwrap();

    assert_eq!(
        response["data"],
        json!({
            "value": "1",
            "message": "red",
        })
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
async fn test_public_enum_with_fields(
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

    // Publish the test package
    let named_addresses = vec![("account".to_string(), account_addr)];
    let txn = futures::executor::block_on(async move {
        let path = PathBuf::from(std::env!("CARGO_MANIFEST_DIR"))
            .join("src/tests/move/public_struct_enum_test");
        TestContext::build_package_with_latest_language(path, named_addresses)
    });
    context.publish_package(&mut account, txn).await;

    // Call entry function with Color::Custom { r: 100, g: 50, b: 25 }
    context
        .api_execute_entry_function(
            &mut account,
            &format!(
                "0x{}::public_struct_test::test_color",
                account_addr.to_hex()
            ),
            json!([]),
            json!([{ "Custom": { "r": 100, "g": 50, "b": 25 } }]),
        )
        .await;

    // Verify the result
    let resource = format!("{}::public_struct_test::TestResult", account_addr);
    let response = &context
        .gen_resource(&account_addr, &resource)
        .await
        .unwrap();

    assert_eq!(
        response["data"],
        json!({
            "value": "175",
            "message": "custom",
        })
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
async fn test_public_enum_with_struct_fields(
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

    // Publish the test package
    let named_addresses = vec![("account".to_string(), account_addr)];
    let txn = futures::executor::block_on(async move {
        let path = PathBuf::from(std::env!("CARGO_MANIFEST_DIR"))
            .join("src/tests/move/public_struct_enum_test");
        TestContext::build_package_with_latest_language(path, named_addresses)
    });
    context.publish_package(&mut account, txn).await;

    // Call entry function with Shape::Circle { center: Point { x: 5, y: 10 }, radius: 15 }
    context
        .api_execute_entry_function(
            &mut account,
            &format!(
                "0x{}::public_struct_test::test_shape",
                account_addr.to_hex()
            ),
            json!([]),
            json!([{
                "Circle": {
                    "center": { "x": "5", "y": "10" },
                    "radius": "15"
                }
            }]),
        )
        .await;

    // Verify the result
    let resource = format!("{}::public_struct_test::TestResult", account_addr);
    let response = &context
        .gen_resource(&account_addr, &resource)
        .await
        .unwrap();

    assert_eq!(
        response["data"],
        json!({
            "value": "30",
            "message": "circle",
        })
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
async fn test_vector_of_public_structs(
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

    // Publish the test package
    let named_addresses = vec![("account".to_string(), account_addr)];
    let txn = futures::executor::block_on(async move {
        let path = PathBuf::from(std::env!("CARGO_MANIFEST_DIR"))
            .join("src/tests/move/public_struct_enum_test");
        TestContext::build_package_with_latest_language(path, named_addresses)
    });
    context.publish_package(&mut account, txn).await;

    // Call entry function with a vector of Points
    context
        .api_execute_entry_function(
            &mut account,
            &format!(
                "0x{}::public_struct_test::test_point_vector",
                account_addr.to_hex()
            ),
            json!([]),
            json!([[
                { "x": "1", "y": "2" },
                { "x": "3", "y": "4" },
                { "x": "5", "y": "6" }
            ]]),
        )
        .await;

    // Verify the result
    let resource = format!("{}::public_struct_test::TestResult", account_addr);
    let response = &context
        .gen_resource(&account_addr, &resource)
        .await
        .unwrap();

    assert_eq!(
        response["data"],
        json!({
            "value": "21",
            "message": "point_vector_received",
        })
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
async fn test_whitelisted_string_works(
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

    // Publish the test package
    let named_addresses = vec![("account".to_string(), account_addr)];
    let txn = futures::executor::block_on(async move {
        let path = PathBuf::from(std::env!("CARGO_MANIFEST_DIR"))
            .join("src/tests/move/public_struct_enum_test");
        TestContext::build_package_with_latest_language(path, named_addresses)
    });
    context.publish_package(&mut account, txn).await;

    // Call entry function with a String value
    context
        .api_execute_entry_function(
            &mut account,
            &format!(
                "0x{}::public_struct_test::test_string",
                account_addr.to_hex()
            ),
            json!([]),
            json!(["hello_world"]),
        )
        .await;

    // Verify the result
    let resource = format!("{}::public_struct_test::TestResult", account_addr);
    let response = &context
        .gen_resource(&account_addr, &resource)
        .await
        .unwrap();

    assert_eq!(
        response["data"],
        json!({
            "value": "11",
            "message": "hello_world",
        })
    );
}

/// Test passing Option<Point> with Some value via API
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[rstest(
    use_txn_payload_v2_format,
    use_orderless_transactions,
    case(false, false),
    case(true, false),
    case(true, true)
)]
async fn test_option_some_struct(
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

    // Publish the test package
    let named_addresses = vec![("account".to_string(), account_addr)];
    let txn = futures::executor::block_on(async move {
        let path = PathBuf::from(std::env!("CARGO_MANIFEST_DIR"))
            .join("src/tests/move/public_struct_enum_test");
        TestContext::build_package_with_latest_language(path, named_addresses)
    });
    context.publish_package(&mut account, txn).await;

    // Call entry function with Option<Point>::Some
    // Option uses vector-based JSON representation: Some(x) = {"vec": [x]}, None = {"vec": []}
    context
        .api_execute_entry_function(
            &mut account,
            &format!(
                "0x{}::public_struct_test::test_option_point",
                account_addr.to_hex()
            ),
            json!([]),
            json!([{ "vec": [{ "x": "10", "y": "20" }] }]),
        )
        .await;

    // Verify the result
    let resource = format!("{}::public_struct_test::TestResult", account_addr);
    let response = &context
        .gen_resource(&account_addr, &resource)
        .await
        .unwrap();

    assert_eq!(
        response["data"],
        json!({
            "value": "30",
            "message": "some_point",
        })
    );
}

/// Test passing Option<Point> with None value via API
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[rstest(
    use_txn_payload_v2_format,
    use_orderless_transactions,
    case(false, false),
    case(true, false),
    case(true, true)
)]
async fn test_option_none_struct(
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

    // Publish the test package
    let named_addresses = vec![("account".to_string(), account_addr)];
    let txn = futures::executor::block_on(async move {
        let path = PathBuf::from(std::env!("CARGO_MANIFEST_DIR"))
            .join("src/tests/move/public_struct_enum_test");
        TestContext::build_package_with_latest_language(path, named_addresses)
    });
    context.publish_package(&mut account, txn).await;

    // Call entry function with Option<Point>::None
    // Option uses vector-based JSON representation: None = {"vec": []}
    context
        .api_execute_entry_function(
            &mut account,
            &format!(
                "0x{}::public_struct_test::test_option_point",
                account_addr.to_hex()
            ),
            json!([]),
            json!([{ "vec": [] }]),
        )
        .await;

    // Verify the result
    let resource = format!("{}::public_struct_test::TestResult", account_addr);
    let response = &context
        .gen_resource(&account_addr, &resource)
        .await
        .unwrap();

    assert_eq!(
        response["data"],
        json!({
            "value": "0",
            "message": "none_point",
        })
    );
}

/// Test passing Option<Color> with Some(Red) via API
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[rstest(
    use_txn_payload_v2_format,
    use_orderless_transactions,
    case(false, false),
    case(true, false),
    case(true, true)
)]
async fn test_option_some_enum(use_txn_payload_v2_format: bool, use_orderless_transactions: bool) {
    let mut context = new_test_context_with_orderless_flags(
        current_function_name!(),
        use_txn_payload_v2_format,
        use_orderless_transactions,
    );
    let mut account = context.create_account().await;
    let account_addr = account.address();

    // Publish the test package
    let named_addresses = vec![("account".to_string(), account_addr)];
    let txn = futures::executor::block_on(async move {
        let path = PathBuf::from(std::env!("CARGO_MANIFEST_DIR"))
            .join("src/tests/move/public_struct_enum_test");
        TestContext::build_package_with_latest_language(path, named_addresses)
    });
    context.publish_package(&mut account, txn).await;

    // Call entry function with Option<Color>::Some(Color::Red)
    // Option uses vector-based JSON representation: Some(x) = {"vec": [x]}, None = {"vec": []}
    // Option<enum> uses vec format with variant name as key
    context
        .api_execute_entry_function(
            &mut account,
            &format!(
                "0x{}::public_struct_test::test_option_color",
                account_addr.to_hex()
            ),
            json!([]),
            json!([{ "vec": [{ "Red": {} }] }]),
        )
        .await;

    // Verify the result
    let resource = format!("{}::public_struct_test::TestResult", account_addr);
    let response = &context
        .gen_resource(&account_addr, &resource)
        .await
        .unwrap();

    assert_eq!(
        response["data"],
        json!({
            "value": "1",
            "message": "some_red",
        })
    );
}

/// Test passing Option<Color> with None via API
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[rstest(
    use_txn_payload_v2_format,
    use_orderless_transactions,
    case(false, false),
    case(true, false),
    case(true, true)
)]
async fn test_option_none_enum(use_txn_payload_v2_format: bool, use_orderless_transactions: bool) {
    let mut context = new_test_context_with_orderless_flags(
        current_function_name!(),
        use_txn_payload_v2_format,
        use_orderless_transactions,
    );
    let mut account = context.create_account().await;
    let account_addr = account.address();

    // Publish the test package
    let named_addresses = vec![("account".to_string(), account_addr)];
    let txn = futures::executor::block_on(async move {
        let path = PathBuf::from(std::env!("CARGO_MANIFEST_DIR"))
            .join("src/tests/move/public_struct_enum_test");
        TestContext::build_package_with_latest_language(path, named_addresses)
    });
    context.publish_package(&mut account, txn).await;

    // Call entry function with Option<Color>::None
    // Option uses vector-based JSON representation: None = {"vec": []}
    context
        .api_execute_entry_function(
            &mut account,
            &format!(
                "0x{}::public_struct_test::test_option_color",
                account_addr.to_hex()
            ),
            json!([]),
            json!([{ "vec": [] }]),
        )
        .await;

    // Verify the result
    let resource = format!("{}::public_struct_test::TestResult", account_addr);
    let response = &context
        .gen_resource(&account_addr, &resource)
        .await
        .unwrap();

    assert_eq!(
        response["data"],
        json!({
            "value": "0",
            "message": "none_color",
        })
    );
}

/// Test passing Container<Color> (generic struct with enum type argument) via API
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[rstest(
    use_txn_payload_v2_format,
    use_orderless_transactions,
    case(false, false),
    case(true, false),
    case(true, true)
)]
async fn test_generic_container_with_enum(
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

    // Publish the package
    let named_addresses = vec![("account".to_string(), account_addr)];
    let txn = futures::executor::block_on(async move {
        let path = PathBuf::from(std::env!("CARGO_MANIFEST_DIR"))
            .join("src/tests/move/public_struct_enum_test");
        TestContext::build_package_with_latest_language(path, named_addresses)
    });
    context.publish_package(&mut account, txn).await;

    // Call entry function with Container<Color> containing Red
    // Container is a generic struct: struct Container<T> { value: T }
    // JSON format: { "value": { "Red": {} } }
    context
        .api_execute_entry_function(
            &mut account,
            &format!(
                "0x{}::public_struct_test::test_container_color",
                account_addr.to_hex()
            ),
            json!([]),
            json!([{ "value": { "Red": {} } }]),
        )
        .await;

    // Verify the result
    let resource = format!("{}::public_struct_test::TestResult", account_addr);
    let response = &context
        .gen_resource(&account_addr, &resource)
        .await
        .unwrap();

    assert_eq!(
        response["data"],
        json!({
            "value": "100",
            "message": "container_red",
        })
    );
}
