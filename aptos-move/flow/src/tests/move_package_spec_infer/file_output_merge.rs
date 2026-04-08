// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use crate::tests::common;
use std::fs;

/// File-mode inference with an existing .spec.move that contains a helper
/// function and a spec block for `add_one`. The tool must:
/// - Preserve the helper function
/// - Append inferred conditions to the existing `add_one` spec block
/// - Create a new spec block for `double` (which has no existing spec)
#[tokio::test]
async fn move_package_spec_infer_file_output_merge() {
    let pkg = common::make_package("mergetest", &[(
        "mergetest",
        "module 0xCAFE::mergetest {
    fun add_one(x: u64): u64 {
        x + 1
    }

    fun double(x: u64): u64 {
        x * 2
    }
}",
    )]);

    // Write an existing .spec.move with a helper and a partial spec for add_one.
    let spec_path = pkg.path().join("sources/mergetest.spec.move");
    fs::write(
        &spec_path,
        "spec 0xCAFE::mergetest {
    spec fun helper(x: u64): u64 {
        x + 1
    }

    spec add_one(x: u64): u64 {
        ensures result == helper(x);
    }
}
",
    )
    .expect("write spec file");

    let dir = pkg.path().to_str().unwrap();
    let client = common::make_client().await;
    let result = common::call_tool(
        &client,
        "move_package_wp",
        serde_json::json!({ "package_path": dir, "spec_output": "file" }),
    )
    .await;
    let formatted = common::format_tool_result(&result);

    // The original source should be untouched.
    let source_path = pkg.path().join("sources/mergetest.move");
    let source_after = fs::read_to_string(&source_path).expect("read source");

    // The spec file should be merged, not overwritten.
    let spec_after = fs::read_to_string(&spec_path).expect("read spec file");

    let full_output = format!(
        "{}\n=== source (untouched) ===\n{}\n=== spec file (merged) ===\n{}",
        formatted, source_after, spec_after
    );
    common::check_baseline(file!(), &full_output);
}
