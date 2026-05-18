// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use crate::tests::common;

#[tokio::test]
async fn move_package_verify_success() {
    let pkg = common::make_package("verified", &[(
        "verified",
        "module 0xCAFE::verified {
    fun double(x: u64): u64 {
        x * 2
    }
    spec double {
        ensures result == x * 2;
    }
}",
    )]);
    let dir = pkg.path().to_str().unwrap();
    let client = common::make_client().await;
    let result = common::call_tool(
        &client,
        "move_package_verify",
        serde_json::json!({ "package_path": dir }),
    )
    .await;
    let formatted = common::format_tool_result(&result);
    common::check_baseline(file!(), &formatted);
}
