// Copyright (c) Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{ast::CompileUnit, move_smith::MoveSmith};
use arbitrary::{Result, Unstructured};
use move_compiler::{
    shared::{known_attributes::KnownAttribute, Flags},
    Compiler as MoveCompiler,
};
use move_compiler_v2::Experiment;
use move_model::metadata::LanguageVersion;
use move_transactional_test_runner::{vm_test_harness, vm_test_harness::TestRunConfig};
use std::{error::Error, fs::File, io::Write, path::PathBuf};
use tempfile::{tempdir, TempDir};

pub fn raw_to_module(data: &[u8]) -> Result<CompileUnit> {
    let mut u = Unstructured::new(data);
    let mut smith = MoveSmith::default();
    smith.generate(&mut u)?;
    Ok(smith.get_compile_unit())
}

pub fn create_tmp_move_file(code: String, name_hint: Option<&str>) -> (PathBuf, TempDir) {
    let dir = tempdir().unwrap();
    let name = name_hint.unwrap_or("temp.move");
    let file_path = dir.path().join(name);
    {
        let mut file = File::create(&file_path).unwrap();
        writeln!(file, "{}", code.as_str()).unwrap();
    }
    (file_path, dir)
}

/// Compiles the given Move code using compiler v1.
pub fn compile_modules(code: String) {
    let (file_path, dir) = create_tmp_move_file(code, None);
    let (_, _units) = MoveCompiler::from_files(
        vec![file_path.to_str().unwrap().to_string()],
        vec![],
        move_stdlib::move_stdlib_named_addresses(),
        Flags::empty().set_skip_attribute_checks(false),
        KnownAttribute::get_all_attribute_names(),
    )
    .build_and_report()
    .unwrap();
    dir.close().unwrap();
}

pub fn run_transactional_test(code: String) -> Result<(), Box<dyn Error>> {
    let (file_path, dir) = create_tmp_move_file(code, None);
    let vm_test_config = TestRunConfig::ComparisonV1V2 {
        language_version: LanguageVersion::V2_0,
        v2_experiments: vec![
            (Experiment::OPTIMIZE.to_string(), true),
            (Experiment::AST_SIMPLIFY.to_string(), false),
            (Experiment::ACQUIRES_CHECK.to_string(), false),
        ],
    };
    let result = vm_test_harness::run_test_with_config_and_exp_suffix(
        vm_test_config,
        file_path.as_path(),
        &None,
    );
    dir.close().unwrap();

    match result {
        Ok(_) => Ok(()),
        Err(e) => process_transactional_test_err(e),
    }
}

pub fn process_transactional_test_err(err: Box<dyn Error>) -> Result<(), Box<dyn Error>> {
    let msg = format!("{:}", err);
    if msg.contains("error[E") {
        Err(err)
    } else {
        Ok(())
    }
}
