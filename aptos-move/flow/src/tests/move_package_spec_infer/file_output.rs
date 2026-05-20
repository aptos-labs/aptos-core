// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use crate::tests::common;
use std::fs;

/// File-mode inference from scratch (no existing .spec.move).
/// The tool should create a new .spec.move with full function signatures.
#[tokio::test]
async fn move_package_spec_infer_file_output() {
    let pkg = common::make_package("filegen", &[(
        "filegen",
        "module 0xCAFE::filegen {
    fun add_one(x: u64): u64 {
        x + 1
    }
}",
    )]);
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
    let source_path = pkg.path().join("sources/filegen.move");
    let source_after = fs::read_to_string(&source_path).expect("read source");

    // The spec file should be created.
    let spec_path = pkg.path().join("sources/filegen.spec.move");
    let spec_after = fs::read_to_string(&spec_path).expect("read spec file");

    let full_output = format!(
        "{}\n=== source (untouched) ===\n{}\n=== spec file ===\n{}",
        formatted, source_after, spec_after
    );
    common::check_baseline(file!(), &full_output);
}
