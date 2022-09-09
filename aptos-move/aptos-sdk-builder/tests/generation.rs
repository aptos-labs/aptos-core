// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use aptos_sdk_builder as buildgen;
use aptos_types::transaction::EntryABI;
use serde_generate as serdegen;
use serde_generate::SourceInstaller as _;
use serde_reflection::Registry;
use std::{io::Write, process::Command};
use tempfile::tempdir;

fn get_aptos_registry() -> Registry {
    let path = "../../testsuite/generate-format/tests/staged/aptos.yaml";
    let content = std::fs::read_to_string(path).unwrap();
    serde_yaml::from_str::<Registry>(content.as_str()).unwrap()
}

const EXPECTED_SCRIPT_FUN_OUTPUT: &str = "3 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 1 8 84 101 115 116 67 111 105 110 8 116 114 97 110 115 102 101 114 0 2 32 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 34 34 34 34 34 34 34 34 34 34 34 34 34 34 34 34 8 135 214 18 0 0 0 0 0 \n";

fn test_rust(abis: &[EntryABI], demo_file: &str, expected_output: &str) {
    let mut registry = get_aptos_registry();
    buildgen::rust::replace_keywords(&mut registry);
    let dir = tempdir().unwrap();

    let installer = serdegen::rust::Installer::new(dir.path().to_path_buf());
    let config = serdegen::CodeGeneratorConfig::new("aptos-types".to_string());
    installer.install_module(&config, &registry).unwrap();

    let stdlib_dir_path = dir.path().join("framework");
    std::fs::create_dir_all(stdlib_dir_path.clone()).unwrap();

    let mut cargo = std::fs::File::create(&stdlib_dir_path.join("Cargo.toml")).unwrap();
    write!(
        cargo,
        r#"[package]
name = "framework"
version = "0.1.0"
edition = "2021"

[dependencies]
aptos-types = {{ path = "../aptos-types", version = "0.1.0" }}
serde_bytes = "0.11.6"
serde = {{ version = "1.0.114", features = ["derive"] }}
bcs = "0.1.3"
once_cell = "1.10.0"

[[bin]]
name = "stdlib_demo"
path = "src/stdlib_demo.rs"
test = false
"#
    )
    .unwrap();
    std::fs::create_dir(stdlib_dir_path.join("src")).unwrap();
    let source_path = stdlib_dir_path.join("src/lib.rs");
    let mut source = std::fs::File::create(&source_path).unwrap();
    buildgen::rust::output(&mut source, abis, /* local types */ false).unwrap();

    std::fs::copy(demo_file, stdlib_dir_path.join("src/stdlib_demo.rs")).unwrap();

    // Use a stable `target` dir to avoid downloading and recompiling crates everytime.
    let target_dir = std::env::current_dir().unwrap().join("../../target");
    let status = Command::new("cargo")
        .current_dir(dir.path().join("framework"))
        .arg("build")
        .arg("--target-dir")
        .arg(target_dir.clone())
        .status()
        .unwrap();
    assert!(status.success());

    let output = Command::new(target_dir.join("debug/stdlib_demo"))
        .output()
        .unwrap();
    assert!(output.status.success());
    assert_eq!(
        std::str::from_utf8(&output.stdout).unwrap(),
        expected_output
    );
}

#[test]
// Ignored because transactions require minting/transfering Coin<AptosCoin>, which the
// transaction builder does not support (it doesn't supported typed functions yet).
#[ignore]
fn test_that_rust_entry_fun_code_compiles() {
    // TODO: need a way to get abis to reactivate this test
    let abis = vec![];
    test_rust(
        &abis, // &cached_packages::head_release_bundle().abis(),
        "examples/rust/script_fun_demo.rs",
        EXPECTED_SCRIPT_FUN_OUTPUT,
    );
}
