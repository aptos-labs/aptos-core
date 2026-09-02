// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use crate::{
    evaluation::{FeedbackLevel, InferenceTactic},
    tests::common,
};
use serde_json::json;
use std::{fs, path::Path};

const BASELINE: &str = "module 0xCAFE::pure { fun answer(): u64 { 42 } }\n";
const COMPLETE: &str = "module 0xCAFE::pure {\n    fun answer(): u64 { 42 }\n    \
                        spec answer {\n        aborts_if false;\n        \
                        ensures result == 42;\n    }\n}\n";
const WEAKENED: &str = "module 0xCAFE::pure {\n    fun answer(): u64 { 42 }\n    \
                        spec answer {\n        aborts_if true;\n        \
                        ensures result == 42;\n    }\n}\n";

/// Verifies, but lacks the `abort` category the task requires.
const ENSURES_ONLY: &str = "module 0xCAFE::pure {\n    fun answer(): u64 { 42 }\n    \
                            spec answer {\n        ensures result == 42;\n    }\n}\n";

fn write_config(path: &Path, baseline: &Path, package: &Path) {
    fs::write(
        path,
        serde_json::to_string(&json!({
            "schema_version": 1,
            "baseline": baseline,
            "package": package,
            "target": "0xCAFE::pure::answer",
            "allowed_edit_paths": ["sources/**/*.move"],
            "required_contract_categories": ["normal-result", "abort"],
            "timeout_seconds": 40,
        }))
        .unwrap(),
    )
    .unwrap();
}

async fn check(candidate_source: &str) -> String {
    let baseline = common::make_package("pure", &[("pure.move", BASELINE)]);
    let package = common::make_package("pure", &[("pure.move", BASELINE)]);
    fs::write(
        package.path().join("sources").join("pure.move"),
        candidate_source,
    )
    .unwrap();
    // The configuration lives outside both trees; a file inside either would
    // itself count as an out-of-scope workspace change.
    let criteria = tempfile::TempDir::new().unwrap();
    let config = criteria.path().join("candidate-check.json");
    write_config(&config, baseline.path(), package.path());

    let client = common::make_evaluation_client(InferenceTactic::AgentOnly, &config).await;
    let result = common::call_tool(
        &client,
        "move_spec_check",
        json!({ "package_path": package.path() }),
    )
    .await;
    common::format_tool_result(&result)
}

#[tokio::test]
async fn a_complete_verified_contract_is_accepted() {
    let output = check(COMPLETE).await;
    assert!(
        output.contains("CANDIDATE_ACCEPTED"),
        "unexpected verdict: {output}"
    );
}

#[tokio::test]
async fn an_unconditional_abort_is_rejected() {
    let output = check(WEAKENED).await;
    assert!(
        output.contains("CANDIDATE_REJECTED") && output.contains("unconditional_abort"),
        "unexpected verdict: {output}"
    );
}

#[tokio::test]
async fn the_check_runs_without_an_evaluation_session() {
    // Without a task configuration the criteria come from the package itself.
    let package = common::make_package("standalone", &[(
        "m",
        "module 0xCAFE::m {
    fun double(x: u64): u64 { x * 2 }
    spec double {
        aborts_if x * 2 > MAX_U64;
        ensures result == x * 2;
    }
}",
    )]);
    let client = common::make_client_for_tactic(InferenceTactic::AgentOnly).await;
    let result = common::call_tool(
        &client,
        "move_spec_check",
        json!({ "package_path": package.path().to_str().unwrap(), "filter": "m" }),
    )
    .await;
    let formatted = common::format_tool_result(&result);
    assert!(
        formatted.contains("CANDIDATE_ACCEPTED"),
        "expected acceptance outside an evaluation session:\n{formatted}"
    );
}

