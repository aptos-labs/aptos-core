// Copyright (c) The Diem Core Contributors
// Copyright (c) The Move Contributors
// SPDX-License-Identifier: Apache-2.0

use anyhow::{bail, Result};
use codespan_reporting::term::termcolor::Buffer;
use move_binary_format::file_format::{CompiledModule, CompiledScript};
use move_compiler::{
    compiled_unit::AnnotatedCompiledUnit,
    shared::{known_attributes::KnownAttribute, NumericalAddress},
};
use move_model::metadata::LanguageVersion;
use std::{
    collections::BTreeMap,
    fs::File,
    io::Write,
    path::{Path, PathBuf},
};
use tempfile::tempdir;

fn compile_source_unit_v2(
    s: &str,
    file_option: Option<PathBuf>,
    named_address_mapping: BTreeMap<String, NumericalAddress>,
    deps: Vec<String>,
) -> Result<Vec<AnnotatedCompiledUnit>> {
    let dir = tempdir()?;
    let file_path = if let Some(file) = file_option {
        file
    } else {
        let path = dir.path().join("modules.move");
        let mut file = File::create(&path)?;
        writeln!(file, "{}", s)?;
        path
    };
    let options = move_compiler_v2::Options {
        sources: vec![file_path.to_str().unwrap().to_string()],
        dependencies: deps,
        named_address_mapping: named_address_mapping
            .into_iter()
            .map(|(alias, addr)| format!("{}={}", alias, addr))
            .collect(),
        known_attributes: KnownAttribute::get_all_attribute_names().clone(),
        language_version: Some(LanguageVersion::latest_stable()),
        ..move_compiler_v2::Options::default()
    };
    let mut error_writer = Buffer::no_color();
    let result = {
        let mut emitter = options.error_emitter(&mut error_writer);
        move_compiler_v2::run_move_compiler(emitter.as_mut(), options)
    };
    let error_str = String::from_utf8_lossy(&error_writer.into_inner()).to_string();
    let (_, units) = result.map_err(|_| anyhow::anyhow!("compilation errors:\n {}", error_str))?;
    dir.close()?;
    Ok(units)
}

pub fn compile_units(s: &str) -> Result<Vec<AnnotatedCompiledUnit>> {
    compile_source_unit_v2(s, None, move_stdlib::move_stdlib_named_addresses(), vec![])
}

pub fn compile_units_with_stdlib(s: &str) -> Result<Vec<AnnotatedCompiledUnit>> {
    compile_source_unit_v2(
        s,
        None,
        move_stdlib::move_stdlib_named_addresses(),
        move_stdlib::move_stdlib_files(),
    )
}

fn expect_modules(
    units: impl IntoIterator<Item = AnnotatedCompiledUnit>,
) -> impl Iterator<Item = Result<CompiledModule>> {
    units.into_iter().map(|unit| match unit {
        AnnotatedCompiledUnit::Module(annot_module) => Ok(annot_module.named_module.module),
        AnnotatedCompiledUnit::Script(_) => bail!("expected modules got script"),
    })
}

pub fn compile_modules_in_file(path: &Path) -> Result<Vec<CompiledModule>> {
    let units = compile_source_unit_v2("", Some(path.to_path_buf()), BTreeMap::new(), vec![])?;
    expect_modules(units).collect()
}

#[allow(dead_code)]
pub fn compile_modules(s: &str) -> Result<Vec<CompiledModule>> {
    expect_modules(compile_units(s)?).collect()
}

pub fn as_module(unit: AnnotatedCompiledUnit) -> CompiledModule {
    match unit {
        AnnotatedCompiledUnit::Module(annot_module) => annot_module.named_module.module,
        AnnotatedCompiledUnit::Script(_) => panic!("expected module got script"),
    }
}

pub fn as_script(unit: AnnotatedCompiledUnit) -> CompiledScript {
    match unit {
        AnnotatedCompiledUnit::Module(_) => panic!("expected script got module"),
        AnnotatedCompiledUnit::Script(annot_script) => annot_script.named_script.script,
    }
}
