// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use crate::tests::common;
use std::fs;

#[tokio::test]
async fn qualified_filters_select_only_one_address() {
    for filter in [
        "0xCAFE::multi::add_one",
        "0x000cafe::multi::add_one",
        "named::multi::add_one",
    ] {
        let pkg = common::make_package("named", &[
            (
                "selected",
                "module 0xCAFE::multi {
                fun add_one(x: u64): u64 { x + 1 }
                fun other() {} spec other { ensures false; }
            }",
            ),
            (
                "unselected",
                "module 0xBEEF::multi {
                fun add_one(x: u64): u64 { x }
                spec add_one { ensures false; }
            }",
            ),
        ]);
        let client = common::make_client().await;
        let args = serde_json::json!({"package_path": pkg.path(), "filter": filter});
        let original = fs::read_to_string(pkg.path().join("sources/unselected.move")).unwrap();
        let inferred = common::call_tool(&client, "move_package_wp", args.clone()).await;
        assert_ne!(
            inferred.is_error,
            Some(true),
            "{}",
            common::format_tool_result(&inferred)
        );
        let selected = fs::read_to_string(pkg.path().join("sources/selected.move")).unwrap();
        assert!(selected.contains("spec add_one"), "{selected}");
        assert_eq!(
            original,
            fs::read_to_string(pkg.path().join("sources/unselected.move")).unwrap()
        );
        // Check scope selection independently of the file-watcher cache's
        // asynchronous invalidation after WP edits.
        let verify_client = common::make_client().await;
        let verified = common::call_tool(&verify_client, "move_package_verify", args).await;
        assert_eq!(
            common::format_tool_result(&verified).trim(),
            "verification succeeded"
        );
    }
}

#[tokio::test]
async fn qualified_filters_reject_ambiguous_and_missing_targets() {
    let pkg = common::make_package("named", &[
        ("first", "module 0xCAFE::multi { fun f() {} }"),
        ("second", "module 0xBEEF::multi { fun f() {} }"),
    ]);
    let client = common::make_client().await;
    for tool in ["move_package_wp", "move_package_verify"] {
        for (filter, expected) in [
            ("multi", "ambiguous"),
            ("multi::f", "ambiguous"),
            ("0xBAD::multi::f", "no module matching"),
            ("named::multi::missing", "no function matching"),
        ] {
            let error = common::call_tool_raw(
                &client,
                tool,
                serde_json::json!({"package_path": pkg.path(), "filter": filter}),
            )
            .await
            .expect_err("invalid filter must be rejected");
            assert!(error.to_string().contains(expected), "{error}");
        }
    }
    for file in ["first", "second"] {
        assert!(
            !fs::read_to_string(pkg.path().join(format!("sources/{file}.move")))
                .unwrap()
                .contains("spec ")
        );
    }
}

#[tokio::test]
async fn filtered_wp_does_not_report_unrelated_model_warnings() {
    let pkg = common::make_package("named", &[
        (
            "selected",
            "module 0xCAFE::selected {
                fun add_one(x: u64): u64 { x + 1 }
            }",
        ),
        (
            "unselected",
            "module 0xBEEF::unselected {
                fun noisy() { let unused = 1; }
            }",
        ),
    ]);
    let client = common::make_client().await;
    let result = common::call_tool(
        &client,
        "move_package_wp",
        serde_json::json!({
            "package_path": pkg.path(),
            "filter": "selected::add_one",
        }),
    )
    .await;
    let output = common::format_tool_result(&result);
    assert_ne!(result.is_error, Some(true), "{output}");
    assert!(!output.contains("unused"), "{output}");
    assert!(!output.contains("unselected"), "{output}");
}
