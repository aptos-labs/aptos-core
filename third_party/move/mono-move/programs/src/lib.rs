// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Synthetic programs for testing and benchmarking the MonoMove runtime.
//!
//! Each module defines a program in multiple flavors:
//! - **Native Rust** (always available) — reference implementation
//! - **Micro-op** (feature `micro-op`) — program for the MonoMove interpreter
//! - **Move bytecode** (feature `move-bytecode`) — for the current Move VM

pub mod bst;
pub mod fib;
pub mod merge_sort;
pub mod nested_loop;
#[cfg(feature = "testing")]
pub mod testing;

// ---------------------------------------------------------------------------
// Move compilation helper
// ---------------------------------------------------------------------------

/// Path to the Move stdlib sources directory.
#[cfg(feature = "move-bytecode")]
pub const MOVE_STDLIB_DIR: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/../../move-stdlib/sources");

/// Compile a Move source string into a single `CompiledModule`.
///
/// Writes the source to a temporary file, invokes Move compiler v2, and
/// returns the first compiled module. Panics on compilation errors.
#[cfg(feature = "move-bytecode")]
pub fn compile_move_source(source: &str) -> move_binary_format::file_format::CompiledModule {
    compile_move_source_with_deps(source, &[])
}

/// Like [`compile_move_source`], but with additional dependency directories
/// (e.g., the Move stdlib sources).
#[cfg(feature = "move-bytecode")]
pub fn compile_move_source_with_deps(
    source: &str,
    deps: &[&str],
) -> move_binary_format::file_format::CompiledModule {
    use std::io::Write;

    let tmp_dir = tempfile::tempdir().expect("failed to create temp dir");
    let src_path = tmp_dir.path().join("source.move");
    std::fs::File::create(&src_path)
        .and_then(|mut f| f.write_all(source.as_bytes()))
        .expect("failed to write temp source file");

    let options = move_compiler_v2::Options {
        sources: vec![src_path.to_string_lossy().into_owned()],
        dependencies: deps.iter().map(|s| s.to_string()).collect(),
        named_address_mapping: vec!["std=0x1".to_string()],
        ..move_compiler_v2::Options::default()
    };

    let (_env, units) =
        move_compiler_v2::run_move_compiler_to_stderr(options).expect("Move compilation failed");

    // `AnnotatedCompiledUnit` is not re-exported by move_compiler_v2, so we
    // import it from legacy_move_compiler (the shared types crate).
    use legacy_move_compiler::compiled_unit::CompiledUnitEnum;

    for unit in units {
        if let CompiledUnitEnum::Module(m) = unit.into_compiled_unit() {
            return m.module;
        }
    }
    panic!("no module found in compiled output");
}
