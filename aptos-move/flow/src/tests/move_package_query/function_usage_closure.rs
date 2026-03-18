// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use crate::tests::common;

/// Tests that "used" includes closure-captured functions while "called" does not.
#[tokio::test]
async fn move_package_query_function_usage_closure() {
    let pkg = common::make_package("closure", &[(
        "ops",
        "module 0xCAFE::ops {
    public fun add(a: u64, b: u64): u64 { a + b }
    public fun apply(f: |u64, u64| u64, x: u64, y: u64): u64 { f(x, y) }
    public fun run(): u64 { apply(add, 1, 2) }
}",
    )]);
    let dir = pkg.path().to_str().unwrap();
    let client = common::make_client().await;
    let result = common::call_tool(
        &client,
        "move_package_query",
        serde_json::json!({
            "package_path": dir,
            "query": { "function_usage": { "function": "ops::run" } }
        }),
    )
    .await;
    let formatted = common::format_tool_result(&result);
    common::check_baseline(file!(), &formatted);
}
