// Copyright © Aptos Foundation
// Parts of the project are originally copyright © Meta Platforms, Inc.
// SPDX-License-Identifier: Apache-2.0

use codespan_reporting::{diagnostic::Severity, term::termcolor::Buffer};
use move_compiler_v2::{logging, run_move_compiler_for_analysis};
use move_decompiler::{Decompiler, Options};
use move_model::metadata::LanguageVersion;
use move_prover_test_utils::{baseline_test, extract_test_directives};
use std::path::Path;

/// Extension for expected output files
pub const EXP_EXT: &str = "exp";

datatest_stable::harness!(test_runner, "tests", r".*\.move$");

fn test_runner(path: &Path) -> datatest_stable::Result<()> {
    logging::setup_logging_for_testing(None);
    let path_str = path.display().to_string();
    let mut compiler_options = move_compiler_v2::Options {
        sources_deps: extract_test_directives(path, "// dep:")?,
        sources: vec![path_str.clone()],
        dependencies: vec!["./tests".to_string()],
        named_address_mapping: vec![
            "std=0x1".to_string(),
            "aptos_std=0x1".to_string(),
            "aptos_framework=0x1".to_string(),
        ],
        ..Default::default()
    };
    compiler_options = compiler_options.set_language_version(LanguageVersion::latest());
    let mut output = String::new();
    let mut error_writer = Buffer::no_color();
    match run_move_compiler_for_analysis(&mut error_writer, compiler_options) {
        Err(e) => {
            output.push_str(&format!(
                "--- Aborting with compilation errors:\n{:#}\n{}\n",
                e,
                String::from_utf8_lossy(&error_writer.into_inner())
            ));
        },
        Ok(env) => {
            let decompiler_options = Options {
                no_expressions: false,
                ..Options::default()
            };
            let mut decompiler = Decompiler::new(decompiler_options);
            for module_env in env.get_modules() {
                if !module_env.is_primary_target() {
                    continue;
                }
                if let Some(compiled_module) = module_env.get_verified_module() {
                    let source_map = module_env.get_source_map().cloned().unwrap_or_else(|| {
                        let mut bytes = vec![];
                        compiled_module
                            .serialize(&mut bytes)
                            .expect("expected serialization success");
                        decompiler.empty_source_map(&module_env.get_full_name_str(), &bytes)
                    });
                    output += "\n";
                    output += &decompiler.decompile_module(compiled_module.clone(), source_map);
                }
            }
            if decompiler
                .env()
                .check_diag(&mut error_writer, Severity::Warning, "decompilation")
                .is_err()
            {
                output.push_str(&format!(
                    "--- Decompilation errors:\n{}\n",
                    String::from_utf8_lossy(&error_writer.into_inner())
                ));
            }
        },
    }
    // Generate/check baseline.
    let baseline_path = path.with_extension(EXP_EXT);
    baseline_test::verify_or_update_baseline(baseline_path.as_path(), &output)?;
    Ok(())
}
