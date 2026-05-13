// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use crate::tests::common;

#[tokio::test]
async fn move_replay_transaction_missing_local_package() {
    let client = common::make_client().await;
    let result = common::call_tool(
        &client,
        "move_replay_transaction",
        serde_json::json!({
            "txn_id": 1,
            "network": "testnet",
            "local_package_paths": ["/no/such/path/12345"],
        }),
    )
    .await;
    let formatted = common::format_tool_result(&result);
    common::check_baseline(file!(), &formatted);
}