#[tokio::test]
async fn the_progress_level_reports_what_moved_between_attempts() {
    // A delta compares two attempts, so the first check reports none and the
    // second names the obligation that started to verify.
    let package = common::make_package("progress", &[(
        "m",
        "module 0xCAFE::m {
    fun double(x: u64): u64 { x * 2 }
    spec double {
        aborts_if x * 2 > MAX_U64;
        ensures result == x;
    }
}",
    )]);
    let client =
        common::make_client_at_level(InferenceTactic::AgentOnly, FeedbackLevel::Progress).await;
    let arguments = json!({ "package_path": package.path().to_str().unwrap(), "filter": "m" });

    let first = common::format_tool_result(
        &common::call_tool(&client, "move_spec_check", arguments.clone()).await,
    );
    assert!(
        !first.contains("Progress:"),
        "a first attempt has nothing to compare against:\n{first}"
    );

    fs::write(
        package.path().join("sources").join("m.move"),
        "module 0xCAFE::m {
    fun double(x: u64): u64 { x * 2 }
    spec double {
        aborts_if x * 2 > MAX_U64;
        ensures result == x * 2;
    }
}",
    )
    .unwrap();
    let second =
        common::format_tool_result(&common::call_tool(&client, "move_spec_check", arguments).await);

    assert!(
        second.contains("Progress: verified"),
        "expected a progress report on the second attempt:\n{second}"
    );
    assert!(
        second.contains("Now verified"),
        "the repaired postcondition should be named:\n{second}"
    );
}

#[tokio::test]
async fn an_omitted_filter_checks_every_target_module() {
    // The documented default: omit `filter` and every target module is checked.
    // An omitted filter must stay `None` all the way down, because an empty
    // filter string matches no module and would select nothing at all.
    let package = common::make_package("wide", &[
        (
            "a",
            "module 0xCAFE::a {
    fun double(x: u64): u64 { x * 2 }
    spec double {
        aborts_if x * 2 > MAX_U64;
        ensures result == x * 2;
    }
}",
        ),
        (
            "b",
            "module 0xCAFE::b {
    fun triple(x: u64): u64 { x * 3 }
    spec triple {
        aborts_if x * 3 > MAX_U64;
        ensures result == x * 3;
    }
}",
        ),
    ]);
    let client = common::make_client_for_tactic(InferenceTactic::AgentOnly).await;
    let result = common::call_tool(
        &client,
        "move_spec_check",
        json!({ "package_path": package.path().to_str().unwrap() }),
    )
    .await;
    let formatted = common::format_tool_result(&result);
    assert!(
        formatted.contains("CANDIDATE_ACCEPTED"),
        "an omitted filter should check the whole package, not select nothing:\n{formatted}"
    );
}

#[tokio::test]
async fn an_unsatisfiable_precondition_is_not_acceptance() {
    // `requires false` discharges every obligation in the body, so a candidate
    // could otherwise pass with an outright false postcondition.
    let package = common::make_package("vacuous", &[(
        "m",
        "module 0xCAFE::m {
    fun answer(): u64 { 42 }
    spec answer {
        requires false;
        aborts_if false;
        ensures result == 7;
    }
}",
    )]);
    let client = common::make_client_for_tactic(InferenceTactic::AgentOnly).await;
    let result = common::call_tool(
        &client,
        "move_spec_check",
        json!({ "package_path": package.path().to_str().unwrap(), "filter": "m" }),
    )
    .await;
    let formatted = common::format_tool_result(&result);
    assert!(
        !formatted.contains("CANDIDATE_ACCEPTED"),
        "a vacuous contract must not be accepted:\n{formatted}"
    );
}

#[tokio::test]
async fn a_baseline_cell_is_not_judged_by_the_task_criteria() {
    // The required categories are the acceptance intervention. Above the
    // baseline level a candidate missing one is rejected; at baseline the same
    // configuration is withheld and the package's own defaults apply, so the
    // control never receives the treatment's feedback.
    for (level, accepted) in [
        (FeedbackLevel::Acceptance, false),
        (FeedbackLevel::Baseline, true),
    ] {
        let baseline = common::make_package("pure", &[("pure.move", BASELINE)]);
        let package = common::make_package("pure", &[("pure.move", BASELINE)]);
        fs::write(
            package.path().join("sources").join("pure.move"),
            ENSURES_ONLY,
        )
        .unwrap();
        let criteria = tempfile::TempDir::new().unwrap();
        let config = criteria.path().join("candidate-check.json");
        write_config(&config, baseline.path(), package.path());

        let client =
            common::make_evaluation_client_at_level(InferenceTactic::AgentOnly, &config, level)
                .await;
        let result = common::call_tool(
            &client,
            "move_spec_check",
            json!({ "package_path": package.path() }),
        )
        .await;
        let output = common::format_tool_result(&result);
        assert_eq!(
            output.contains("CANDIDATE_ACCEPTED"),
            accepted,
            "at {level:?} the task criteria should {}apply:\n{output}",
            if accepted { "not " } else { "" }
        );
    }
}

