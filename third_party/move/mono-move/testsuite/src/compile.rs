// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Shared compile/assemble entry points used by both differential and
//! snapshot tests.

use anyhow::{anyhow, Context, Result};
use codespan_reporting::term::termcolor::Buffer;
use legacy_move_compiler::{compiled_unit::CompiledUnit, shared::known_attributes::KnownAttribute};
use move_asm::assembler::{self, Options as AsmOptions};
use move_binary_format::{
    access::ModuleAccess,
    file_format::{CompiledScript, FunctionDefinitionIndex},
    CompiledModule,
};
use move_compiler_v2::Options;
use move_model::metadata::LanguageVersion;
use std::{io::Write, path::Path};

/// Kind of input source a test is driving.
#[derive(Clone, Copy, Debug)]
pub enum SourceKind {
    Move,
    Masm,
}

impl SourceKind {
    /// Infer from a file extension. Returns `None` for unrecognized extensions.
    pub fn from_extension(ext: &str) -> Option<Self> {
        match ext {
            "move" => Some(Self::Move),
            "masm" => Some(Self::Masm),
            _ => None,
        }
    }
}

/// Compile or assemble `source` into its contained modules.
pub fn compile(source: &str, kind: SourceKind) -> Result<Vec<CompiledModule>> {
    match kind {
        SourceKind::Move => compile_move_source(source),
        SourceKind::Masm => assemble_masm_source(source).map(|m| vec![m]),
    }
}

pub const TEST_UTILS_PATH: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/tests/test_utils/test_utils.move"
);

/// Compile a Move source file at `path` into all contained modules.
///
/// The full Move stdlib is injected as dependencies.
pub fn compile_move_path(path: &Path) -> Result<Vec<CompiledModule>> {
    run_compiler(Options {
        sources: vec![path.to_string_lossy().into_owned()],
        dependencies: aptos_move_stdlib::move_stdlib_files(),
        named_address_mapping: aptos_move_stdlib::move_stdlib_named_addresses_strings(),
        known_attributes: KnownAttribute::get_all_attribute_names().clone(),
        language_version: Some(LanguageVersion::latest_stable()),
        ..Options::default()
    })
    .map(modules)
}

/// Compile the Move stdlib into its modules, so they can be published into a
/// test's storage.
pub fn compile_move_stdlib() -> Result<Vec<CompiledModule>> {
    run_compiler(Options {
        sources: aptos_move_stdlib::move_stdlib_files(),
        named_address_mapping: aptos_move_stdlib::move_stdlib_named_addresses_strings(),
        known_attributes: KnownAttribute::get_all_attribute_names().clone(),
        language_version: Some(LanguageVersion::latest_stable()),
        ..Options::default()
    })
    .map(modules)
}

/// Runs the v2 compiler and returns its modules and scripts.
fn run_compiler(options: Options) -> Result<Vec<CompiledUnit>> {
    let mut errors = Buffer::no_color();
    let result = {
        let mut emitter = options.error_emitter(&mut errors);
        move_compiler_v2::run_move_compiler(emitter.as_mut(), options)
    };
    let (_env, units) = result.map_err(|e| {
        anyhow!(
            "Move compilation failed:\n{:#}\n{}",
            e,
            String::from_utf8_lossy(&errors.into_inner())
        )
    })?;
    Ok(units
        .into_iter()
        .map(|unit| unit.into_compiled_unit())
        .collect())
}

/// Extracts modules from compiled units, discarding scripts.
fn modules(units: Vec<CompiledUnit>) -> Vec<CompiledModule> {
    units
        .into_iter()
        .filter_map(|unit| match unit {
            CompiledUnit::Module(module) => Some(module.module),
            CompiledUnit::Script(_) => None,
        })
        .collect()
}

/// Compile Move source text into all contained modules.
///
/// The Move stdlib and the `test_utils` library are injected as dependencies,
/// so test sources can reference both.
pub fn compile_move_source(source: &str) -> Result<Vec<CompiledModule>> {
    compile_move_units(source).map(modules)
}

/// Compiles source text containing exactly one script.
/// Modules in `source` resolve references during compilation but are not
/// returned or published; execution links against published modules.
pub fn compile_move_script(source: &str) -> Result<CompiledScript> {
    let mut scripts = compile_move_units(source)?
        .into_iter()
        .filter_map(|unit| match unit {
            CompiledUnit::Script(script) => Some(script.script),
            CompiledUnit::Module(_) => None,
        });
    match (scripts.next(), scripts.next()) {
        (Some(script), None) => Ok(script),
        (None, _) => Err(anyhow!("the source defines no script")),
        (Some(_), Some(_)) => Err(anyhow!("the source defines more than one script")),
    }
}

/// Compiles Move source text with the Move stdlib and the `test_utils` library
/// as dependencies.
fn compile_move_units(source: &str) -> Result<Vec<CompiledUnit>> {
    let tmp_dir = tempfile::tempdir().context("failed to create temp dir")?;
    let path = tmp_dir.path().join("sources.move");
    std::fs::File::create(&path)
        .and_then(|mut f| f.write_all(source.as_bytes()))
        .context("failed to write temp source file")?;

    let mut dependencies = aptos_move_stdlib::move_stdlib_files();
    dependencies.push(TEST_UTILS_PATH.to_string());
    run_compiler(Options {
        sources: vec![path.to_string_lossy().into_owned()],
        dependencies,
        named_address_mapping: aptos_move_stdlib::move_stdlib_named_addresses_strings(),
        known_attributes: KnownAttribute::get_all_attribute_names().clone(),
        language_version: Some(LanguageVersion::latest_stable()),
        ..Options::default()
    })
}

/// Returns the definition index of `name`, including native functions, or
/// [`None`] if `module` does not define it.
pub fn function_def_index(module: &CompiledModule, name: &str) -> Option<FunctionDefinitionIndex> {
    let position = module.function_defs().iter().position(|fdef| {
        module
            .identifier_at(module.function_handle_at(fdef.function).name)
            .as_str()
            == name
    })?;
    Some(FunctionDefinitionIndex(position as u16))
}

/// Assemble `.masm` source text into a single module.
pub fn assemble_masm_source(source: &str) -> Result<CompiledModule> {
    let options = AsmOptions::default();
    let result = assembler::assemble(&options, source, std::iter::empty())
        .map_err(|e| anyhow!("assembly failed: {:?}", e))?;
    result.left().context("expected module, got script")
}
