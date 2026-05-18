// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use crate::tests::common;

#[tokio::test]
async fn move_package_verify_filter_invalid_function() {
    let pkg = common::make_package("hello", &[(
        "hello",
        "module 0xCAFE::hello {
    public fun greet(): u64 { 42 }
}",
    )]);
    let dir = pkg.path().to_str().unwrap();
    let client = common::make_client().await;
    let result = common::call_tool_raw(
        &client,
        "move_package_verify",
        serde_json::json!({ "package_path": dir, "filter": "hello::nonexistent" }),
    )
    .await;
    let formatted = match result {
        Ok(tool_result) => common::format_tool_result(&tool_result),
        Err(err) => common::format_service_error(&err),
    };
    common::check_baseline(file!(), &formatted);
}
