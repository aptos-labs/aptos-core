// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Loops comparing exempt same-module calls, checked cross-module calls, and
//! closure calls. The source also serves as a differential test.

use crate::compile::find_module;
use move_binary_format::file_format::CompiledModule;

/// Canonical Move source; the same file the differential test drives.
pub const SOURCE: &str = include_str!("../../tests/test_cases/differential/programs/calls.move");

/// The module the entries live in.
pub const MODULE: &str = "calls";

/// The entry functions, each performing `n` calls of one kind.
pub const ENTRIES: [&str; 3] = ["same_module", "cross_module", "closure"];

/// Compiles the source into `(calls, calls_helper)` for the Move VM benchmark.
pub fn move_bytecode_calls() -> (CompiledModule, CompiledModule) {
    let modules = crate::compile::compile_move_source(SOURCE).expect("Move compilation failed");
    (
        find_module(&modules, MODULE).clone(),
        find_module(&modules, "calls_helper").clone(),
    )
}
