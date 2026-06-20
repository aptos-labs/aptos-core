// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Runs a Move package's `#[test]` unit tests on mono-move and prints a
//! coverage scoreboard. Each test below is just a different package path
//! through the same entry point ([`unit_test::run_package_unit_tests`]).
//!
//! Tests mono-move can't run yet (missing natives, heap arguments, ...) are
//! reported as *unsupported* and do not fail the run; only a test mono-move
//! runs but gets wrong is a failure. Run with `--nocapture` to see the
//! scoreboard.

use mono_move_testsuite::unit_test;
use std::path::Path;

fn run_tests_for_pkg(pkg: &str, use_latest_language: bool) {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../../../aptos-move/framework")
        .join(pkg);

    let summary = unit_test::run_package_unit_tests(&path, use_latest_language)
        .unwrap_or_else(|err| panic!("failed to run {pkg} unit tests: {err}"));

    println!("{}", summary.render());

    assert!(
        summary.failed.is_empty(),
        "{} test(s) mono-move run produced the wrong result:\n{}",
        summary.failed.len(),
        summary.failed.join("\n"),
    );
}

#[test]
#[ignore]
fn move_stdlib_on_mono_move() {
    run_tests_for_pkg("move-stdlib", false);
}

#[test]
#[ignore]
fn aptos_stdlib_on_mono_move() {
    run_tests_for_pkg("aptos-stdlib", false);
}

#[test]
#[ignore]
fn aptos_framework_on_mono_move() {
    run_tests_for_pkg("aptos-framework", false);
}

#[test]
fn aptos_token_on_mono_move() {
    run_tests_for_pkg("aptos-token", false);
}

#[test]
fn aptos_token_objects_on_mono_move() {
    run_tests_for_pkg("aptos-token-objects", false);
}

#[test]
fn aptos_trading_on_mono_move() {
    run_tests_for_pkg("aptos-trading", false);
}

#[test]
fn aptos_experimental_on_mono_move() {
    run_tests_for_pkg("aptos-experimental", true);
}
