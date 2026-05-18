// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use crate::tests::common;

#[tokio::test]
async fn move_package_query_module_summary() {
    let pkg = common::make_package("summary", &[(
        "token",
        "module 0xCAFE::token {
    const MAX_SUPPLY: u64 = 1000000;

    struct Token has key, store {
        value: u64,
        owner: address,
    }

    public fun mint(amount: u64, owner: address): Token {
        assert!(amount <= MAX_SUPPLY, 1);
        Token { value: amount, owner }
    }

    public entry fun burn(t: Token) {
        let Token { value: _, owner: _ } = t;
    }
}",
    )]);
    let dir = pkg.path().to_str().unwrap();
    let client = common::make_client().await;
    let result = common::call_tool(
        &client,
        "move_package_query",
        serde_json::json!({ "package_path": dir, "query": "module_summary" }),
    )
    .await;
    let formatted = common::format_tool_result(&result);
    common::check_baseline(file!(), &formatted);
}