#[tokio::test]
async fn an_enforcing_task_rejects_a_changed_implementation() {
    // A task configuration enforces the baseline comparison: a candidate that
    // rewrote the function to make its own postcondition true has not
    // specified the function it was given, and is rejected rather than
    // accepted with a warning.
    let baseline = common::make_package("pure", &[("pure.move", BASELINE)]);
    let package = common::make_package("pure", &[("pure.move", BASELINE)]);
    fs::write(
        package.path().join("sources").join("pure.move"),
        "module 0xCAFE::pure {\n    fun answer(): u64 { 0 }\n    \
         spec answer {\n        aborts_if false;\n        ensures result == 0;\n    }\n}\n",
    )
    .unwrap();
    let criteria = tempfile::TempDir::new().unwrap();
    let config = criteria.path().join("candidate-check.json");
    fs::write(
        &config,
        serde_json::to_string(&json!({
            "schema_version": 1,
            "baseline": baseline.path(),
            "package": package.path(),
            "target": "0xCAFE::pure::answer",
            "allowed_edit_paths": ["sources/**/*.move"],
            "required_contract_categories": ["normal-result", "abort"],
            "timeout_seconds": 40,
            "enforce_edit_policy": true,
        }))
        .unwrap(),
    )
    .unwrap();
    let client = common::make_evaluation_client(InferenceTactic::AgentOnly, &config).await;
    let result = common::call_tool(
        &client,
        "move_spec_check",
        json!({ "package_path": package.path() }),
    )
    .await;
    let output = common::format_tool_result(&result);
    assert!(
        output.contains("CANDIDATE_REJECTED") && output.contains("the implementation changed"),
        "a changed implementation must be rejected under an enforcing task:\n{output}"
    );
}

#[tokio::test]
async fn an_inline_assume_is_rejected() {
    // `assume` narrows what is verified without declaring a precondition: this
    // contract proves only because `x < 10` rules out the overflow, and the
    // block leaves the bytecode untouched, so nothing else would notice.
    let package = common::make_package("assume", &[(
        "m",
        "module 0xCAFE::m {
    fun double(x: u64): u64 {
        spec { assume x < 10; };
        x * 2
    }
    spec double {
        aborts_if false;
        ensures result == x * 2;
    }
}",
    )]);
    let client = common::make_client_for_tactic(InferenceTactic::AgentOnly).await;
    let result = common::call_tool(
        &client,
        "move_spec_check",
        json!({ "package_path": package.path().to_str().unwrap(), "filter": "m" }),
    )
    .await;
    let formatted = common::format_tool_result(&result);
    assert!(
        formatted.contains("CANDIDATE_REJECTED") && formatted.contains("unjustified_assumption"),
        "an inline assume must be rejected:\n{formatted}"
    );
}

async fn rejected_with(source: &str, code: &str) -> String {
    let package = common::make_package("weak", &[("m", source)]);
    let client = common::make_client_for_tactic(InferenceTactic::AgentOnly).await;
    let result = common::call_tool(
        &client,
        "move_spec_check",
        json!({ "package_path": package.path().to_str().unwrap(), "filter": "m" }),
    )
    .await;
    let formatted = common::format_tool_result(&result);
    assert!(
        formatted.contains("CANDIDATE_REJECTED") && formatted.contains(code),
        "expected rejection `{code}`:\n{formatted}"
    );
    formatted
}

#[tokio::test]
async fn an_abstract_condition_is_rejected() {
    // Applied at call sites, never verified against the body: this false
    // postcondition would otherwise count as normal-result coverage.
    rejected_with(
        "module 0xCAFE::m {
    fun answer(): u64 { 42 }
    spec answer {
        aborts_if false;
        ensures [abstract] result == 7;
    }
}",
        "abstract_condition",
    )
    .await;
}

#[tokio::test]
async fn an_added_intrinsic_pragma_is_rejected() {
    // An intrinsic function is never verified. The corpus has legitimate
    // intrinsics, so only opacity or intrinsicness the candidate adds counts.
    let baseline = common::make_package("pure", &[("pure.move", BASELINE)]);
    let package = common::make_package("pure", &[("pure.move", BASELINE)]);
    fs::write(
        package.path().join("sources").join("pure.move"),
        "module 0xCAFE::pure {\n    fun answer(): u64 { 42 }\n    \
         spec answer {\n        pragma intrinsic = true;\n        aborts_if false;\n        \
         ensures result == 7;\n    }\n}\n",
    )
    .unwrap();
    let criteria = tempfile::TempDir::new().unwrap();
    let config = criteria.path().join("candidate-check.json");
    write_config(&config, baseline.path(), package.path());
    let client = common::make_evaluation_client(InferenceTactic::AgentOnly, &config).await;
    let result = common::call_tool(
        &client,
        "move_spec_check",
        json!({ "package_path": package.path() }),
    )
    .await;
    let output = common::format_tool_result(&result);
    assert!(
        output.contains("CANDIDATE_REJECTED") && output.contains("added_intrinsic"),
        "an added intrinsic pragma must be rejected:\n{output}"
    );
}

