// Copyright (c) Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{ast::Module, move_smith::MoveSmith};
use arbitrary::{Result, Unstructured};
use move_compiler::{
    shared::{known_attributes::KnownAttribute, Flags},
    Compiler as MoveCompiler,
};
use std::{fs::File, io::Write};
use tempfile::tempdir;

pub fn raw_to_module(data: &[u8]) -> Result<Module> {
    let mut u = Unstructured::new(data);
    let mut smith = MoveSmith::default();
    smith.generate_module(&mut u)
}

pub fn compile_modules(code: String) {
    let dir = tempdir().unwrap();
    let file_path = dir.path().join("modules.move");
    {
        let mut file = File::create(&file_path).unwrap();
        writeln!(file, "{}", code.as_str()).unwrap();
    }

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
