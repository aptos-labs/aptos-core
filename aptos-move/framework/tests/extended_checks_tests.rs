// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use aptos_framework::{BuildOptions, BuiltPackage};
use aptos_package_builder::PackageBuilder;
use move_core_types::diag_writer::DiagWriter;

/// Builds a one-module package and returns whether it built, with its diagnostics.
fn build(source: &str) -> (bool, String) {
    let mut builder = PackageBuilder::new("Checks");
    builder.add_source("m", source);
    let dir = builder.write_to_temp().unwrap();
    let (writer, buffer) = DiagWriter::new_buffer();
    let built = BuiltPackage::build_to(&writer, dir.path().to_path_buf(), BuildOptions::default());
    let output = String::from_utf8_lossy(buffer.lock().unwrap().as_slice()).to_string();
    (built.is_ok(), output)
}

/// `resource_group_member` without its `group` argument is an error, not a
/// panic on the missing argument.
#[test]
fn resource_group_member_without_arguments_is_an_error() {
    let (built, output) = build("module 0x42::m { struct S has key { x: u64 } }");
    assert!(built, "{output}");
    let (built, output) =
        build("module 0x42::m { #[resource_group_member] struct S has key { x: u64 } }");
    assert!(
        !built && output.contains("resource_group_member must contain 1 parameters"),
        "{output}"
    );
}