#[tokio::test]
async fn a_changed_opaque_contract_is_rejected() {
    // The helper was already opaque, with a true contract. Rewriting that
    // contract is not verified here -- only the target is -- yet the target
    // proves against it.
    let source = "module 0xCAFE::pure {\n    fun helper(x: u64): u64 { x }\n    \
                  spec helper {\n        pragma opaque;\n        aborts_if false;\n        \
                  ensures result == x;\n    }\n    fun answer(): u64 { helper(42) }\n}\n";
    let baseline = common::make_package("pure", &[("pure.move", source)]);
    let package = common::make_package("pure", &[("pure.move", source)]);
    fs::write(
        package.path().join("sources").join("pure.move"),
        "module 0xCAFE::pure {\n    fun helper(x: u64): u64 { x }\n    \
         spec helper {\n        pragma opaque;\n        aborts_if false;\n        \
         ensures result == 7;\n    }\n    fun answer(): u64 { helper(42) }\n    \
         spec answer {\n        aborts_if false;\n        ensures result == 7;\n    }\n}\n",
    )
    .unwrap();
    let criteria = tempfile::TempDir::new().unwrap();
    let config = criteria.path().join("candidate-check.json");
    write_config(&config, baseline.path(), package.path());
    let client = common::make_evaluation_client(InferenceTactic::AgentOnly, &config).await;
    let result = common::call_tool(
        &client,
        "move_spec_check",
        json!({ "package_path": package.path() }),
    )
    .await;
    let output = common::format_tool_result(&result);
    assert!(
        output.contains("CANDIDATE_REJECTED") && output.contains("changed_opaque_contract"),
        "a rewritten opaque contract must be rejected:\n{output}"
    );
}

#[tokio::test]
async fn an_added_opaque_helper_is_rejected() {
    // The target proves against the helper's contract, which is assumed at
    // the call site and, being out of scope, never verified: `answer` would
    // otherwise be accepted with `ensures result == 7` on a function that
    // returns 42.
    let source = "module 0xCAFE::pure {\n    fun helper(x: u64): u64 { x }\n    \
                  fun answer(): u64 { helper(42) }\n}\n";
    let baseline = common::make_package("pure", &[("pure.move", source)]);
    let package = common::make_package("pure", &[("pure.move", source)]);
    fs::write(
        package.path().join("sources").join("pure.move"),
        "module 0xCAFE::pure {\n    fun helper(x: u64): u64 { x }\n    \
         spec helper {\n        pragma opaque;\n        aborts_if false;\n        \
         ensures result == 7;\n    }\n    fun answer(): u64 { helper(42) }\n    \
         spec answer {\n        aborts_if false;\n        ensures result == 7;\n    }\n}\n",
    )
    .unwrap();
    let criteria = tempfile::TempDir::new().unwrap();
    let config = criteria.path().join("candidate-check.json");
    write_config(&config, baseline.path(), package.path());
    let client = common::make_evaluation_client(InferenceTactic::AgentOnly, &config).await;
    let result = common::call_tool(
        &client,
        "move_spec_check",
        json!({ "package_path": package.path() }),
    )
    .await;
    let output = common::format_tool_result(&result);
    assert!(
        output.contains("CANDIDATE_REJECTED") && output.contains("added_opaque"),
        "an added opaque helper must be rejected:\n{output}"
    );
}

