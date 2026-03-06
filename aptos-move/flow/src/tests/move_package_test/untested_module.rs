// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use crate::tests::common;

/// A module with no tests should have all its executable lines reported as
/// uncovered (not silently omitted from the coverage output).
#[tokio::test]
async fn move_package_coverage_untested_module() {
    let pkg = common::make_package("test_cov_untested", &[
        (
            "tested",
            r#"module 0xCAFE::tested {
    public fun add(a: u64, b: u64): u64 { a + b }

    #[test]
    fun test_add() {
        assert!(add(2, 3) == 5, 0);
    }
}"#,
        ),
        (
            "untested",
            r#"module 0xCAFE::untested {
    public fun multiply(a: u64, b: u64): u64 { a * b }
    public fun subtract(a: u64, b: u64): u64 {
        assert!(a >= b, 1);
        a - b
    }
}"#,
        ),
    ]);
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

    let result = common::call_tool(
        &client,
        "move_package_coverage",
        serde_json::json!({ "package_path": dir }),
    )
    .await;
    let formatted = common::format_tool_result(&result);
    common::check_baseline(file!(), &formatted);
}
