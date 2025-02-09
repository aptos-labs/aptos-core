// Copyright (c) The Diem Core Contributors
// Copyright (c) The Move Contributors
// SPDX-License-Identifier: Apache-2.0

use codespan_reporting::{diagnostic::Severity, term::termcolor::Buffer};
use move_command_line_common::testing::get_compiler_exp_extension;
use move_compiler::shared::known_attributes::KnownAttribute;
use move_model::{
    metadata::LanguageVersion, options::ModelBuilderOptions, run_model_builder_in_compiler_mode,
    PackageInfo,
};
use move_prover_test_utils::baseline_test::verify_or_update_baseline;
use std::path::Path;

fn test_runner(path: &Path, options: ModelBuilderOptions) -> datatest_stable::Result<()> {
    let mut options = options;
    options.compile_via_model = true;
    let source_info = PackageInfo {
        sources: vec![path.to_str().unwrap().to_string()],
        address_map: std::collections::BTreeMap::<String, _>::new(),
    };
    let source_dep_info = PackageInfo {
        sources: vec![],
        address_map: std::collections::BTreeMap::<String, _>::new(),
    };
    let dep_info = vec![];
    let env = run_model_builder_in_compiler_mode(
        source_info,
        source_dep_info,
        dep_info,
        false,
        KnownAttribute::get_all_attribute_names(),
        LanguageVersion::latest(),
        false,
        false,
        false,
        true,
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
    if path.display().to_string().contains("/compile_via_model/") {
        test_runner(path, ModelBuilderOptions {
            compile_via_model: true,
            ..Default::default()
        })
    } else {
        test_runner(path, ModelBuilderOptions::default())
    }
}

datatest_stable::harness!(runner, "tests/sources", r".*\.move$");