#[tokio::test]
async fn a_redefined_spec_function_is_rejected() {
    // The helper's opaque contract is unchanged, but the spec function it
    // references is redefined, so the target proves `result == 7` against a
    // helper that returns 42.
    let source = "module 0xCAFE::pure {\n    spec fun expected(r: u64, x: u64): bool { r == x }\n    \
                  fun helper(x: u64): u64 { x }\n    spec helper {\n        pragma opaque;\n        \
                  aborts_if false;\n        ensures expected(result, x);\n    }\n    \
                  fun answer(): u64 { helper(42) }\n}\n";
    let baseline = common::make_package("pure", &[("pure.move", source)]);
    let package = common::make_package("pure", &[("pure.move", source)]);
    fs::write(
        package.path().join("sources").join("pure.move"),
        "module 0xCAFE::pure {\n    spec fun expected(r: u64, x: u64): bool { r == 7 }\n    \
         fun helper(x: u64): u64 { x }\n    spec helper {\n        pragma opaque;\n        \
         aborts_if false;\n        ensures expected(result, x);\n    }\n    \
         fun answer(): u64 { helper(42) }\n    spec answer {\n        aborts_if false;\n        \
         ensures result == 7;\n    }\n}\n",
    )
    .unwrap();
    let criteria = tempfile::TempDir::new().unwrap();
    let config = criteria.path().join("candidate-check.json");
    write_config(&config, baseline.path(), package.path());
    let client = common::make_evaluation_client(InferenceTactic::AgentOnly, &config).await;
    let result = common::call_tool(
        &client,
        "move_spec_check",
        json!({ "package_path": package.path() }),
    )
    .await;
    let output = common::format_tool_result(&result);
    assert!(
        output.contains("CANDIDATE_REJECTED") && output.contains("changed_spec_function"),
        "a redefined spec function must be rejected:\n{output}"
    );
}

#[tokio::test]
async fn an_assume_outside_the_target_scope_is_rejected() {
    // The prover inlines a transparent callee, so an `assume` in a helper the
    // filter does not select still constrains the target's proof. Without this
    // the target's false postcondition would verify.
    let source = "module 0xCAFE::pure {\n    fun helper(x: u64): u64 { x }\n    \
                  fun answer(): u64 { helper(42) }\n}\n";
    let baseline = common::make_package("pure", &[("pure.move", source)]);
    let package = common::make_package("pure", &[("pure.move", source)]);
    fs::write(
        package.path().join("sources").join("pure.move"),
        "module 0xCAFE::pure {\n    fun helper(x: u64): u64 {\n        \
         spec { assume x == 7; };\n        x\n    }\n    \
         fun answer(): u64 { helper(42) }\n    \
         spec answer {\n        aborts_if false;\n        ensures result == 7;\n    }\n}\n",
    )
    .unwrap();
    let criteria = tempfile::TempDir::new().unwrap();
    let config = criteria.path().join("candidate-check.json");
    write_config(&config, baseline.path(), package.path());
    let client = common::make_evaluation_client(InferenceTactic::AgentOnly, &config).await;
    let result = common::call_tool(
        &client,
        "move_spec_check",
        json!({ "package_path": package.path() }),
    )
    .await;
    let output = common::format_tool_result(&result);
    assert!(
        output.contains("CANDIDATE_REJECTED") && output.contains("unjustified_assumption"),
        "an out-of-scope assume must be rejected:\n{output}"
    );
}

#[tokio::test]
async fn a_vacuous_inferred_condition_is_rejected() {
    // The inference engine marks a clause it derived from unconstrained havoc
    // `vacuous`; by its own definition it carries no information, so it must
    // not satisfy a required category.
    rejected_with(
        "module 0xCAFE::m {
    fun answer(): u64 { 42 }
    spec answer {
        aborts_if false;
        ensures [inferred = vacuous] result == 42;
    }
}",
        "vacuous_inferred_condition",
    )
    .await;
}

#[tokio::test]
async fn a_weakened_opaque_contract_is_rejected() {
    // The helper's conditions are untouched, but `aborts_if_is_partial` turns
    // its `aborts_if` into a lower bound: a weaker promise at every call site.
    let source = "module 0xCAFE::pure {\n    fun helper(x: u64): u64 { x }\n    \
                  spec helper {\n        pragma opaque;\n        aborts_if false;\n        \
                  ensures result == x;\n    }\n    fun answer(): u64 { helper(42) }\n}\n";
    let baseline = common::make_package("pure", &[("pure.move", source)]);
    let package = common::make_package("pure", &[("pure.move", source)]);
    fs::write(
        package.path().join("sources").join("pure.move"),
        "module 0xCAFE::pure {\n    fun helper(x: u64): u64 { x }\n    \
         spec helper {\n        pragma opaque;\n        pragma aborts_if_is_partial;\n        \
         aborts_if false;\n        ensures result == x;\n    }\n    \
         fun answer(): u64 { helper(42) }\n    \
         spec answer {\n        aborts_if false;\n        ensures result == 42;\n    }\n}\n",
    )
    .unwrap();
    let criteria = tempfile::TempDir::new().unwrap();
    let config = criteria.path().join("candidate-check.json");
    write_config(&config, baseline.path(), package.path());
    let client = common::make_evaluation_client(InferenceTactic::AgentOnly, &config).await;
    let result = common::call_tool(
        &client,
        "move_spec_check",
        json!({ "package_path": package.path() }),
    )
    .await;
    let output = common::format_tool_result(&result);
    assert!(
        output.contains("CANDIDATE_REJECTED") && output.contains("changed_opaque_contract"),
        "a weakened opaque contract must be rejected:\n{output}"
    );
}

