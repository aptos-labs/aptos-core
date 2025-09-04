// Copyright © Velor Foundation
// Parts of the project are originally copyright © Meta Platforms, Inc.
// SPDX-License-Identifier: Apache-2.0

//! Logic for loader module universes.

use move_core_types::language_storage::ModuleId;
use move_model::{code_writer::CodeWriter, emit, emitln, model::Loc};
use std::{
    collections::HashSet,
    fs::create_dir,
    path::{Path, PathBuf},
};

pub fn generate_package(
    base_path: &Path,
    self_addr: &ModuleId,
    deps: &[ModuleId],
    self_val: u64,
) -> PathBuf {
    let mut package_path = base_path.to_path_buf();
    package_path.push(self_addr.name().as_str());

    let mut source_path = package_path.clone();
    source_path.push("sources");

    if !package_path.exists() {
        create_dir(&package_path).unwrap();
        create_dir(&source_path).unwrap();
    }

    source_path.push(format!("{}.move", self_addr.name()));
    std::fs::write(&source_path, generate_module(self_addr, deps, self_val)).unwrap();

    package_path.push("Move.toml");
    std::fs::write(&package_path, generate_package_toml(self_addr, deps)).unwrap();
    package_path.pop();
    package_path
}

pub fn generate_package_toml(self_addr: &ModuleId, deps: &[ModuleId]) -> String {
    let writer = CodeWriter::new(Loc::default());
    emitln!(
        writer,
        r#"[package]
name = "{}"
version = "0.0.0"

[dependencies]"#,
        self_addr.name(),
    );
    let mut visited = HashSet::new();
    for dep in deps {
        if !visited.contains(dep) {
            emitln!(
                writer,
                r#"{} = {{ local = "../{}"}}"#,
                dep.name(),
                dep.name()
            );
            visited.insert(dep);
        }
    }
    writer.process_result(|s| s.to_string())
}

pub fn generate_module(self_addr: &ModuleId, deps: &[ModuleId], self_val: u64) -> String {
    let writer = CodeWriter::new(Loc::default());
    emit!(
        writer,
        "module {}::{} ",
        self_addr.address(),
        self_addr.name()
    );
    emitln!(writer, "{");
    writer.indent();
    let mut visited = HashSet::new();
    for dep in deps {
        if !visited.contains(dep) {
            emitln!(writer, "use {}::{};", dep.address(), dep.name());
            visited.insert(dep);
        }
    }
    emitln!(writer);
    emitln!(writer, "public fun foo(): u64 {");
    writer.indent();
    emit!(writer, "let a = {}", self_val);
    for dep in deps {
        emit!(writer, "+ {}::foo()", dep.name(),);
    }
    emit!(writer, ";\n");
    emitln!(writer, "a");
    writer.unindent();
    emitln!(writer, "}");
    writer.unindent();
    emitln!(writer, "public entry fun foo_entry(expected_value: u64) {");
    writer.indent();
    emitln!(writer, "assert!(Self::foo() == expected_value, 42);");
    writer.unindent();
    emitln!(writer, "}");

    emitln!(writer, "}");
    writer.process_result(|s| s.to_string())
}
