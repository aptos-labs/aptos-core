// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use crate::tests::common;

const MIXED: &str = "module 0xCAFE::mixed {
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

    fun unlisted(x: u64): u64 {
        x + 1
    }
    spec unlisted {
        ensures result == x + 3;
    }
}";

const EXTRA: &str = "module 0xCAFE::extra {
    fun fine(x: u64): u64 {
        x
    }
    spec fine {
        ensures result == x;
    }
}";

async fn verify(arguments: serde_json::Value) -> String {
    let pkg = common::make_package("listed", &[("mixed", MIXED), ("extra", EXTRA)]);
    let mut arguments = arguments;
    arguments["package_path"] = serde_json::json!(pkg.path().to_str().unwrap());
    let client = common::make_client().await;
    match common::call_tool_raw(&client, "move_package_verify", arguments).await {
        Ok(result) => common::format_tool_result(&result),
        Err(error) => common::format_service_error(&error),
    }
}

/// The listed functions, across modules, are verified together: `bad` fails,
/// `unlisted` would fail too but is not verified.
#[tokio::test]
async fn move_package_verify_functions() {
    let formatted = verify(serde_json::json!({
        "functions": ["mixed::good", "mixed::bad", "extra::fine"]
    }))
    .await;
    common::check_baseline(file!(), &formatted);
}

#[tokio::test]
async fn move_package_verify_functions_and_filter_conflict() {
    let formatted = verify(serde_json::json!({
        "filter": "mixed::good",
        "functions": ["mixed::bad"]
    }))
    .await;
    assert!(
        formatted.contains("either `filter` or `functions`"),
        "unexpected result: {formatted}"
    );
}

#[tokio::test]
async fn move_package_verify_functions_rejects_a_module() {
    let formatted = verify(serde_json::json!({
        "functions": ["mixed::good", "extra"]
    }))
    .await;
    assert!(
        formatted.contains("does not name a function"),
        "unexpected result: {formatted}"
    );
}