/// Baseline and candidate of an opaque helper, differing only in `change`.
async fn opaque_helper_change(helper_spec: &str, change: &str) -> String {
    let module = |spec: &str| {
        format!(
            "module 0xCAFE::pure {{\n    fun helper(x: u64): u64 {{ x }}\n    \
             spec helper {{\n        pragma opaque;\n{spec}    }}\n    \
             fun answer(): u64 {{ helper(42) }}\n}}\n"
        )
    };
    let baseline = common::make_package("pure", &[("pure.move", &module(helper_spec))]);
    let package = common::make_package("pure", &[("pure.move", &module(helper_spec))]);
    fs::write(
        package.path().join("sources").join("pure.move"),
        module(change),
    )
    .unwrap();
    let criteria = tempfile::TempDir::new().unwrap();
    let config = criteria.path().join("candidate-check.json");
    write_config(&config, baseline.path(), package.path());
    let client = common::make_evaluation_client(InferenceTactic::AgentOnly, &config).await;
    let result = common::call_tool(
        &client,
        "move_spec_check",
        json!({ "package_path": package.path() }),
    )
    .await;
    let output = common::format_tool_result(&result);
    assert!(
        !output.contains("does not compile"),
        "the fixture must compile, or the test proves nothing:\n{output}"
    );
    output
}

#[tokio::test]
async fn a_changed_abort_code_of_an_opaque_helper_is_rejected() {
    // `aborts_if P with CODE` keeps the code beside the condition, so a
    // fingerprint over the condition alone would miss a changed promise.
    let output = opaque_helper_change(
        "        aborts_if x == 0 with 1;\n        ensures result == x;\n",
        "        aborts_if x == 0 with 7;\n        ensures result == x;\n",
    )
    .await;
    assert!(
        output.contains("changed_opaque_contract"),
        "a changed abort code must be rejected:\n{output}"
    );
}

#[tokio::test]
async fn an_added_strict_abort_pragma_is_rejected() {
    // Under `aborts_if_is_strict` a helper with no `aborts_if` promises never
    // to abort. That is a new promise, trusted at every call site.
    let output = opaque_helper_change(
        "        ensures result == x;\n",
        "        pragma aborts_if_is_strict;\n        ensures result == x;\n",
    )
    .await;
    assert!(
        output.contains("changed_opaque_contract"),
        "an added strict-abort pragma must be rejected:\n{output}"
    );
}

#[tokio::test]
async fn a_lemma_does_not_cover_a_required_category() {
    // A lemma states an auxiliary fact for the proof, not the behaviour of
    // executable code. Under a module-level task it is in scope, and its
    // `ensures` must not satisfy the category on behalf of the function that
    // was left unspecified.
    let package = common::make_package("lemma", &[(
        "pure",
        "module 0xCAFE::pure {
    fun answer(): u64 { 42 }
    spec module {
        lemma trivial(x: u64) {
            ensures x == x;
        }
    }
}",
    )]);
    let criteria = tempfile::TempDir::new().unwrap();
    let config = criteria.path().join("candidate-check.json");
    fs::write(
        &config,
        serde_json::to_string(&json!({
            "schema_version": 1,
            "package": package.path(),
            "target": "0xCAFE::pure",
            "allowed_edit_paths": ["sources/**/*.move"],
            "required_contract_categories": ["normal-result"],
            "timeout_seconds": 40,
        }))
        .unwrap(),
    )
    .unwrap();
    let client = common::make_evaluation_client(InferenceTactic::AgentOnly, &config).await;
    let result = common::call_tool(
        &client,
        "move_spec_check",
        json!({ "package_path": package.path() }),
    )
    .await;
    let output = common::format_tool_result(&result);
    assert!(
        !output.contains("does not compile"),
        "the fixture must compile, or the test proves nothing:\n{output}"
    );
    assert!(
        !output.contains("CANDIDATE_ACCEPTED"),
        "a lemma must not stand in for the target's contract:\n{output}"
    );
}

