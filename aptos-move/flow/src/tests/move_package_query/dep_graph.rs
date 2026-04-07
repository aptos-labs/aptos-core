// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use crate::tests::common;

#[tokio::test]
async fn move_package_query_dep_graph() {
    let pkg = common::make_package("dep_graph", &[
        (
            "alpha",
            "module 0xCAFE::alpha {
    use 0xCAFE::beta;
    public fun call_beta(): u64 { beta::value() }
}",
        ),
        (
            "beta",
            "module 0xCAFE::beta {
    public fun value(): u64 { 1 }
}",
        ),
    ]);
    let dir = pkg.path().to_str().unwrap();
    let client = common::make_client().await;
    let result = common::call_tool(
        &client,
        "move_package_query",
        serde_json::json!({ "package_path": dir, "query": "dep_graph" }),
    )
    .await;
    let formatted = common::format_tool_result(&result);
    common::check_baseline(file!(), &formatted);
}
