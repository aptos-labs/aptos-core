// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Test harness for Move CLI integration tests.
//!
//! Provides helpers to parse and execute CLI commands via clap, build temporary
//! Move packages, and compare output against `.exp` baseline files.
//!
//! Compiler output (e.g. `BUILDING`, `INCLUDING DEPENDENCY`, diagnostics) is
//! captured via a buffer-backed `MoveEnv` writer and included in baselines.

pub mod mock;

use crate::{MoveEnv, MoveTool};
use aptos_cli_common::CliResult;
use aptos_package_builder::PackageBuilder;
use clap::Parser;
use move_core_types::diag_writer::DiagWriter;
use move_prover_test_utils::baseline_test::verify_or_update_baseline;
use regex::Regex;
use std::{
    path::{Path, PathBuf},
    sync::{Arc, Mutex, Once},
};
use tempfile::TempDir;
use termcolor::Buffer;

static INIT_HOOKS: Once = Once::new();

/// Lightweight wrapper so we can parse `MoveTool` from string args via clap.
#[derive(Parser)]
#[clap(name = "test")]
struct TestCli {
    #[clap(subcommand)]
    tool: MoveTool,
}

/// Output from a CLI test invocation including captured stderr.
pub struct CliOutput {
    pub result: CliResult,
    pub stderr: String,
}

/// Parse CLI args, execute the command with a buffer-backed `MoveEnv`, and
/// return the `CliResult` together with captured compiler output.
pub fn run_cli(args: &[&str]) -> CliOutput {
    let (writer, buffer) = DiagWriter::new_buffer();
    run_cli_with_env(args, Arc::new(MoveEnv::default_with_writer(writer)), buffer)
}

/// Same as [`run_cli`] but with a custom `MoveEnv` and buffer handle.
pub fn run_cli_with_env(args: &[&str], env: Arc<MoveEnv>, buffer: Arc<Mutex<Buffer>>) -> CliOutput {
    INIT_HOOKS.call_once(crate::register_package_hooks);

    let cli = TestCli::try_parse_from(std::iter::once("test").chain(args.iter().copied()));
    let cli = match cli {
        Ok(c) => c,
        Err(e) => {
            return CliOutput {
                result: Err(format!("{}", e)),
                stderr: String::new(),
            }
        },
    };

    let runtime = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .expect("failed to build tokio runtime");

    let result = runtime.block_on(cli.tool.execute(env.clone()));
    let stderr =
        String::from_utf8_lossy(buffer.lock().unwrap_or_else(|e| e.into_inner()).as_slice())
            .to_string();
    CliOutput { result, stderr }
}

