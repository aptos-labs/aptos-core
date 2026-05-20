// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Test utilities for running the Move Prover on the example packages
//! shipped alongside the prover papers in `move-prover/doc/`.
//!
//! Each package is verified end-to-end and its captured diagnostic output
//! is compared against a `prover.exp` baseline stored next to the package's
//! `Move.toml`. A prover error does **not** fail the test directly; the
//! error is captured into the baseline and the test only fails on baseline
//! mismatch (set `UB=1` to update).

use codespan_reporting::diagnostic::Severity;
use move_core_types::diag_writer::DiagWriter;
use move_model::metadata::{CompilerVersion, LanguageVersion};
use move_package::{source_package::layout::SourcePackageLayout, BuildConfig, ModelConfig};
use move_prover_test_utils::baseline_test::verify_or_update_baseline;
use regex::Regex;
use std::{path::Path, time::Instant};
use tempfile::TempDir;

/// Filename of the prover-output baseline, stored next to the package's
/// `Move.toml`.
pub const BASELINE_FILE: &str = "prover.exp";

/// Run the Move Prover on the package at `pkg_path` and compare the
/// captured diagnostic output against `prover.exp` next to its `Move.toml`.
///
/// Honors `UB=1` (`UPBL=1`, `UPDATE_BASELINE=1`) to (re)create the baseline.
pub fn run_paper_with_baseline(pkg_path: impl AsRef<Path>) {
    let pkg_path = pkg_path.as_ref();
    let pkg_path = pkg_path
        .canonicalize()
        .unwrap_or_else(|e| panic!("cannot canonicalize {}: {}", pkg_path.display(), e));
    let baseline = pkg_path.join(BASELINE_FILE);

    let captured = run_prover_capturing(&pkg_path);
    let sanitized = sanitize_output(&pkg_path, &captured);

    verify_or_update_baseline(&baseline, &sanitized)
        .unwrap_or_else(|e| panic!("baseline mismatch for {}: {}", pkg_path.display(), e));
}

/// Build the package model and run the prover, capturing all diagnostics
/// into an in-memory buffer. Always returns a string; on prover failure a
/// trailing `Prover returned error: …` line is appended.
fn run_prover_capturing(pkg_path: &Path) -> String {
    // The Move package system (via `move_model_for_package`) currently
    // mutates the process-wide current directory. Save and restore it so
    // unrelated tests in the same binary aren't affected.
    let saved_cd = std::env::current_dir().expect("current directory");

    let rerooted =
        SourcePackageLayout::try_find_root(pkg_path).unwrap_or_else(|_| pkg_path.to_path_buf());

    let mut args = vec!["package".to_string()];
    let prover_toml = rerooted.join("Prover.toml");
    if prover_toml.exists() {
        args.push(format!("--config={}", prover_toml.to_string_lossy()));
    }
    let mut options =
        move_prover::cli::Options::create_from_args(&args).expect("prover CLI options");
    options.set_quiet();
    // Redact non-deterministic values (signer addresses, fresh temp ids, …)
    // in the prover's diagnostic output so the captured baseline is stable
    // across runs — same mechanism as `move-prover/tests/testsuite.rs`.
    options.prover.stable_test_output = true;
    options.backend.stable_test_output = true;

    // Use a fresh temp dir for `output.bpl` so concurrent test invocations
    // don't collide on the same path.
    let temp_dir = TempDir::new().expect("temp dir");
    options.output_path = temp_dir
        .path()
        .join("output.bpl")
        .to_string_lossy()
        .to_string();

    let mut config = BuildConfig {
        dev_mode: true,
        verify_mode: true,
        test_mode: false,
        ..BuildConfig::default()
    };
    // Paper example packages use experimental BP / state-label syntax —
    // verify against the latest (possibly unstable) language version.
    config.compiler_config.language_version = Some(LanguageVersion::latest());
    config.compiler_config.compiler_version = Some(CompilerVersion::latest());
    let compiler_version = config.compiler_config.compiler_version.unwrap();
    let language_version = config.compiler_config.language_version.unwrap();

    let (mut writer, buf) = DiagWriter::new_buffer();
    let result = (|| -> anyhow::Result<()> {
        let mut model = config.move_model_for_package(&rerooted, ModelConfig {
            all_files_as_targets: false,
            target_filter: None,
            compiler_version,
            language_version,
            with_bytecode: true,
        })?;
        // `check_diag` renders the env's accumulated diagnostics into the
        // writer (so compilation errors land in our captured baseline) and
        // bails out with the given suffix when `has_errors()`.
        model.check_diag(&mut writer, Severity::Warning, "in compilation")?;
        move_prover::run_move_prover_with_model_v2(
            &mut model,
            &mut writer,
            options,
            Instant::now(),
        )?;
        Ok(())
    })();

    std::env::set_current_dir(saved_cd).expect("restore current directory");

    // Mirror the format produced by `move-prover/tests/testsuite.rs`: error
    // message (if any) first, then the captured diagnostic buffer. An empty
    // baseline means clean verification.
    let mut diags = match &result {
        Ok(()) => String::new(),
        Err(err) => format!("Move prover returns: {err}\n"),
    };
    diags += &String::from_utf8_lossy(buf.lock().unwrap().as_slice());
    diags
}

/// Strip filesystem-dependent paths from the captured output so the
/// baseline is stable across machines and runs.
fn sanitize_output(pkg_path: &Path, s: &str) -> String {
    let s = s.replace(&pkg_path.display().to_string(), "<PKG>");
    let re_tmp = Regex::new(r#"(/private)?(/var|/tmp)(/[^\s,\]"`]+)*/"#).expect("regex");
    let s = re_tmp.replace_all(&s, "<TEMPDIR>/").to_string();
    let re_tmp_bare = Regex::new(r"<TEMPDIR>/\.tmp[a-zA-Z0-9]+").expect("regex");
    re_tmp_bare.replace_all(&s, "<TEMPDIR>").to_string()
}
