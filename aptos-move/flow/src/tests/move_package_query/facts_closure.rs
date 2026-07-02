// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use crate::tests::common;

/// Facts for lambda-lifted functions: verifies `isLambdaLifted`/`definedIn` tagging,
/// and that `acquiresInferred` (host) and `resourceAccess` (lifted body) differ.
#[tokio::test]
async fn move_package_query_facts_closure() {
    let pkg = common::make_package("facts_closure", &[(
        "closures",
        "module 0xCAFE::closures {
    struct Config has key { v: u64 }

    fun apply(f: |u64|u64, x: u64): u64 { f(x) }
    fun run(f: ||u64): u64 { f() }

    public fun with_capture(c: u64): u64 {
        apply(|y| y + c, 10)
    }

    public fun setup(): u64 {
        run(|| borrow_global<Config>(@0xCAFE).v)
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
