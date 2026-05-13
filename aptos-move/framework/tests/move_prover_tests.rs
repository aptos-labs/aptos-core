// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use aptos_framework::{extended_checks, prover::ProverOptions};
use move_binary_format::file_format_common::VERSION_DEFAULT;
use move_core_types::diag_writer::DiagWriter;
use move_model::metadata::{CompilerVersion, LanguageVersion};
use move_prover_test_utils::baseline_test::verify_or_update_baseline;
use regex::Regex;
use std::{
    collections::BTreeMap,
    path::{Path, PathBuf},
};

const ENV_TEST_INCONSISTENCY: &str = "MVP_TEST_INCONSISTENCY";
const ENV_TEST_UNCONDITIONAL_ABORT_AS_INCONSISTENCY: &str =
    "MVP_TEST_UNCONDITIONAL_ABORT_AS_INCONSISTENCY";
const ENV_TEST_DISALLOW_TIMEOUT_OVERWRITE: &str = "MVP_TEST_DISALLOW_TIMEOUT_OVERWRITE";
const ENV_TEST_VC_TIMEOUT: &str = "MVP_TEST_VC_TIMEOUT";

// Note: to run these tests, use:
//
//   cargo test -- --include-ignored prover

pub fn path_in_crate<S>(relative: S) -> PathBuf
where
    S: Into<String>,
{
    let mut path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    path.push(relative.into());
    path
}

pub fn read_env_var(v: &str) -> String {
    std::env::var(v).unwrap_or_else(|_| String::new())
}

/// Build the test-mode `ProverOptions` shared by both the panic-on-error
/// and baseline-driven entry points.
fn build_test_options(shards: usize, only_shard: Option<usize>) -> ProverOptions {
    let mut options = ProverOptions::default_for_test();
    options.shards = Some(shards);
    options.only_shard = only_shard;
    options.check_inconsistency = read_env_var(ENV_TEST_INCONSISTENCY) == "1";
    options.unconditional_abort_as_inconsistency =
        read_env_var(ENV_TEST_UNCONDITIONAL_ABORT_AS_INCONSISTENCY) == "1";
    options.disallow_global_timeout_to_be_overwritten =
        read_env_var(ENV_TEST_DISALLOW_TIMEOUT_OVERWRITE) == "1";
    options.vc_timeout = read_env_var(ENV_TEST_VC_TIMEOUT)
        .parse::<usize>()
        .ok()
        .or(options.vc_timeout);
    options
}

/// Panics with a helpful message if the prover's external tools (Boogie / Z3
/// or CVC5) are not configured in the environment.
fn assert_prover_tools_available(options: &ProverOptions) {
    let no_tools = read_env_var("BOOGIE_EXE").is_empty()
        || !options.cvc5 && read_env_var("Z3_EXE").is_empty()
        || options.cvc5 && read_env_var("CVC5_EXE").is_empty();
    if no_tools {
        panic!(
            "Prover tools are not configured, \
        See https://github.com/aptos-labs/aptos-core/blob/main/aptos-move/framework/FRAMEWORK-PROVER-GUIDE.md \
        for instructions, or \
        use \"-- --skip prover\" to filter out the prover tests"
        );
    }
}

pub fn run_prover_for_pkg(
    path_to_pkg: impl Into<String>,
    shards: usize,
    only_shard: Option<usize>,
) {
    let pkg_path = path_in_crate(path_to_pkg);
    let options = build_test_options(shards, only_shard);
    assert_prover_tools_available(&options);
    options
        .prove(
            false,
            pkg_path.as_path(),
            BTreeMap::default(),
            Some(VERSION_DEFAULT),
            Some(CompilerVersion::latest_stable()),
            Some(LanguageVersion::latest_stable()),
            false, // skip_attribute_checks
            extended_checks::get_all_attribute_names(),
            &[],
        )
        .unwrap()
}

