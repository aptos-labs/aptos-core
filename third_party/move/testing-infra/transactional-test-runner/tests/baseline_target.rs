// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Tests that the public runner compares existing baselines and rejects missing
//! baselines without creating them under `UpdatePolicy::Forbidden`.

use move_transactional_test_runner::{
    framework::{BaselineTarget, UpdatePolicy},
    vm_test_harness::{run_test_with_config_and_baseline, TestRunConfig},
};
use std::path::{Path, PathBuf};
use tempfile::TempDir;

/// Returns a source with a matching source-adjacent baseline.
fn example_source() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/vm_test_harness/example.move")
}

fn run_against(baseline: &BaselineTarget) -> Result<(), Box<dyn std::error::Error>> {
    run_test_with_config_and_baseline(TestRunConfig::default(), &example_source(), baseline)
}

#[test]
fn a_canonical_baseline_still_compares_normally() {
    run_against(&BaselineTarget::beside_source_with(
        None,
        UpdatePolicy::Forbidden,
    ))
    .expect("the recorded output should match");
}

#[test]
fn a_canonical_baseline_is_never_created_by_a_compare_only_run() {
    // Use a temporary path so a policy regression cannot modify the source
    // tree.
    let dir = TempDir::new().unwrap();
    let missing = dir.path().join("nonexistent-canonical.exp");

    let error = run_against(&BaselineTarget::at(&missing, UpdatePolicy::Forbidden))
        .expect_err("a missing canonical baseline must fail the test");
    assert!(
        error.to_string().contains("missing baseline"),
        "unexpected error: {error}"
    );
    assert!(
        !missing.exists(),
        "a compare-only run must not create the baseline it was missing"
    );
}
