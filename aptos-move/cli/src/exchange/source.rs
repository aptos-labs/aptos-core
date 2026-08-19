// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Source mode: compiles a self-contained Move module (source text, no
//! dependencies) with compiler v2 — so specifications from `spec` blocks
//! are populated in the model — and dumps it in the same form as the
//! masm mode (a `move_model_exchange::Module`).  Contracts come from
//! `FunctionEnv::get_spec()` and loop invariants from the `Prop`
//! instructions the stackless generator emits at loop headers.  Typing
//! assumptions are not synthesized: consumers derive well-formedness from
//! the declared types (see the `move-model-exchange` crate docs).

use crate::exchange::dump_module_from_model;
use anyhow::{anyhow, bail, Result};
use codespan_reporting::term::termcolor::Buffer;
use move_model::metadata::LATEST_STABLE_COMPILER_VERSION_VALUE;
use move_model_exchange as exchange;
use std::io::Write;

/// Runs the source-mode frontend on Move source text.  The text goes
/// through a uniquely-named temporary file (concurrent invocations do not
/// interfere); prefer [`move_file_to_module`] when the source is on disk,
/// so that diagnostics reference the real path.
pub fn move_source_to_module(source: &str) -> Result<exchange::Module> {
    let mut file = tempfile::Builder::new()
        .prefix("move-exchange-source-")
        .suffix(".move")
        .tempfile()?;
    file.write_all(source.as_bytes())?;
    move_file_to_module(file.path())
}

/// Runs the source-mode frontend on a Move source file.
pub fn move_file_to_module(path: &std::path::Path) -> Result<exchange::Module> {
    // Vector bytecodes are represented in stackless code as calls into the
    // standard `0x1::vector` module. Load the same source dependency used by
    // compiler-v2's own tests so source checking and the model both have the
    // native function declarations available.
    let stdlib_sources = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../third_party/move/move-stdlib/sources")
        .canonicalize()?;
    let compiler_options = move_compiler_v2::Options {
        sources: vec![path.to_string_lossy().to_string()],
        dependencies: vec![stdlib_sources.to_string_lossy().to_string()],
        sources_deps: vec![],
        named_address_mapping: vec!["std=0x1".to_string()],
        compiler_version: Some(LATEST_STABLE_COMPILER_VERSION_VALUE),
        skip_attribute_checks: true,
        known_attributes: Default::default(),
        compile_test_code: false,
        compile_verify_code: true,
        ..Default::default()
    };
    let mut error_writer = Buffer::no_color();
    let env = move_compiler_v2::run_move_compiler_for_analysis(&mut error_writer, compiler_options)
        .map_err(|e| {
            anyhow!(
                "Move compilation failed: {:#}\n{}",
                e,
                String::from_utf8_lossy(&error_writer.into_inner())
            )
        })?;
    let target_modules: Vec<_> = env
        .get_modules()
        .filter(|m| m.is_target())
        .map(|m| m.get_id())
        .collect();
    let [module_id] = target_modules.as_slice() else {
        bail!(
            "expected exactly one module in the source, found {}",
            target_modules.len()
        );
    };
    dump_module_from_model(&env, *module_id)
}
