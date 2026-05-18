// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use crate::tests::common;

#[tokio::test]
async fn list_tools_success() {
    let client = common::make_client().await;
    let result = client.list_tools(None).await.expect("list_tools");
    let formatted = common::format_tools_list(&result);
    common::check_baseline(file!(), &formatted);
}
