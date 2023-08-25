// Copyright © Aptos Foundation
// Parts of the project are originally copyright © Meta Platforms, Inc.
// SPDX-License-Identifier: Apache-2.0

//! Logic for account universes. This is not in the parent module to enforce privacy.

use move_core_types::language_storage::ModuleId;
use move_model::{code_writer::CodeWriter, emit, emitln, model::Loc};

pub fn create_module(
    self_addr: &ModuleId,
    deps: &[ModuleId],
    self_val: u64,
    expected_val: u64,
) -> String {
    let writer = CodeWriter::new(Loc::default());
    emit!(
        writer,
        "module 0x{}.{} ",
        self_addr.address(),
        self_addr.name()
    );
    emitln!(writer, "{");
    writer.indent();
    for dep in deps {
        emitln!(writer, "import 0x{}.{};", dep.address(), dep.name());
    }
    emitln!(writer);
    emitln!(writer, "public entry foo(): u64 {");
    writer.indent();
    emitln!(writer, "let a: u64;");
    writer.unindent();
    emitln!(writer, "label b0:");
    writer.indent();
    emit!(writer, "a = {}", self_val);
    for dep in deps {
        emit!(writer, "+ {}.foo()", dep.name());
    }
    emit!(writer, ";\n");
    emitln!(writer, "assert(copy(a) == {}, 42);", expected_val);
    emitln!(writer, "return move(a);");
    writer.unindent();
    emitln!(writer, "}");
    writer.unindent();
    emitln!(writer, "}");
    writer.process_result(|s| s.to_string())
}
