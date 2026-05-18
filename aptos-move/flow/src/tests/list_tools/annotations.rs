// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use crate::tests::common;

/// Every MCP tool must declare `readOnlyHint` and `destructiveHint` annotations.
/// These are required for Anthropic marketplace submission.
#[tokio::test]
async fn all_tools_have_annotations() {
    let client = common::make_client().await;
    let result = client.list_tools(None).await.expect("list_tools");

    for tool in &result.tools {
        let name = &tool.name;
        let desc = tool
            .description
            .as_ref()
            .unwrap_or_else(|| panic!("{name}: missing description"));
        assert!(!desc.is_empty(), "{name}: empty description");
        let ann = tool
            .annotations
            .as_ref()
            .unwrap_or_else(|| panic!("{name}: missing annotations"));
        assert!(
            ann.read_only_hint.is_some(),
            "{name}: missing read_only_hint"
        );
        assert!(
            ann.destructive_hint.is_some(),
            "{name}: missing destructive_hint"
        );
    }
}
