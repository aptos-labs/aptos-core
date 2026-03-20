// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use crate::tests::common;
use std::fs;

/// Package has two functions: `add_one` and `double`.  Filtering inference to
/// `multi::add_one` must produce specs only for `add_one`; `double` should be
/// left unchanged.
#[tokio::test]
async fn move_package_spec_infer_filter_function() {
    let pkg = common::make_package("multi", &[(
        "multi",
        "module 0xCAFE::multi {
    fun add_one(x: u64): u64 {
        x + 1
    }

    fun double(x: u64): u64 {
        x * 2
    }
}",
    )]);
    let dir = pkg.path().to_str().unwrap();
    let client = common::make_client().await;
    let result = common::call_tool(
        &client,
        "move_package_spec_infer",
        serde_json::json!({ "package_path": dir, "filter": "multi::add_one" }),
    )
    .await;
    let formatted = common::format_tool_result(&result);

    // Also read the source file after inference to verify only add_one got specs.
    let source_path = pkg.path().join("sources/multi.move");
    let source_after = fs::read_to_string(&source_path).expect("read source after inference");
    let full_output = format!(
        "{}\n=== source after inference ===\n{}",
        formatted, source_after
    );
    common::check_baseline(file!(), &full_output);
}
