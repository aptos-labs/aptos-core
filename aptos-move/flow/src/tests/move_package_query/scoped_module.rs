// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use crate::tests::common;

#[tokio::test]
async fn move_package_query_scopes_large_queries_to_one_module() {
    let pkg = common::make_package("scoped_query", &[
        (
            "alpha",
            "module 0xCAFE::alpha {
    #[view]
    public fun alpha_value(): u64 { 1 }
}",
        ),
        (
            "beta",
            "module 0xCAFE::beta {
    public fun beta_value(): u64 { 2 }
}",
        ),
    ]);
    let dir = pkg.path().to_str().unwrap();
    let client = common::make_client().await;

    for query in ["module_summary", "facts"] {
        let result = common::call_tool(
            &client,
            "move_package_query",
            serde_json::json!({
                "package_path": dir,
                "query": query,
                "module": "alpha"
            }),
        )
        .await;
        let formatted = common::format_tool_result(&result);
        assert!(formatted.contains("alpha"), "{query}: {formatted}");
        assert!(!formatted.contains("beta_value"), "{query}: {formatted}");
    }
}

#[tokio::test]
async fn move_package_query_reports_unknown_module_scope() {
    let pkg = common::make_package("missing_scope", &[(
        "alpha",
        "module 0xCAFE::alpha { public fun value(): u64 { 1 } }",
    )]);
    let dir = pkg.path().to_str().unwrap();
    let client = common::make_client().await;
    let result = common::call_tool(
        &client,
        "move_package_query",
        serde_json::json!({
            "package_path": dir,
            "query": "facts",
            "module": "missing"
        }),
    )
    .await;
    let formatted = common::format_tool_result(&result);
    assert!(
        formatted.contains("module `missing` not found"),
        "{formatted}"
    );
}
