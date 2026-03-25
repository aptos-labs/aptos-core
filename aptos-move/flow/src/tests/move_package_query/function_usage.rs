// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use crate::tests::common;

#[tokio::test]
async fn move_package_query_function_usage() {
    let pkg = common::make_package("func_usage", &[
        (
            "math",
            "module 0xCAFE::math {
    public fun add(a: u64, b: u64): u64 { a + b }
    public fun double(x: u64): u64 { add(x, x) }
}",
        ),
        (
            "app",
            "module 0xCAFE::app {
    use 0xCAFE::math;
    public fun run(): u64 { math::double(21) }
}",
        ),
    ]);
    let dir = pkg.path().to_str().unwrap();
    let client = common::make_client().await;
    let result = common::call_tool(
        &client,
        "move_package_query",
        serde_json::json!({
            "package_path": dir,
            "query": "function_usage",
            "function": "app::run"
        }),
    )
    .await;
    let formatted = common::format_tool_result(&result);
    common::check_baseline(file!(), &formatted);
}
