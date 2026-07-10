// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Library for the mono-move differential, snapshot, and unit test harnesses.

pub mod compile;
pub mod engine;
pub mod extensions;
pub mod matcher;
pub mod module_provider;
pub mod parser;
pub mod print_sections;
pub mod programs;
pub mod resource_provider;
pub mod runner;
pub mod unit_test;
pub mod v1_test_natives;

pub use compile::{
    assemble_masm_source, compile, compile_move_path, compile_move_source, SourceKind,
};
pub use engine::{
    build_natives, with_loaded_mono_function, with_mono_function, MonoRunner, RunResult,
};
pub use module_provider::InMemoryModuleProvider;
pub use resource_provider::InMemoryResourceProvider;
pub use runner::{finalize_events_v1, finalize_events_v2};
