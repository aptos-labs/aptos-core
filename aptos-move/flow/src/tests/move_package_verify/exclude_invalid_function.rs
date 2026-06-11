// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use crate::tests::common;

/// `exclude` entries of the form `module::function` must validate the function
/// exists in the module. Otherwise a typo silently slips through — and worse,
/// when combined with `filter` it can leave an empty scope, causing the
/// anti-vacuity check to swallow the typo as "nothing to verify".
#[tokio::test]
async fn move_package_verify_exclude_invalid_function() {
    let pkg = common::make_package("exclude_invalid_fn", &[(
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
    let result = common::call_tool_raw(
        &client,
        "move_package_verify",
        serde_json::json!({
            "package_path": dir,
            "filter": "good_mod",
            "exclude": ["good_mod::inc", "good_mod::typo"],
        }),
    )
    .await;
    let formatted = match result {
        Ok(tool_result) => common::format_tool_result(&tool_result),
        Err(err) => common::format_service_error(&err),
    };
    common::check_baseline(file!(), &formatted);
}