#[tokio::test]
async fn an_obligation_suppressing_pragma_is_rejected() {
    // `addition_overflow_unchecked` removes the overflow abort the code really
    // has, so a contract that omits it would otherwise verify.
    let output = rejected_with(
        "module 0xCAFE::m {
    fun add(x: u64, y: u64): u64 { x + y }
    spec add {
        pragma addition_overflow_unchecked;
        aborts_if false;
        ensures result == x + y;
    }
}",
        "suppressed_obligation",
    )
    .await;
    assert!(
        output.contains("addition_overflow_unchecked"),
        "the message must name the pragma:\n{output}"
    );
}

#[tokio::test]
async fn a_partial_emits_pragma_is_rejected() {
    // The parallel of `aborts_if_is_partial` for events: the declared `emits`
    // clauses become a lower bound.
    rejected_with(
        "module 0xCAFE::m {
    fun answer(): u64 { 42 }
    spec answer {
        pragma emits_is_partial;
        aborts_if false;
        ensures result == 42;
    }
}",
        "suppressed_obligation",
    )
    .await;
}

#[tokio::test]
async fn a_concrete_condition_does_not_cover_a_category() {
    // A `[concrete]` condition is verified against the body but never assumed
    // at a call site, so it publishes nothing to callers and cannot satisfy
    // the task's required category on the contract's behalf.
    let package = common::make_package("concrete", &[(
        "pure",
        "module 0xCAFE::pure {
    fun answer(): u64 { 42 }
    spec answer {
        aborts_if false;
        ensures [concrete] result == 42;
    }
}",
    )]);
    let criteria = tempfile::TempDir::new().unwrap();
    let config = criteria.path().join("candidate-check.json");
    fs::write(
        &config,
        serde_json::to_string(&json!({
            "schema_version": 1,
            "package": package.path(),
            "target": "0xCAFE::pure::answer",
            "allowed_edit_paths": ["sources/**/*.move"],
            "required_contract_categories": ["normal-result"],
            "timeout_seconds": 40,
        }))
        .unwrap(),
    )
    .unwrap();
    let client = common::make_evaluation_client(InferenceTactic::AgentOnly, &config).await;
    let result = common::call_tool(
        &client,
        "move_spec_check",
        json!({ "package_path": package.path() }),
    )
    .await;
    let output = common::format_tool_result(&result);
    assert!(
        !output.contains("does not compile"),
        "the fixture must compile, or the test proves nothing:\n{output}"
    );
    assert!(
        !output.contains("CANDIDATE_ACCEPTED"),
        "a proof-only condition must not cover a required category:\n{output}"
    );
}

#[tokio::test]
async fn an_opaque_helper_outside_scope_is_disclosed_without_a_baseline() {
    // A scoped session has no pristine copy, so the check cannot tell which
    // opacity the candidate added. It must not stay silent either: the
    // target's proof used the helper's contract instead of its body.
    let package = common::make_package("scoped", &[(
        "pure",
        "module 0xCAFE::pure {
    fun helper(x: u64): u64 { x }
    spec helper {
        pragma opaque;
        aborts_if false;
        ensures result == 7;
    }
    fun answer(): u64 { helper(42) }
    spec answer {
        aborts_if false;
        ensures result == 7;
    }
}",
    )]);
    let client = common::make_client_for_tactic(InferenceTactic::AgentOnly).await;
    let result = common::call_tool(
        &client,
        "move_spec_check",
        json!({ "package_path": package.path().to_str().unwrap(), "filter": "pure::answer" }),
    )
    .await;
    let output = common::format_tool_result(&result);
    assert!(
        !output.contains("does not compile"),
        "the fixture must compile, or the test proves nothing:\n{output}"
    );
    assert!(
        output.contains("assumed, not proved") && output.contains("pure::helper"),
        "an out-of-scope opaque helper must be disclosed:\n{output}"
    );
}

#[tokio::test]
async fn a_vacuous_loop_invariant_is_rejected() {
    // `invariant true` constrains nothing, so it must not satisfy the
    // loop-invariant category the way a real invariant would.
    rejected_with(
        "module 0xCAFE::m {
    fun count(n: u64): u64 {
        let i = 0;
        while (i < n) {
            i = i + 1;
        } spec {
            invariant true;
        };
        i
    }
    spec count {
        aborts_if false;
        ensures result == n;
    }
}",
        "vacuous_invariant",
    )
    .await;
}

