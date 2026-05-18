// Parts of the file are Copyright (c) The Diem Core Contributors
// Parts of the file are Copyright (c) The Move Contributors
// Parts of the file are Copyright (c) Aptos Foundation
// All Aptos Foundation code and content is licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

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
