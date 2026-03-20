// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use crate::tests::common;

/// Package has two modules: `good_mod` with a correct spec and `bad_mod`
/// with a wrong spec.  Filtering verification to `good_mod` must succeed
/// even though `bad_mod` would fail if verified.
#[tokio::test]
async fn move_package_verify_filter_module() {
    let pkg = common::make_package("two_mods", &[
        (
            "good_mod",
            "module 0xCAFE::good_mod {
    fun inc(x: u64): u64 {
        x + 1
    }
    spec inc {
        ensures result == x + 1;
    }
}",
        ),
        (
            "bad_mod",
            "module 0xCAFE::bad_mod {
    fun dec(x: u64): u64 {
        x - 1
    }
    spec dec {
        ensures result == x + 1;
    }
}",
        ),
    ]);
    let dir = pkg.path().to_str().unwrap();
    let client = common::make_client().await;
    let result = common::call_tool(
        &client,
        "move_package_verify",
        serde_json::json!({ "package_path": dir, "filter": "good_mod" }),
    )
    .await;
    let formatted = common::format_tool_result(&result);
    common::check_baseline(file!(), &formatted);
}
