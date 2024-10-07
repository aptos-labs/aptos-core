// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

//! This file contains tests for compiling framework code with the v1 compiler, to make sure no V2 feature is used before it's ready for mainnet.

use aptos_framework::{extended_checks, path_in_crate};
use move_model::metadata::{CompilerVersion, LanguageVersion};
use move_package::CompilerConfig;
use tempfile::tempdir;

fn compile_pkg_with_v1(path_to_pkg: impl Into<String>) {
    let pkg_path = path_in_crate(path_to_pkg);
    let compiler_config = CompilerConfig {
        known_attributes: extended_checks::get_all_attribute_names().clone(),
        bytecode_version: Some(6),
        language_version: Some(LanguageVersion::V1),
        compiler_version: Some(CompilerVersion::V1),
        ..Default::default()
    };
    let build_config = move_package::BuildConfig {
        install_dir: Some(tempdir().unwrap().path().to_path_buf()),
        compiler_config: compiler_config.clone(),
        ..Default::default()
    };
    build_config
        .compile_package(pkg_path.as_path(), &mut std::io::stdout())
        .unwrap();
}

#[test]
fn compile_aptos_stdlib_with_v1() {
    compile_pkg_with_v1("aptos-stdlib");
}

#[test]
fn compile_move_stdlib_with_v1() {
    compile_pkg_with_v1("move-stdlib");
}
