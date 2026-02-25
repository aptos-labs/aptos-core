// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use crate::tests::common;

#[tokio::test]
async fn move_package_coverage_get_uncovered() {
    // Create package with two functions, only one tested
    let pkg = common::make_package("test_cov", &[(
        "example",
        r#"module 0xCAFE::example {
    public fun covered(): u64 { 1 }
    public fun uncovered(): u64 { 2 }

    #[test]
    fun test_covered() {
        assert!(covered() == 1, 0);
    }
}"#,
    )]);
    let dir = pkg.path().to_str().unwrap();
    let client = common::make_client().await;

    // Establish baseline first to clear any stale leftover coverage maps.
    let _ = common::call_tool(
        &client,
        "move_package_test",
        serde_json::json!({
            "package_path": dir,
            "establish_baseline": true
        }),
    )
    .await;

    // Get coverage - should show uncovered() as uncovered
    let result = common::call_tool(
        &client,
        "move_package_coverage",
        serde_json::json!({ "package_path": dir }),
    )
    .await;
    let formatted = common::format_tool_result(&result);
    common::check_baseline(file!(), &formatted);
}
