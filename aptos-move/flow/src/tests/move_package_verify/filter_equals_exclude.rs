// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use crate::tests::common;

/// When `filter` and `exclude` together leave no verifiable function in
/// scope, the tool must report "nothing to verify" instead of returning a
/// vacuous "verification succeeded."
#[tokio::test]
async fn move_package_verify_filter_equals_exclude() {
    let pkg = common::make_package("filter_eq_exclude", &[(
        "good_mod",
        "module 0xCAFE::good_mod {
    fun inc(x: u64): u64 {
        x + 1
    }
    spec inc {
        ensures result == x + 1;
    }
}",
    )]);
    let dir = pkg.path().to_str().unwrap();
    let client = common::make_client().await;
    let result = common::call_tool(
        &client,
        "move_package_verify",
        serde_json::json!({
            "package_path": dir,
            "filter": "good_mod",
            "exclude": ["good_mod"],
        }),
    )
    .await;
    let formatted = common::format_tool_result(&result);
    common::check_baseline(file!(), &formatted);
}
