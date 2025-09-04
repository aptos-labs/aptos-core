// Copyright (c) Velor Foundation
// SPDX-License-Identifier: Apache-2.0

use codespan_reporting::{diagnostic::Severity, term::termcolor::Buffer};
use move_compiler_v2::{diagnostics::human::HumanEmitter, run_move_compiler, Experiment};
use move_linter::MoveLintChecks;
use move_model::metadata::{CompilerVersion, LanguageVersion};
use move_prover_test_utils::baseline_test;
use std::{
    collections::BTreeMap,
    path::{Path, PathBuf},
};

/// Extension for expected output files.
pub const EXP_EXT: &str = "exp";

datatest_stable::harness!(test_runner, "tests", r".*\.move$");

fn test_runner(path: &Path) -> datatest_stable::Result<()> {
    let compiler_options = move_compiler_v2::Options {
        sources: vec![path.display().to_string()],
        dependencies: vec![path_from_crate_root("../../move-stdlib/sources")],
        named_address_mapping: vec![
            "std=0x1".to_string(),
            "velor_std=0x1".to_string(),
            "velor_framework=0x1".to_string(),
        ],
        language_version: Some(LanguageVersion::latest_stable()),
        compiler_version: Some(CompilerVersion::latest_stable()),
        experiments: vec![Experiment::LINT_CHECKS.to_string()],
        external_checks: vec![MoveLintChecks::make(BTreeMap::from([(
            "checks".to_string(),
            "experimental".to_string(),
        )]))],
        ..Default::default()
    };
    let mut output = String::new();
    let mut error_writer = Buffer::no_color();
    let mut emitter = HumanEmitter::new(&mut error_writer);
    match run_move_compiler(&mut emitter, compiler_options) {
        Err(e) => {
            output.push_str(&format!(
                "Aborting with compilation errors:\n{:#}\n{}\n",
                e,
                String::from_utf8_lossy(&error_writer.into_inner())
            ));
        },
        Ok((env, _)) => {
            env.report_diag(&mut error_writer, Severity::Warning);
            let diag = String::from_utf8_lossy(&error_writer.into_inner()).to_string();
            if !diag.is_empty() {
                output.push_str(&format!("\nDiagnostics:\n{}", diag));
            } else {
                output.push_str("\nNo errors or warnings!");
            }
        },
    }
    // Generate/check baseline.
    let baseline_path = path.with_extension(EXP_EXT);
    baseline_test::verify_or_update_baseline(baseline_path.as_path(), &output)?;
    Ok(())
}

/// Returns a path relative to the crate root.
fn path_from_crate_root(path: &str) -> String {
    let mut buf = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    buf.push(path);
    buf.to_string_lossy().to_string()
}
