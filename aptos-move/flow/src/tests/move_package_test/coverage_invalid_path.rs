// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use crate::tests::common;

#[tokio::test]
async fn move_package_coverage_invalid_path() {
    let client = common::make_client().await;
    let result = common::call_tool(
        &client,
        "move_package_coverage",
        serde_json::json!({ "package_path": "/nonexistent/path/to/package" }),
    )
    .await;
    let formatted = common::format_tool_result(&result);
    common::check_baseline(file!(), &formatted);
}
