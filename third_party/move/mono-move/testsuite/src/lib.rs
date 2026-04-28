// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Library for the mono-move differential and snapshot test harnesses.

pub mod compile;
pub mod matcher;
pub mod module_provider;
pub mod parser;
pub mod runner;

pub use compile::{
    assemble_masm_source, compile, compile_move_path, compile_move_source, SourceKind,
};
pub use module_provider::InMemoryModuleProvider;
