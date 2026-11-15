// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Runs the framework packages' Move unit tests on V1 and MonoMove, failing a
//! test whose outcomes differ. Tests MonoMove cannot run yet are reported as
//! unsupported, with a per-reason histogram; run with `--nocapture` to see it.

mod common;

use aptos_framework::extended_checks;
use move_model::metadata::LanguageVersion;
use move_package::{BuildConfig, CompilerConfig};
use std::path::Path;

fn run_tests_for_pkg(pkg: &str, use_latest_language: bool) {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../../../aptos-move/framework")
        .join(pkg);

    let mut build_config = BuildConfig {
        compiler_config: CompilerConfig {
            known_attributes: extended_checks::get_all_attribute_names().clone(),
            ..Default::default()
        },
        ..Default::default()
    };
    if use_latest_language {
        let language_version = LanguageVersion::latest();
        let bytecode_version = language_version.infer_bytecode_version(None);
        build_config.compiler_config.language_version = Some(language_version);
        build_config.compiler_config.bytecode_version = Some(bytecode_version);
    }
    common::run_package_on_both(&path, build_config);
}

#[test]
fn move_stdlib_on_mono_move() {
    run_tests_for_pkg("move-stdlib", false);
}

#[test]
fn aptos_stdlib_on_mono_move() {
    run_tests_for_pkg("aptos-stdlib", false);
}

#[test]
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
