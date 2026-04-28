// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Shared compile/assemble entry points used by both differential and
//! snapshot tests.

use anyhow::{anyhow, Context, Result};
use codespan_reporting::term::termcolor::Buffer;
use legacy_move_compiler::{compiled_unit::CompiledUnit, shared::known_attributes::KnownAttribute};
use move_asm::assembler::{self, Options as AsmOptions};
use move_binary_format::CompiledModule;
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

/// Compile a Move source file at `path` into all contained modules.
///
/// The full Move stdlib is injected as dependencies.
pub fn compile_move_path(path: &Path) -> Result<Vec<CompiledModule>> {
    let options = Options {
        sources: vec![path.to_string_lossy().into_owned()],
        dependencies: move_stdlib::move_stdlib_files(),
        named_address_mapping: move_stdlib::move_stdlib_named_addresses_strings(),
        known_attributes: KnownAttribute::get_all_attribute_names().clone(),
        language_version: Some(LanguageVersion::latest_stable()),
        ..Options::default()
    };

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
        .filter_map(|unit| match unit.into_compiled_unit() {
            CompiledUnit::Module(m) => Some(m.module),
            CompiledUnit::Script(_) => None,
        })
        .collect())
}

/// Compile Move source text into all contained modules.
///
/// Inherits the stdlib-injecting behavior of [`compile_move_path`].
pub fn compile_move_source(source: &str) -> Result<Vec<CompiledModule>> {
    let tmp_dir = tempfile::tempdir().context("failed to create temp dir")?;
    let path = tmp_dir.path().join("sources.move");
    std::fs::File::create(&path)
        .and_then(|mut f| f.write_all(source.as_bytes()))
        .context("failed to write temp source file")?;
    compile_move_path(&path)
}

/// Assemble `.masm` source text into a single module.
pub fn assemble_masm_source(source: &str) -> Result<CompiledModule> {
    let options = AsmOptions::default();
    let result = assembler::assemble(&options, source, std::iter::empty())
        .map_err(|e| anyhow!("assembly failed: {:?}", e))?;
    result.left().context("expected module, got script")
}
