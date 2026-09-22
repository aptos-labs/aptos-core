// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use crate::{
    evaluation::{FeedbackLevel, InferenceTactic},
    tests::common,
};

/// A loop without an invariant makes the prover havoc the mutated state, so the
/// postcondition is unprovable. The evidence names what the loop does on its
/// first iterations.
const LOOP_PACKAGE: &str = "module 0xCAFE::loop_sum {
    fun sum_to(n: u64): u64 {
        let i = 0;
        let s = 0;
        while (i < n) {
            s = s + i;
            i = i + 1;
        };
        s
    }
    spec sum_to {
        ensures result == n * (n - 1) / 2;
    }
}";

#[tokio::test]
async fn move_package_verify_loop_invariant_evidence() {
    let pkg = common::make_package("loop_sum", &[("loop_sum", LOOP_PACKAGE)]);
    let dir = pkg.path().to_str().unwrap();
    let client =
        common::make_client_at_level(InferenceTactic::HybridGuided, FeedbackLevel::Diagnostics)
            .await;
    let result = common::call_tool(
        &client,
        "move_package_verify",
        serde_json::json!({ "package_path": dir }),
    )
    .await;
    let formatted = common::format_tool_result(&result);
    common::check_baseline(file!(), &formatted);
}

/// The same package at `acceptance`: the evidence reaches every feedback
/// level.
#[tokio::test]
async fn move_package_verify_evidence_is_not_gated_by_feedback_level() {
    let pkg = common::make_package("loop_sum", &[("loop_sum", LOOP_PACKAGE)]);
    let dir = pkg.path().to_str().unwrap();
    let client =
        common::make_client_at_level(InferenceTactic::HybridGuided, FeedbackLevel::Acceptance)
            .await;
    let result = common::call_tool(
        &client,
        "move_package_verify",
        serde_json::json!({ "package_path": dir }),
    )
    .await;
    let formatted = common::format_tool_result(&result);
    assert!(
        formatted.contains("loop-invariant evidence"),
        "evidence must reach every feedback level:\n{formatted}"
    );
}
