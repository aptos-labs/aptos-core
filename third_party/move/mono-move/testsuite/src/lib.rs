// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Library for the mono-move differential test harness.

pub mod matcher;
pub mod module_provider;
pub mod parser;
pub mod runner;

pub use module_provider::InMemoryModuleProvider;
use move_binary_format::file_format::CompiledModule;

/// Compile Move sources into all contained modules.
pub fn compile_move_modules(source: &str) -> Vec<CompiledModule> {
    use std::io::Write;

    let tmp_dir = tempfile::tempdir().expect("Should always be able to create temporary directory");
    let src_path = tmp_dir.path().join("sources.move");
    std::fs::File::create(&src_path)
        .and_then(|mut f| f.write_all(source.as_bytes()))
        .expect("failed to write temp source file");

    let options = move_compiler_v2::Options {
        sources: vec![src_path.to_string_lossy().into_owned()],
        named_address_mapping: vec!["std=0x1".to_string()],
        ..move_compiler_v2::Options::default()
    };

    let (_env, units) =
        move_compiler_v2::run_move_compiler_to_stderr(options).expect("Move compilation failed");

    use legacy_move_compiler::compiled_unit::CompiledUnitEnum;

    let mut modules = Vec::new();
    for unit in units {
        if let CompiledUnitEnum::Module(m) = unit.into_compiled_unit() {
            modules.push(m.module);
        }
    }
    assert!(!modules.is_empty(), "no module found in compiled output");
    modules
}