#[tokio::test]
async fn a_numeric_unroll_pragma_is_rejected() {
    // `unroll` carries a depth, not a flag: a boolean pragma check never sees
    // it, and the proof would cover only that many iterations.
    let output = rejected_with(
        "module 0xCAFE::m {
    fun count(n: u64): u64 {
        let i = 0;
        while (i < n) {
            i = i + 1;
        };
        i
    }
    spec count {
        pragma unroll = 4;
        aborts_if false;
        ensures result == n;
    }
}",
        "suppressed_obligation",
    )
    .await;
    assert!(
        output.contains("unroll = 4"),
        "the message must name the depth:\n{output}"
    );
}

#[tokio::test]
async fn a_changed_intrinsic_contract_is_rejected() {
    // An intrinsic function is modelled by the prover rather than proved, so
    // its contract is assumed at call sites just as an opaque one's is.
    // Rewriting an existing one must not slip through as unchanged.
    let source = "module 0xCAFE::pure {\n    fun helper(x: u64): u64 { x }\n    \
                  spec helper {\n        pragma intrinsic;\n        aborts_if false;\n        \
                  ensures result == x;\n    }\n    fun answer(): u64 { helper(42) }\n}\n";
    let baseline = common::make_package("pure", &[("pure.move", source)]);
    let package = common::make_package("pure", &[("pure.move", source)]);
    fs::write(
        package.path().join("sources").join("pure.move"),
        "module 0xCAFE::pure {\n    fun helper(x: u64): u64 { x }\n    \
         spec helper {\n        pragma intrinsic;\n        aborts_if false;\n        \
         ensures result == 7;\n    }\n    fun answer(): u64 { helper(42) }\n    \
         spec answer {\n        aborts_if false;\n        ensures result == 7;\n    }\n}\n",
    )
    .unwrap();
    let criteria = tempfile::TempDir::new().unwrap();
    let config = criteria.path().join("candidate-check.json");
    write_config(&config, baseline.path(), package.path());
    let client = common::make_evaluation_client(InferenceTactic::AgentOnly, &config).await;
    let result = common::call_tool(
        &client,
        "move_spec_check",
        json!({ "package_path": package.path() }),
    )
    .await;
    let output = common::format_tool_result(&result);
    assert!(
        !output.contains("does not compile"),
        "the fixture must compile, or the test proves nothing:\n{output}"
    );
    assert!(
        output.contains("CANDIDATE_REJECTED") && output.contains("changed_opaque_contract"),
        "a rewritten intrinsic contract must be rejected:\n{output}"
    );
}

#[tokio::test]
async fn a_suppressing_pragma_outside_scope_is_rejected() {
    // The prover inlines a transparent callee, so a bounded unroll on a helper
    // the filter does not select still bounds the target's proof.
    let source = "module 0xCAFE::pure {\n    fun helper(n: u64): u64 {\n        \
                  let i = 0;\n        while (i < n) { i = i + 1; };\n        i\n    }\n    \
                  fun answer(): u64 { helper(3) }\n}\n";
    let baseline = common::make_package("pure", &[("pure.move", source)]);
    let package = common::make_package("pure", &[("pure.move", source)]);
    fs::write(
        package.path().join("sources").join("pure.move"),
        "module 0xCAFE::pure {\n    fun helper(n: u64): u64 {\n        \
         let i = 0;\n        while (i < n) { i = i + 1; };\n        i\n    }\n    \
         spec helper {\n        pragma unroll = 4;\n    }\n    \
         fun answer(): u64 { helper(3) }\n    \
         spec answer {\n        aborts_if false;\n        ensures result == 3;\n    }\n}\n",
    )
    .unwrap();
    let criteria = tempfile::TempDir::new().unwrap();
    let config = criteria.path().join("candidate-check.json");
    write_config(&config, baseline.path(), package.path());
    let client = common::make_evaluation_client(InferenceTactic::AgentOnly, &config).await;
    let result = common::call_tool(
        &client,
        "move_spec_check",
        json!({ "package_path": package.path() }),
    )
    .await;
    let output = common::format_tool_result(&result);
    assert!(
        !output.contains("does not compile"),
        "the fixture must compile, or the test proves nothing:\n{output}"
    );
    assert!(
        output.contains("suppressed_obligation") && output.contains("unroll = 4"),
        "a suppressing pragma on an inlined helper must be rejected:\n{output}"
    );
}
