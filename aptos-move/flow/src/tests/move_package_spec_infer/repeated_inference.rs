// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use crate::tests::common;
use std::fs;

#[tokio::test]
async fn repeated_wp_explains_how_to_refresh_generated_conditions() {
    let pkg = common::make_package("repeat_wp", &[(
        "repeat_wp",
        "module 0xCAFE::repeat_wp {
    public fun add_one(x: u64): u64 { x + 1 }
    public fun double(x: u64): u64 { x * 2 }
}",
    )]);
    let dir = pkg.path().to_str().unwrap();
    let client = common::make_client().await;
    let args = serde_json::json!({ "package_path": dir });

    let first = common::call_tool(&client, "move_package_wp", args.clone()).await;
    assert!(!first.is_error.unwrap_or_default());
    let source_path = pkg.path().join("sources/repeat_wp.move");
    let source_after_first = fs::read_to_string(&source_path).expect("read inferred source");

    let second = common::call_tool(&client, "move_package_wp", args).await;
    let formatted = common::format_tool_result(&second);
    let source_after_second = fs::read_to_string(&source_path).expect("read source after repeat");

    assert!(formatted.contains("WP made no changes"), "{formatted}");
    assert!(formatted.contains("2 function(s)"), "{formatted}");
    assert_eq!(formatted.matches("WP made no changes").count(), 1);
    assert!(
        formatted.contains("remove all `[inferred]` function conditions"),
        "{formatted}"
    );
    assert_eq!(source_after_first, source_after_second);
}
