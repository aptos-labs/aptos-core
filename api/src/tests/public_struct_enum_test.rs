// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! API tests for public structs and enums with copy ability as transaction arguments.
//!
//! These tests verify that the API correctly handles JSON to BCS conversion
//! for public structs and enums passed as entry function arguments.

use super::setup_public_struct_test;
use aptos_api_test_context::current_function_name;
use aptos_types::account_address::AccountAddress;
use rstest::rstest;
use serde_json::{json, Value};

/// Fetch the `TestResult` resource for `account_addr` and assert its `value` and `message` fields.
async fn assert_test_result(
    context: &mut super::TestContext,
    account_addr: &AccountAddress,
    expected_value: &str,
    expected_message: &str,
) {
    let resource = format!("{}::public_struct_test::TestResult", account_addr);
    let response = context.gen_resource(account_addr, &resource).await.unwrap();
    assert_eq!(
        response["data"],
        json!({ "value": expected_value, "message": expected_message })
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
async fn test_public_struct_point(
    use_txn_payload_v2_format: bool,
    use_orderless_transactions: bool,
) {
    let (mut context, mut account) = setup_public_struct_test(
        current_function_name!(),
        use_txn_payload_v2_format,
        use_orderless_transactions,
    )
    .await;
    let account_addr = account.address();

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

    assert_test_result(&mut context, &account_addr, "30", "point_received").await;
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
    let (mut context, mut account) = setup_public_struct_test(
        current_function_name!(),
        use_txn_payload_v2_format,
        use_orderless_transactions,
    )
    .await;
    let account_addr = account.address();

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

    assert_test_result(&mut context, &account_addr, "10", "rectangle_received").await;
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
    let (mut context, mut account) = setup_public_struct_test(
        current_function_name!(),
        use_txn_payload_v2_format,
        use_orderless_transactions,
    )
    .await;
    let account_addr = account.address();

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

    assert_test_result(&mut context, &account_addr, "30", "test_data").await;
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
    let (mut context, mut account) = setup_public_struct_test(
        current_function_name!(),
        use_txn_payload_v2_format,
        use_orderless_transactions,
    )
    .await;
    let account_addr = account.address();

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

    assert_test_result(&mut context, &account_addr, "1", "red").await;
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
    let (mut context, mut account) = setup_public_struct_test(
        current_function_name!(),
        use_txn_payload_v2_format,
        use_orderless_transactions,
    )
    .await;
    let account_addr = account.address();

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

    assert_test_result(&mut context, &account_addr, "175", "custom").await;
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
    let (mut context, mut account) = setup_public_struct_test(
        current_function_name!(),
        use_txn_payload_v2_format,
        use_orderless_transactions,
    )
    .await;
    let account_addr = account.address();

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

    assert_test_result(&mut context, &account_addr, "30", "circle").await;
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
    let (mut context, mut account) = setup_public_struct_test(
        current_function_name!(),
        use_txn_payload_v2_format,
        use_orderless_transactions,
    )
    .await;
    let account_addr = account.address();

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

    assert_test_result(&mut context, &account_addr, "21", "point_vector_received").await;
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
    let (mut context, mut account) = setup_public_struct_test(
        current_function_name!(),
        use_txn_payload_v2_format,
        use_orderless_transactions,
    )
    .await;
    let account_addr = account.address();

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

    assert_test_result(&mut context, &account_addr, "11", "hello_world").await;
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
    let (mut context, mut account) = setup_public_struct_test(
        current_function_name!(),
        use_txn_payload_v2_format,
        use_orderless_transactions,
    )
    .await;
    let account_addr = account.address();

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

    assert_test_result(&mut context, &account_addr, "30", "some_point").await;
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
    let (mut context, mut account) = setup_public_struct_test(
        current_function_name!(),
        use_txn_payload_v2_format,
        use_orderless_transactions,
    )
    .await;
    let account_addr = account.address();

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

    assert_test_result(&mut context, &account_addr, "0", "none_point").await;
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
    let (mut context, mut account) = setup_public_struct_test(
        current_function_name!(),
        use_txn_payload_v2_format,
        use_orderless_transactions,
    )
    .await;
    let account_addr = account.address();

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

    assert_test_result(&mut context, &account_addr, "1", "some_red").await;
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
    let (mut context, mut account) = setup_public_struct_test(
        current_function_name!(),
        use_txn_payload_v2_format,
        use_orderless_transactions,
    )
    .await;
    let account_addr = account.address();

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

    assert_test_result(&mut context, &account_addr, "0", "none_color").await;
}

/// Test calling a view function that takes a public struct as argument.
///
/// This exercises the `/view` endpoint code path through `convert_view_function` →
/// `try_into_vm_values` → `try_into_vm_value_struct`, which is a different entry point
/// from the entry-function submission path and is not covered by e2e move tests.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_view_with_public_struct() {
    let (context, account) = setup_public_struct_test(current_function_name!(), false, false).await;
    let account_addr = account.address();

    let request: Value = json!({
        "function": format!("0x{}::public_struct_test::check_point", account_addr.to_hex()),
        "type_arguments": [],
        "arguments": [{ "x": "3", "y": "7" }],
    });

    let resp = context.post("/view", request).await;

    // check_point returns p.x + p.y = 3 + 7 = 10
    assert_eq!(resp, json!(["10"]));
}

/// Test calling a view function that takes a public enum as argument.
///
/// Covers the same `/view` code path as test_view_with_public_struct but with
/// enum variant parsing (WithVariants layout) rather than plain struct parsing.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_view_with_public_enum() {
    let (context, account) = setup_public_struct_test(current_function_name!(), false, false).await;
    let account_addr = account.address();

    let request: Value = json!({
        "function": format!("0x{}::public_struct_test::check_color", account_addr.to_hex()),
        "type_arguments": [],
        "arguments": [{ "Custom": { "r": 10, "g": 20, "b": 30 } }],
    });

    let resp = context.post("/view", request).await;

    // check_color returns r + g + b = 10 + 20 + 30 = 60
    assert_eq!(resp, json!(["60"]));
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
    let (mut context, mut account) = setup_public_struct_test(
        current_function_name!(),
        use_txn_payload_v2_format,
        use_orderless_transactions,
    )
    .await;
    let account_addr = account.address();

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

    assert_test_result(&mut context, &account_addr, "100", "container_red").await;
}
