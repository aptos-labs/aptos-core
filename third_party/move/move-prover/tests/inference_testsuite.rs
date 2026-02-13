// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

//! End-to-end inference test suite.
//!
//! For each `.move` file under `tests/inference/`, this driver:
//! 1. Compiles a Move model from the source.
//! 2. Runs the spec inference pipeline in Unified output mode, producing `.enriched.move`
//!    files that contain the original source with inferred specs inlined.
//! 3. Runs the Move Prover on the enriched file (original source + inline specs).
//! 4. Records all diagnostics to a `.exp` baseline file.

use codespan_reporting::term::termcolor::Buffer;
use libtest_mimic::{Arguments, Trial};
use log::warn;
use move_command_line_common::env::read_env_var;
use move_compiler_v2::Experiment;
use move_model::metadata::LanguageVersion;
use move_prover::{
    cli::Options,
    inference::{InferenceOptions, InferenceOutput},
    run_inference_with_bytecode_dump, run_move_prover_v2,
};
use move_prover_test_utils::{baseline_test::verify_or_update_baseline, extract_test_directives};
use std::{
    path::{Path, PathBuf},
    sync::atomic::{AtomicBool, Ordering},
};
use walkdir::WalkDir;

/// Set to `true` to dump stackless bytecode at the beginning and end of the
/// inference pipeline (similar to bytecode-pipeline tests). Off by default to
/// keep baseline files focused on the verification outcome.
const DEBUG: bool = false;

static NOT_CONFIGURED_WARNED: AtomicBool = AtomicBool::new(false);

const FUNCTION_VALUE_EXPERIMENTS: &[&str] = &[
    Experiment::KEEP_INLINE_FUNS,
    Experiment::LIFT_INLINE_FUNS,
    Experiment::SKIP_INLINING_INLINE_FUNS,
];

fn test_runner(path: &Path) -> anyhow::Result<()> {
    let mut baseline_out = String::new();

    // ── Step 1: Run spec inference in Unified mode ───────────────────

    let enriched_path = path.with_extension("enriched.move");

    // Remove any stale enriched file from a previous run so it doesn't
    // get picked up as a source during inference.
    let _ = std::fs::remove_file(&enriched_path);

    let mut inf_options = make_options(path)?;
    inf_options.inference = InferenceOptions {
        inference: true,
        inference_output: InferenceOutput::Unified,
        inference_output_dir: None, // write next to source
    };
    inf_options.setup_logging_for_test();
    inf_options.prover.stable_test_output = true;
    inf_options.backend.stable_test_output = true;

    let mut error_writer = Buffer::no_color();
    let experiments: Vec<String> = FUNCTION_VALUE_EXPERIMENTS
        .iter()
        .map(|s| String::from(*s))
        .collect();
    let (dump, result) = if DEBUG {
        match run_inference_with_bytecode_dump(&mut error_writer, inf_options, experiments.clone())
        {
            Ok(dump) => (Some(dump), Ok(())),
            Err(err) => (None, Err(err)),
        }
    } else {
        (
            None,
            run_move_prover_v2(&mut error_writer, inf_options, experiments.clone()),
        )
    };

    // Collect inference-phase diagnostics (shown at the end in a comment).
    let mut diags = String::new();
    match result {
        Ok(()) => {},
        Err(err) => {
            diags += &format!("Inference returns: {}\n", err);
        },
    }
    let inf_diags = String::from_utf8_lossy(&error_writer.into_inner()).to_string();
    if !inf_diags.is_empty() {
        diags += &format!("Inference diagnostics:\n{}", inf_diags);
    }
    if let Some(dump) = dump {
        diags += &dump;
    }

    // Record the enriched source file contents directly (no header).
    if enriched_path.exists() {
        let contents = std::fs::read_to_string(&enriched_path)?;
        baseline_out += contents.trim_end();
        baseline_out += "\n";
    }

    // ── Step 2: Run the prover on the enriched source ────────────────

    // Use the enriched file as the single source for verification.
    let verify_source = if enriched_path.exists() {
        &enriched_path
    } else {
        path
    };

    let verify_result = (|| -> anyhow::Result<()> {
        let no_tools = read_env_var("BOOGIE_EXE").is_empty() || read_env_var("Z3_EXE").is_empty();

        let mut verify_options = make_options(verify_source)?;
        verify_options.setup_logging_for_test();
        verify_options.prover.stable_test_output = true;
        verify_options.backend.stable_test_output = true;
        if no_tools {
            verify_options.prover.generate_only = true;
            if NOT_CONFIGURED_WARNED
                .compare_exchange(false, true, Ordering::Relaxed, Ordering::Relaxed)
                .is_ok()
            {
                warn!(
                    "Prover tools are not configured, verification will be skipped. \
                     Set BOOGIE_EXE and Z3_EXE to enable full verification."
                );
            }
        }
        verify_options.backend.check_tool_versions()?;

        let mut error_writer = Buffer::no_color();
        let result = run_move_prover_v2(&mut error_writer, verify_options, experiments);
        let verify_diags = String::from_utf8_lossy(&error_writer.into_inner()).to_string();
        match result {
            Ok(()) if verify_diags.is_empty() => {
                diags += "Verification: Succeeded.\n";
            },
            Ok(()) => {
                diags += &format!("Verification:\n{}", verify_diags);
            },
            Err(err) => {
                diags += &format!("Verification: {}\n", err);
                if !verify_diags.is_empty() {
                    diags += &verify_diags;
                }
            },
        }
        Ok(())
    })();

    // Clean up the enriched file so it doesn't interfere with
    // future runs or get accidentally committed.
    let _ = std::fs::remove_file(&enriched_path);

    verify_result?;

    // ── Step 3: Append diagnostics as a block comment ───────────────
    if !diags.is_empty() {
        baseline_out += &format!("/*\n{}\n*/\n", diags.trim_end());
    }

    // ── Step 4: Compare against baseline ─────────────────────────────

    let baseline_path = path.with_extension("exp.move");
    verify_or_update_baseline(baseline_path.as_path(), &baseline_out)?;

    Ok(())
}

