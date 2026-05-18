// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use crate::tests::common;
use tempfile::TempDir;

#[tokio::test]
async fn move_replay_transaction_non_package_dir() {
    // A real directory that exists but has no Move.toml.
    let tmp = TempDir::new().unwrap();
    let path = tmp.path().to_string_lossy().into_owned();

    let client = common::make_client().await;
    let result = common::call_tool(
        &client,
        "move_replay_transaction",
        serde_json::json!({
            "txn_id": 1,
            "network": "testnet",
            "local_package_paths": [path],
        }),
    )
    .await;
    // Strip the temp path from the formatted output so the baseline is stable.
    let raw = common::format_tool_result(&result);
    let stable = raw.replace(tmp.path().to_string_lossy().as_ref(), "<TMPDIR>");
    common::check_baseline(file!(), &stable);
}
