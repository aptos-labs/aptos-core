// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use crate::tests::common;

#[tokio::test]
async fn move_package_test_establish_baseline() {
    let pkg = common::make_package("test_baseline", &[(
        "example",
        r#"module 0xCAFE::example {
    public fun value(): u64 { 42 }

    #[test]
    fun test_value() {
        assert!(value() == 42, 0);
    }
}"#,
    )]);
    let dir = pkg.path().to_str().unwrap();
    let client = common::make_client().await;
    let result = common::call_tool(
        &client,
        "move_package_test",
        serde_json::json!({
            "package_path": dir,
            "establish_baseline": true
        }),
    )
    .await;
    let formatted = common::format_tool_result(&result);
    common::check_baseline(file!(), &formatted);
}
