// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use super::new_test_context_with_orderless_flags;
use aptos_api_test_context::{current_function_name, TestContext};
use rstest::rstest;
use std::path::PathBuf;

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[rstest(
    use_txn_payload_v2_format,
    use_orderless_transactions,
    case(false, false),
    case(true, false),
    case(true, true)
)]
async fn test_abi(use_txn_payload_v2_format: bool, use_orderless_transactions: bool) {
    let mut context = new_test_context_with_orderless_flags(
        current_function_name!(),
        use_txn_payload_v2_format,
        use_orderless_transactions,
    );
    let mut account = context.create_account().await;

    // Publish packages
    let named_addresses = vec![("abi".to_string(), account.address())];
    let txn = futures::executor::block_on(async move {
        let path = PathBuf::from(std::env!("CARGO_MANIFEST_DIR")).join("src/tests/move/pack_abi");
        TestContext::build_package(path, named_addresses)
    });
    context.publish_package(&mut account, txn).await;

    // Get abi.
    let modules = context
        .get(format!("/accounts/{}/modules", account.address(),).as_str())
        .await;

    let exposed_functions = modules.as_array().unwrap()[0]["abi"]["exposed_functions"]
        .as_array()
        .unwrap();

    let exposed_function_names: Vec<&str> = exposed_functions
        .iter()
        .map(|f| f["name"].as_str().unwrap())
        .collect();

    // All entry (including private entry) and public functions should be in the ABI.
    // Private (non-entry) functions should not be included.
    assert_eq!(exposed_function_names, [
        "private_entry_function",
        "public_entry_function",
        "public_function",
        "view_function",
    ]);

    // Confirm that the view function is reported as a view function.
    let view_function = exposed_functions
        .iter()
        .find(|f| f["name"].as_str().unwrap() == "view_function")
        .unwrap();

    assert_eq!(view_function["is_view"], true);

    // Confirm that the other functions are not reported as view functions.
    for name in [
        "private_entry_function",
        "public_entry_function",
        "public_function",
    ]
    .iter()
    {
        let function = exposed_functions
            .iter()
            .find(|f| &f["name"].as_str().unwrap() == name)
            .unwrap();

        assert_eq!(function["is_view"], false);
    }

    // Confirm that MyEvent is considered an event.
    let structs = modules.as_array().unwrap()[0]["abi"]["structs"]
        .as_array()
        .unwrap();
    let my_event = structs
        .iter()
        .find(|s| s["name"].as_str().unwrap() == "MyEvent")
        .unwrap();

    assert_eq!(my_event["is_event"], true);

    // Confirm that State is not considered an event.
    let my_struct = structs
        .iter()
        .find(|s| s["name"].as_str().unwrap() == "State")
        .unwrap();

    assert_eq!(my_struct["is_event"], false);

    // Confirm that MyEnum is considered an enum with correct variants.
    let my_enum = structs
        .iter()
        .find(|s| s["name"].as_str().unwrap() == "MyEnum")
        .unwrap();

    assert_eq!(my_enum["is_enum"], true);
    assert_eq!(my_enum["fields"].as_array().unwrap().len(), 0);

    let variants = my_enum["variants"].as_array().unwrap();
    assert_eq!(variants.len(), 2);
    assert_eq!(variants[0]["name"], "This");
    assert_eq!(variants[0]["fields"].as_array().unwrap().len(), 0);
    assert_eq!(variants[1]["name"], "That");
    assert_eq!(variants[1]["fields"].as_array().unwrap().len(), 1);
    assert_eq!(variants[1]["fields"][0]["name"], "value");
    assert_eq!(variants[1]["fields"][0]["type"], "u64");

    // Confirm that State is not considered an enum and has no variants.
    assert_eq!(my_struct["is_enum"], false);
    assert_eq!(my_struct["variants"].as_array().unwrap().len(), 0);

    let test_option = structs
        .iter()
        .find(|s| s["name"].as_str().unwrap() == "TestOption")
        .unwrap();
    assert_eq!(test_option["fields"][0]["name"], "o");

    let option_module = context.get("/accounts/0x1/module/option").await;

    let option_structs = option_module["abi"]["structs"].as_array().unwrap();

    assert_eq!(option_structs[0]["name"], "Option");
    assert_eq!(option_structs[0]["fields"][0]["name"], "vec");
    assert_eq!(option_structs[0]["is_enum"], false);
    assert_eq!(option_structs[0]["fields"][0]["type"], "vector<T0>");
    assert_eq!(option_structs[0]["variants"].as_array().unwrap().len(), 0);
}
