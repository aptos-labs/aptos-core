// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use crate::tests::common;

/// Module-level filter combined with a function-level exclude *inside* the
/// filtered module is valid (verify all of A except one function). The
/// degenerate-scope check must compare full forms, not just module parts.
#[tokio::test]
async fn move_package_verify_filter_module_exclude_function() {
    let pkg = common::make_package("filter_mod_exclude_fn", &[(
        "good_mod",
        "module 0xCAFE::good_mod {
    fun inc(x: u64): u64 {
        x + 1
    }
    spec inc {
        ensures result == x + 1;
    }

    fun dec(x: u64): u64 {
        x - 1
    }
    spec dec {
        // intentionally wrong; this would fail if not excluded.
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
            "exclude": ["good_mod::dec"],
        }),
    )
    .await;
    let formatted = common::format_tool_result(&result);
    common::check_baseline(file!(), &formatted);
}
