// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use crate::tests::common;

/// Facts for nested closures: verifies `definedIn` resolves through a chain of
/// lambda-lifted functions to the outermost user function, not just one hop.
#[tokio::test]
async fn move_package_query_facts_nested() {
    let pkg = common::make_package("facts_nested", &[(
        "nested",
        "module 0xCAFE::nested {
    fun apply(f: |u64|u64, x: u64): u64 { f(x) }

    public fun outer(c: u64): u64 {
        apply(|x| apply(|y| x + y + c, x), 10)
    }
}",
    )]);
    let dir = pkg.path().to_str().unwrap();
    let client = common::make_client().await;
    let result = common::call_tool(
        &client,
        "move_package_query",
        serde_json::json!({ "package_path": dir, "query": "facts" }),
    )
    .await;
    let formatted = common::format_tool_result(&result);
    common::check_baseline(file!(), &formatted);
}
