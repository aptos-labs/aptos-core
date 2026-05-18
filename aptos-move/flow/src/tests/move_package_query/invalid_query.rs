// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use crate::tests::common;

#[tokio::test]
async fn move_package_query_invalid_query() {
    let pkg = common::make_package("simple", &[(
        "hello",
        "module 0xCAFE::hello { public fun greet(): u64 { 42 } }",
    )]);
    let dir = pkg.path().to_str().unwrap();
    let client = common::make_client().await;
    let result = common::call_tool_raw(
        &client,
        "move_package_query",
        serde_json::json!({ "package_path": dir, "query": "nonsense" }),
    )
    .await;
    let formatted = common::format_service_error(&result.unwrap_err());
    common::check_baseline(file!(), &formatted);
}
