// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use crate::tests::common;
use std::fs;

#[tokio::test]
async fn move_package_test_newly_covered() {
    // Create package with two functions, only one tested initially
    let pkg = common::make_package("test_newly", &[(
        "example",
        r#"module 0xCAFE::example {
    public fun foo(): u64 { 1 }
    public fun bar(): u64 { 2 }

    #[test]
    fun test_foo() {
        assert!(foo() == 1, 0);
    }
}"#,
    )]);
    let dir = pkg.path().to_str().unwrap();
    let client = common::make_client().await;

    // First, establish baseline with only foo() covered
    let _ = common::call_tool(
        &client,
        "move_package_test",
        serde_json::json!({
            "package_path": dir,
            "establish_baseline": true
        }),
    )
    .await;

    // Now add a test for bar()
    let source_path = pkg.path().join("sources/example.move");
    let updated_source = r#"module 0xCAFE::example {
    public fun foo(): u64 { 1 }
    public fun bar(): u64 { 2 }

    #[test]
    fun test_foo() {
        assert!(foo() == 1, 0);
    }

    #[test]
    fun test_bar() {
        assert!(bar() == 2, 0);
    }
}"#;
    fs::write(&source_path, updated_source).expect("write updated source");

    // Run tests again - should show bar() as newly covered
    let result = common::call_tool(
        &client,
        "move_package_test",
        serde_json::json!({ "package_path": dir }),
    )
    .await;
    let formatted = common::format_tool_result(&result);
    common::check_baseline(file!(), &formatted);
}
