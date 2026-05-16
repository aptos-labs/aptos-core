// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Snapshot tests for the move-linter.
//!
//! Each `tests/**/*.move` file is a fixture: the linter runs over it and the
//! output is compared against a sibling `<name>.exp` baseline.
//!
//! # Adding a fixture
//!
//! 1. Create `tests/<subdir>/<name>.move` with the Move code under test.
//! 2. Optionally add a `// @checks=<spec>` header on the first line to override
//!    the lint spec (e.g. `experimental`, `strict,-needless_visibility`,
//!    `use_receiver_style`). The grammar matches the CLI `--checks` flag.
//!    No header → `default` tier.
//! 3. Run `UPBL=1 cargo test -p move-linter` to generate `<name>.exp`.
//! 4. Run `cargo test -p move-linter` to verify.

use codespan_reporting::{diagnostic::Severity, term::termcolor::Buffer};
use move_compiler_v2::{diagnostics::human::HumanEmitter, run_move_compiler, Experiment};
use move_linter::{LintSpec, MoveLintChecks};
use move_model::metadata::{CompilerVersion, LanguageVersion};
use move_prover_test_utils::{baseline_test, extract_test_directives};
use std::{
    collections::BTreeSet,
    path::{Path, PathBuf},
};

/// Extension for expected output files.
pub const EXP_EXT: &str = "exp";

/// Header pattern that test fixtures use to override the default lint spec.
/// E.g. `// @checks=strict,-needless_visibility`.
const CHECKS_HEADER: &str = "// @checks=";

datatest_stable::harness!(test_runner, "tests", r".*\.move$");

fn test_runner(path: &Path) -> datatest_stable::Result<()> {
    let spec = match read_spec_from_file(path) {
        Ok(spec) => spec,
        Err(e) => return finalize_baseline(path, format!("Invalid `// @checks=` header: {e}\n")),
    };
    let lint_checks = match MoveLintChecks::make(spec) {
        Ok(c) => c,
        Err(e) => return finalize_baseline(path, format!("Lint configuration error: {e}\n")),
    };
    let compiler_options = move_compiler_v2::Options {
        sources: vec![path.display().to_string()],
        dependencies: vec![path_from_crate_root("../../move-stdlib/sources")],
        named_address_mapping: vec![
            "std=0x1".to_string(),
            "aptos_std=0x1".to_string(),
            "aptos_framework=0x1".to_string(),
        ],
        language_version: Some(LanguageVersion::latest()),
        compiler_version: Some(CompilerVersion::latest_stable()),
        experiments: vec![Experiment::LINT_CHECKS.to_string()],
        known_attributes: BTreeSet::from([
            "view".to_string(),
            "resource_group".to_string(),
            "resource_group_member".to_string(),
        ]),
        external_checks: vec![lint_checks],
        // Make test callers visible to the linter.
        compile_test_code: true,
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
    finalize_baseline(path, output)
}

fn finalize_baseline(path: &Path, output: String) -> datatest_stable::Result<()> {
    let baseline_path = path.with_extension(EXP_EXT);
    baseline_test::verify_or_update_baseline(baseline_path.as_path(), &output)?;
    Ok(())
}

/// Read the `// @checks=<spec>` header from a `.move` test fixture, if any.
/// Convention: place the header on the first line. When absent, fall back to
/// the `default` tier (matches `aptos move lint` invoked without `--checks`).
fn read_spec_from_file(path: &Path) -> anyhow::Result<LintSpec> {
    match extract_test_directives(path, CHECKS_HEADER)?.first() {
        Some(s) => LintSpec::parse(s),
        None => Ok(LintSpec::default()),
    }
}

/// Returns a path relative to the crate root.
fn path_from_crate_root(path: &str) -> String {
    let mut buf = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    buf.push(path);
    buf.to_string_lossy().to_string()
}
