// Copyright (c) The Diem Core Contributors
// Copyright (c) The Move Contributors
// SPDX-License-Identifier: Apache-2.0

use codespan_reporting::{diagnostic::Severity, term::termcolor::Buffer};
use legacy_move_compiler::{
    shared::{known_attributes::KnownAttribute, PackagePaths},
    Flags,
};
use move_command_line_common::testing::get_compiler_exp_extension;
use move_model::{
    options::ModelBuilderOptions, run_model_builder_with_options_and_compilation_flags,
};
use move_prover_test_utils::baseline_test::verify_or_update_baseline;
use std::path::Path;

fn test_runner(path: &Path, options: ModelBuilderOptions) -> datatest_stable::Result<()> {
    let targets = vec![PackagePaths {
        name: None,
        paths: vec![path.to_str().unwrap().to_string()],
        named_address_map: std::collections::BTreeMap::<String, _>::new(),
    }];
    let env = run_model_builder_with_options_and_compilation_flags(
        targets,
        vec![],
        vec![],
        options,
        Flags::verification(),
        KnownAttribute::get_all_attribute_names(),
    )?;
    let diags = if env.diag_count(Severity::Warning) > 0 {
        let mut writer = Buffer::no_color();
        env.report_diag(&mut writer, Severity::Warning);
        String::from_utf8_lossy(&writer.into_inner()).to_string()
    } else {
        "All good, no errors!".to_string()
    };
    let baseline_path = path.with_extension(get_compiler_exp_extension());
    verify_or_update_baseline(baseline_path.as_path(), &diags)?;
    Ok(())
}

fn runner(path: &Path) -> datatest_stable::Result<()> {
    test_runner(path, ModelBuilderOptions {
        ..Default::default()
    })
}

datatest_stable::harness!(runner, "tests/sources", r".*\.move$");
