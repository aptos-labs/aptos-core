// Copyright (c) The Diem Core Contributors
// Copyright (c) The Move Contributors
// SPDX-License-Identifier: Apache-2.0

#![forbid(unsafe_code)]

#[macro_use(sp)]
extern crate move_ir_types;

pub mod command_line;
pub mod compiled_unit;
pub mod diagnostics;
pub mod expansion;
pub mod interface_generator;
pub mod ir_translation;
pub mod parser;
pub mod shared;
pub mod unit_test;
pub mod verification;

pub use command_line::{
    compiler::{
        generate_interface_files, output_compiled_units, Compiler, FullyCompiledProgram,
        SteppedCompiler, PASS_EXPANSION, PASS_PARSER,
    },
    MOVE_COMPILED_INTERFACES_DIR,
};
pub use parser::comments::{CommentMap, FileCommentMap, MatchedFileCommentMap};
pub use shared::Flags;