/// Run the prover on `path_to_pkg` and compare the captured diagnostic output
/// against a `.exp` baseline file (path derived from `test_file`, which should
/// be `file!()` at the call site).
///
/// Unlike [`run_prover_for_pkg`], a prover error does **not** fail the test.
/// The error is captured into the baseline output, so subsequent runs verify
/// the captured text against the stored baseline. Set `UB=1` (or `UPBL=1` /
/// `UPDATE_BASELINE=1`) to (re)create the baseline from the current output.
pub fn run_prover_for_pkg_with_baseline(
    test_file: &str,
    path_to_pkg: impl Into<String>,
    shards: usize,
    only_shard: Option<usize>,
) {
    let pkg_path = path_in_crate(path_to_pkg);
    let mut options = build_test_options(shards, only_shard);
    // Redact non-deterministic values (signer addresses, fresh temp ids, …)
    // in the prover's diagnostic output so the captured baseline is stable
    // across runs — same mechanism as `move-prover/tests/testsuite.rs`.
    options.stable_test_output = true;
    assert_prover_tools_available(&options);

    let (mut writer, buf) = DiagWriter::new_buffer();
    let result = options.prove_to(
        &mut writer,
        false,
        pkg_path.as_path(),
        BTreeMap::default(),
        Some(VERSION_DEFAULT),
        Some(CompilerVersion::latest_stable()),
        Some(LanguageVersion::latest_stable()),
        false, // skip_attribute_checks
        extended_checks::get_all_attribute_names(),
        &[],
    );

    // Mirror the format produced by `move-prover/tests/testsuite.rs`: error
    // message (if any) first, then the captured diagnostic buffer. An empty
    // baseline means clean verification.
    let mut diags = match &result {
        Ok(()) => String::new(),
        Err(err) => format!("Move prover returns: {err}\n"),
    };
    diags += &String::from_utf8_lossy(buf.lock().unwrap().as_slice());

    check_baseline(test_file, &diags);
}

/// Compute the `.exp` baseline path for a test source file. Pass `file!()`
/// from the test module.
fn exp_path(test_file: &str) -> PathBuf {
    let workspace_root = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .expect("workspace root must exist");
    workspace_root.join(test_file).with_extension("exp")
}

/// Strip filesystem-dependent paths from the captured prover output so the
/// baseline is stable across machines and runs.
fn sanitize_output(s: &str) -> String {
    // Replace temp-dir-style paths: /tmp/..., /var/..., /private/var/...
    let re_tmp = Regex::new(r#"(/private)?(/var|/tmp)(/[^\s,\]"`]+)*/"#).expect("regex");
    let s = re_tmp.replace_all(s, "<TEMPDIR>/");
    let re_tmp_bare = Regex::new(r"<TEMPDIR>/\.tmp[a-zA-Z0-9]+").expect("regex");
    let s = re_tmp_bare.replace_all(&s, "<TEMPDIR>");

    // Replace CARGO_MANIFEST_DIR (the framework crate root).
    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    let s = s.replace(manifest_dir, "<FRAMEWORK_DIR>");

    s.to_string()
}

/// Compare `output` against the `.exp` baseline next to the given test source.
/// When `UB` (or `UPBL` / `UPDATE_BASELINE`) is set, updates the baseline.
fn check_baseline(test_file: &str, output: &str) {
    let baseline = exp_path(test_file);
    let sanitized = sanitize_output(output);
    verify_or_update_baseline(&baseline, &sanitized)
        .unwrap_or_else(|e| panic!("baseline mismatch for {}: {}", test_file, e));
}

#[test]
fn move_framework_prover_tests() {
    run_prover_for_pkg("aptos-framework", 1, None);
}

#[test]
fn move_token_prover_tests() {
    run_prover_for_pkg("aptos-token", 1, None);
}

#[test]
fn move_aptos_stdlib_prover_tests() {
    run_prover_for_pkg("aptos-stdlib", 1, None);
}

#[test]
fn move_stdlib_prover_tests() {
    run_prover_for_pkg("move-stdlib", 1, None);
}