/// Sanitize CLI output for baseline comparison.
///
/// Replaces any absolute paths that look like temp directories or the package
/// build directory with a stable placeholder. Also strips ANSI escape codes
/// and non-deterministic log lines.
pub fn sanitize_output(s: &str) -> String {
    // Strip ANSI escape codes (colors, bold, etc.)
    let re_ansi = Regex::new(r"\x1b\[[0-9;]*m").expect("regex");
    let s = re_ansi.replace_all(s, "");

    // Remove non-deterministic [INFO] log lines (emitted only when a logger
    // happens to be initialized).
    let re_info = Regex::new(r"(?m)^\[INFO\].*\n?").expect("regex");
    let s = re_info.replace_all(&s, "");

    // Remove "Compiling, may take a little while..." progress line (non-deterministic
    // depending on whether compilation actually fetches git deps).
    let re_compiling = Regex::new(r"(?m)^Compiling, may take a little while.*\n?").expect("regex");
    let s = re_compiling.replace_all(&s, "");

    // Replace temp-dir-style paths: /tmp/..., /var/..., /private/var/...
    let re_tmp = Regex::new(r#"(/private)?(/var|/tmp)(/[^\s,\]]+)*/"#).expect("regex");
    let s = re_tmp.replace_all(&s, "<TEMPDIR>/");

    // Replace CARGO_MANIFEST_DIR-relative absolute paths (macOS / Linux)
    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    let s = s.replace(manifest_dir, "<CLI_DIR>");

    // Replace generic home-dir paths that may appear in Move.toml resolution
    let re_home = Regex::new(r#"/Users/[^\s/]+/"#).expect("regex");
    let s = re_home.replace_all(&s, "<HOME>/");

    // Replace non-deterministic prover counterexample values.
    // Lines like `  =         a = 6334` become `  =         a = <val>`.
    let re_cex = Regex::new(r"(?m)^(  =\s+\w+ = )\d+$").expect("regex");
    let s = re_cex.replace_all(&s, "${1}<val>");

    s.to_string()
}

/// Compute the `.exp` baseline path for a test source file.
///
/// Pass `file!()` from the test module — this returns the sibling `.exp` path.
/// `file!()` returns a path relative to the workspace root (e.g.
/// `aptos-move/cli/src/tests/compile/success.rs`), so we resolve from the
/// workspace root rather than `CARGO_MANIFEST_DIR`.
pub fn exp_path(test_file: &str) -> PathBuf {
    // CARGO_MANIFEST_DIR is <workspace>/aptos-move/cli — go up to workspace root
    let workspace_root = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .expect("workspace root must exist");
    workspace_root.join(test_file).with_extension("exp")
}

/// Compare combined stderr + result against the `.exp` baseline file next to
/// the given test source.  When the `UB` (or `UPBL`) env var is set, the
/// baseline is updated instead.
pub fn check_baseline(test_file: &str, output: &CliOutput) {
    let baseline = exp_path(test_file);
    let mut combined = String::new();
    if !output.stderr.is_empty() {
        combined.push_str(&output.stderr);
        if !combined.ends_with('\n') {
            combined.push('\n');
        }
    }
    match &output.result {
        Ok(s) => combined.push_str(s),
        Err(s) => combined.push_str(s),
    }
    let sanitized = sanitize_output(&combined);
    verify_or_update_baseline(&baseline, &sanitized)
        .unwrap_or_else(|e| panic!("baseline mismatch for {}: {}", test_file, e));
}

/// Build a temporary Move package with the given sources.
///
/// No framework dependency is added by default — the test sources should be
/// self-contained. Use [`make_package_with_framework`] when the Move code
/// imports from `AptosFramework` / `AptosStdlib` / `MoveStdlib`.
pub fn make_package(name: &str, sources: &[(&str, &str)]) -> TempDir {
    let mut builder = PackageBuilder::new(name);
    builder.add_alias(name, "0xCAFE");
    for (file_name, source) in sources {
        builder.add_source(file_name, source);
    }
    builder
        .write_to_temp()
        .expect("failed to create temp package")
}

/// Like [`make_package`] but adds a local `AptosFramework` dependency.
#[allow(dead_code)]
pub fn make_package_with_framework(name: &str, sources: &[(&str, &str)]) -> TempDir {
    let mut builder = PackageBuilder::new(name);
    builder.add_local_dep("AptosFramework", &aptos_framework_path());
    builder.add_alias(name, "0xCAFE");
    for (file_name, source) in sources {
        builder.add_source(file_name, source);
    }
    builder
        .write_to_temp()
        .expect("failed to create temp package")
}

/// Build a `MoveEnv` with a mock `AptosContext` for testing network commands.
///
/// The provided closure configures expectations on the mock before it is
/// sealed inside the env. Returns both the env and a buffer handle for
/// reading captured output.
pub fn env_with_mock(
    setup: impl FnOnce(&mut mock::MockAptosCtx),
) -> (Arc<MoveEnv>, Arc<Mutex<Buffer>>) {
    let mut ctx = mock::MockAptosCtx::new();
    setup(&mut ctx);
    let debugger_factory: Box<
        dyn Fn(aptos_rest_client::Client) -> anyhow::Result<Box<dyn crate::MoveDebugger>>
            + Send
            + Sync,
    > = Box::new(|_| Err(anyhow::anyhow!("debugger not available in tests")));
    let (writer, buffer) = DiagWriter::new_buffer();
    (
        Arc::new(MoveEnv::new_with_writer(
            Box::new(ctx),
            debugger_factory,
            writer,
        )),
        buffer,
    )
}

/// Return the local path to the `aptos-framework` package, derived from
/// `CARGO_MANIFEST_DIR` (i.e. `aptos-move/cli/`).
pub fn aptos_framework_path() -> String {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../aptos-move/framework/aptos-framework")
        .canonicalize()
        .expect("aptos-framework dir must exist")
        .display()
        .to_string()
}
