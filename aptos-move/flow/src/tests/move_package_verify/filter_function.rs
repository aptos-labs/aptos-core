// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use crate::tests::common;

/// Package has two functions: `good` with a correct spec and `bad` with a
/// wrong spec.  Filtering verification to `good` must succeed even though
/// `bad` would fail if verified.
#[tokio::test]
async fn move_package_verify_filter_function() {
    let pkg = common::make_package("mixed", &[(
        "mixed",
        "module 0xCAFE::mixed {
    fun good(x: u64): u64 {
        x + 1
    }
    spec good {
        ensures result == x + 1;
    }

    fun bad(x: u64): u64 {
        x + 1
    }
    spec bad {
        ensures result == x + 2;
    }
}",
    )]);
    let dir = pkg.path().to_str().unwrap();
    let client = common::make_client().await;
    let result = common::call_tool(
        &client,
        "move_package_verify",
        serde_json::json!({ "package_path": dir, "filter": "mixed::good" }),
    )
    .await;
    let formatted = common::format_tool_result(&result);
    common::check_baseline(file!(), &formatted);
}
