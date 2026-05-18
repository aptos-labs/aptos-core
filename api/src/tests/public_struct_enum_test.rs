// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! API tests for public structs and enums with copy ability as transaction arguments.
//!
//! These tests verify that the API correctly handles JSON to BCS conversion
//! for public structs and enums passed as entry function arguments.

use super::setup_public_struct_test;
use aptos_api_test_context::current_function_name;
use aptos_types::account_address::AccountAddress;
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
async fn test_public_struct_enum_entry_functions() {
    let (mut context, mut account) =
        setup_public_struct_test(current_function_name!(), false, false).await;
    let account_addr = account.address();

    // test_public_struct_point: Point { x: 10, y: 20 }
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

    // test_public_struct_nested: Rectangle with nested Points
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

    // test_public_struct_with_string: Data { values: [5, 10, 15], name: "test_data" }
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

    // test_public_enum_unit_variant: Color::Red
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

    // test_public_enum_with_fields: Color::Custom { r: 100, g: 50, b: 25 }
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

    // test_public_enum_with_struct_fields: Shape::Circle { center: Point, radius: 15 }
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

    // test_vector_of_public_structs: vector of Points
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

    // test_whitelisted_string_works: String value
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

    // test_option_some_struct: Option<Point>::Some
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

    // test_option_none_struct: Option<Point>::None
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

    // test_option_some_enum: Option<Color>::Some(Color::Red)
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

    // test_option_none_enum: Option<Color>::None
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

    // test_struct_with_enum_field: Labeled { color: Color::Green, value: 10 }
    context
        .api_execute_entry_function(
            &mut account,
            &format!(
                "0x{}::public_struct_test::test_labeled",
                account_addr.to_hex()
            ),
            json!([]),
            json!([{ "color": { "Green": {} }, "value": "10" }]),
        )
        .await;
    assert_test_result(&mut context, &account_addr, "12", "labeled_received").await;

    // test_nested_vector_of_structs: vector<vector<Point>>
    context
        .api_execute_entry_function(
            &mut account,
            &format!(
                "0x{}::public_struct_test::test_nested_point_vector",
                account_addr.to_hex()
            ),
            json!([]),
            json!([[
                [{ "x": "1", "y": "2" }, { "x": "3", "y": "4" }],
                [{ "x": "5", "y": "6" }]
            ]]),
        )
        .await;
    assert_test_result(
        &mut context,
        &account_addr,
        "21",
        "nested_point_vector_received",
    )
    .await;

    // test_generic_container_with_enum: Container<Color> containing Red
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

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_public_struct_enum_views() {
    let (context, account) = setup_public_struct_test(current_function_name!(), false, false).await;
    let account_addr = account.address();

    // test_view_with_public_struct: check_point(Point{3,7}) = 10
    let request: Value = json!({
        "function": format!("0x{}::public_struct_test::check_point", account_addr.to_hex()),
        "type_arguments": [],
        "arguments": [{ "x": "3", "y": "7" }],
    });
    let resp = context.post("/view", request).await;
    assert_eq!(resp, json!(["10"]));

    // test_view_with_public_enum: check_color(Custom{10,20,30}) = 60
    let request: Value = json!({
        "function": format!("0x{}::public_struct_test::check_color", account_addr.to_hex()),
        "type_arguments": [],
        "arguments": [{ "Custom": { "r": 10, "g": 20, "b": 30 } }],
    });
    let resp = context.post("/view", request).await;
    assert_eq!(resp, json!(["60"]));

    // test_view_with_multiple_struct_args: check_two_points(Point{1,2}, Point{3,4}) = 10
    let request: Value = json!({
        "function": format!("0x{}::public_struct_test::check_two_points", account_addr.to_hex()),
        "type_arguments": [],
        "arguments": [{ "x": "1", "y": "2" }, { "x": "3", "y": "4" }],
    });
    let resp = context.post("/view", request).await;
    assert_eq!(resp, json!(["10"]));
}