/// Build prover `Options` for the given Move source.
fn make_options(path: &Path) -> anyhow::Result<Options> {
    let temp_dir = tempfile::TempDir::new()?;
    std::fs::create_dir_all(temp_dir.path())?;
    let base_name = format!("{}.bpl", path.file_stem().unwrap().to_str().unwrap());
    let output = temp_dir
        .path()
        .join(base_name)
        .to_str()
        .unwrap()
        .to_string();

    let mut flags: Vec<String> = vec![
        "mvp_test".to_string(),
        "--verbose=warn".to_string(),
        "--dependency=../move-stdlib/sources".to_string(),
        "--dependency=../move-stdlib/nursery/sources".to_string(),
        "--dependency=../extensions/move-table-extension/sources".to_string(),
        "--named-addresses".to_string(),
        "std=0x1".to_string(),
        "extensions=0x2".to_string(),
        format!("--output={}", output),
    ];

    // Add flags specified in the source via `// flag:` directives.
    flags.extend(extract_test_directives(path, "// flag:")?);

    // The source file itself.
    flags.push(path.to_string_lossy().to_string());

    let mut options = Options::create_from_args(&flags)?;
    options.language_version = Some(LanguageVersion::latest());
    // Keep temp_dir alive by leaking it — tests are short-lived anyway.
    std::mem::forget(temp_dir);
    Ok(options)
}

fn collect_tests(tests: &mut Vec<Trial>, dir: &str) {
    let base = PathBuf::from(dir);
    for entry in WalkDir::new(&base)
        .follow_links(false)
        .min_depth(1)
        .into_iter()
        .flatten()
    {
        let name = entry.file_name().to_string_lossy().to_string();
        if !name.ends_with(".move")
            || name.ends_with(".spec.move")
            || name.ends_with(".enriched.move")
            || name.ends_with(".exp.move")
        {
            continue;
        }
        let path = entry.path().to_path_buf();
        let test_name = format!("inference::{}", path.strip_prefix(&base).unwrap().display());
        tests.push(Trial::test(test_name, move || {
            test_runner(&path).map_err(|err| format!("{:?}", err).into())
        }));
    }
}

fn main() {
    let mut tests = vec![];
    collect_tests(&mut tests, "tests/inference");
    tests.sort_unstable_by(|a, b| a.name().cmp(b.name()));
    let args = Arguments::from_args();
    libtest_mimic::run(&args, tests).exit()
}
