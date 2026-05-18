// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use crate::tests::common;

/// Coverage scoped to an uncovered function should only report lines from that function.
#[tokio::test]
async fn move_package_coverage_filter_uncovered() {
    let _guard = common::serial_test_lock().await;
    let pkg = common::make_package("test_cov_fn_filter", &[(
        "example",
        r#"module 0xCAFE::example {
    public fun covered(): u64 { 1 }
    public fun uncovered_a(): u64 { 2 }
    public fun uncovered_b(): u64 { 3 }

    #[test]
    fun test_covered() {
        assert!(covered() == 1, 0);
    }
}"#,
    )]);
    let dir = pkg.path().to_str().unwrap();
    let client = common::make_client().await;

    let _ = common::call_tool(
        &client,
        "move_package_test",
        serde_json::json!({
            "package_path": dir,
            "establish_baseline": true
        }),
    )
    .await;

    // Scoped to uncovered_a — should only show its line, not uncovered_b's.
    let result = common::call_tool(
        &client,
        "move_package_coverage",
        serde_json::json!({
            "package_path": dir,
            "function": "example::uncovered_a"
        }),
    )
    .await;
    let formatted = common::format_tool_result(&result);
    common::check_baseline(file!(), &formatted);
}
