// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Shared test harness for MCP server integration tests.
//!
//! Provides helpers to create temporary Move packages, connect an in-process
//! MCP client/server pair via `tokio::io::duplex`, and compare tool output
//! against `.exp` baseline files.

use crate::{
    evaluation::{EvaluationConfig, FeedbackLevel, InferenceTactic},
    mcp::{session::FlowSession, telemetry::Telemetry, McpArgs},
    GlobalOpts, Platform,
};
use aptos_package_builder::PackageBuilder;
use move_model::metadata::LanguageVersion;
use move_prover_test_utils::baseline_test::verify_or_update_baseline;
use regex::Regex;
use rmcp::{
    model::{CallToolRequestParams, CallToolResult, ListToolsResult, RawContent},
    ServiceError, ServiceExt,
};
use std::path::{Path, PathBuf};
use tempfile::TempDir;
use tokio::sync::Mutex as AsyncMutex;

/// Lock to serialize tests that invoke the Move test runner.
/// `LOGGING_FILE_WRITER` in the Move VM is a global static that would be
/// corrupted by concurrent test executions.
static TEST_RUNNER_LOCK: AsyncMutex<()> = AsyncMutex::const_new(());

/// Acquire the test runner lock. Hold the returned guard for the duration of
/// any test that triggers `move_package_test` or `move_package_coverage`.
pub async fn serial_test_lock() -> tokio::sync::MutexGuard<'static, ()> {
    TEST_RUNNER_LOCK.lock().await
}

/// Build a temporary Move package with the given sources.
///
/// Adds a `name = "0xCAFE"` alias; no framework dependency.
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

/// Create an in-process MCP client connected to a `FlowSession` server.
///
/// Uses `tokio::io::duplex` for an in-memory transport pair; the server runs
/// as a background tokio task.
pub async fn make_client() -> rmcp::service::RunningService<rmcp::RoleClient, ()> {
    make_client_for_tactic(InferenceTactic::HybridGuided).await
}

/// Create an in-process MCP client with a specific inference tactic.
pub async fn make_client_for_tactic(
    tactic: InferenceTactic,
) -> rmcp::service::RunningService<rmcp::RoleClient, ()> {
    make_client_for_config(tactic, None).await
}

/// Create an in-process MCP client which writes tool telemetry to `path`.
pub async fn make_client_with_telemetry(
    tactic: InferenceTactic,
    path: &Path,
) -> rmcp::service::RunningService<rmcp::RoleClient, ()> {
    make_client_for_config(tactic, Some(path)).await
}

/// Create an evaluation-mode client whose candidate check reads `config`.
pub async fn make_evaluation_client(
    tactic: InferenceTactic,
    candidate_check: &Path,
) -> rmcp::service::RunningService<rmcp::RoleClient, ()> {
    make_evaluation_client_at_level(tactic, candidate_check, FeedbackLevel::Acceptance).await
}

/// Create an evaluation-mode client at a specific feedback level, with a
/// candidate check configured.
pub async fn make_evaluation_client_at_level(
    tactic: InferenceTactic,
    candidate_check: &Path,
    feedback_level: FeedbackLevel,
) -> rmcp::service::RunningService<rmcp::RoleClient, ()> {
    make_session_client(
        tactic,
        None,
        Some(candidate_check.to_path_buf()),
        true,
        feedback_level,
    )
    .await
}

/// Create a client at a specific feedback level.
pub async fn make_client_at_level(
    tactic: InferenceTactic,
    feedback_level: FeedbackLevel,
) -> rmcp::service::RunningService<rmcp::RoleClient, ()> {
    make_session_client(tactic, None, None, false, feedback_level).await
}

async fn make_client_for_config(
    tactic: InferenceTactic,
    telemetry_path: Option<&Path>,
) -> rmcp::service::RunningService<rmcp::RoleClient, ()> {
    make_session_client(
        tactic,
        telemetry_path,
        None,
        false,
        FeedbackLevel::Acceptance,
    )
    .await
}

async fn make_session_client(
    tactic: InferenceTactic,
    telemetry_path: Option<&Path>,
    candidate_check: Option<PathBuf>,
    evaluation_mode: bool,
    feedback_level: FeedbackLevel,
) -> rmcp::service::RunningService<rmcp::RoleClient, ()> {
    // Suppress movefmt so baselines are deterministic across platforms.
    // SAFETY: test-only; each test process is single-threaded at this point.
    unsafe { std::env::set_var("MOVE_FLOW_NO_FMT", "1") };

    let args = McpArgs {
        candidate_check,
        dev_mode: false,
        named_addresses: vec![],
        target_filter: None,
        bytecode_version: None,
        language_version: LanguageVersion::latest(),
        experiments: vec![],
        tool_timeout: 120,
        telemetry_jsonl: None,
    };
    let global = GlobalOpts {
        platform: Platform::Claude,
        content_dir: None,
        inference_tactic: Some(tactic),
        evaluation_mode,
        feedback_level: None,
    };
    let evaluation = EvaluationConfig {
        inference_tactic: tactic,
        evaluation_mode,
        feedback_level,
    };

    let (client_half, server_half) = tokio::io::duplex(8192);

    let telemetry = match telemetry_path {
        Some(path) => Telemetry::new(Some(path), evaluation).expect("create telemetry"),
        None => Telemetry::disabled(),
    };
    let session = FlowSession::new(args, global, evaluation, telemetry);
    let (server_read, server_write) = tokio::io::split(server_half);
    tokio::spawn(async move {
        let service = session
            .serve((server_read, server_write))
            .await
            .expect("server serve");
        service.waiting().await.expect("server waiting");
    });

    let (client_read, client_write) = tokio::io::split(client_half);
    ().serve((client_read, client_write))
        .await
        .expect("client serve")
}

/// Call an MCP tool by name with the given JSON arguments and return the
/// [`CallToolResult`].
pub async fn call_tool(
    client: &rmcp::service::RunningService<rmcp::RoleClient, ()>,
    name: &str,
    args: serde_json::Value,
) -> CallToolResult {
    call_tool_raw(client, name, args)
        .await
        .expect("call_tool RPC failed")
}

/// Call an MCP tool, returning the raw `Result` to allow testing error paths.
pub async fn call_tool_raw(
    client: &rmcp::service::RunningService<rmcp::RoleClient, ()>,
    name: &str,
    args: serde_json::Value,
) -> Result<CallToolResult, rmcp::ServiceError> {
    client
        .call_tool(CallToolRequestParams {
            meta: None,
            name: name.to_owned().into(),
            arguments: args.as_object().cloned(),
            task: None,
        })
        .await
}

/// Format a [`CallToolResult`] into a stable text representation for baseline
/// comparison.
pub fn format_tool_result(result: &CallToolResult) -> String {
    let mut out = String::new();
    if result.is_error == Some(true) {
        out.push_str("is_error: true\n");
    }
    for content in &result.content {
        match &content.raw {
            RawContent::Text(t) => {
                out.push_str(&t.text);
                if !t.text.ends_with('\n') {
                    out.push('\n');
                }
            },
            other => {
                out.push_str(&format!("{:?}\n", other));
            },
        }
    }
    out
}

/// Format a [`ServiceError`] into a stable text representation for baseline
/// comparison.
pub fn format_service_error(err: &ServiceError) -> String {
    format!("rpc_error: {}\n", err)
}

/// Format a [`ListToolsResult`] into a sorted, stable text representation
/// including the full input schema for each tool.
pub fn format_tools_list(result: &ListToolsResult) -> String {
    let mut tools: Vec<_> = result
        .tools
        .iter()
        .map(|t| serde_json::to_value(t).expect("serialize tool"))
        .collect();
    tools.sort_by(|a, b| {
        let na = a.get("name").and_then(|v| v.as_str()).unwrap_or("");
        let nb = b.get("name").and_then(|v| v.as_str()).unwrap_or("");
        na.cmp(nb)
    });
    let arr = serde_json::Value::Array(tools);
    serde_json::to_string_pretty(&arr).expect("pretty print") + "\n"
}

/// Sanitize output for baseline comparison.
///
/// Replaces temp paths, ANSI codes, home directories, and other
/// non-deterministic fragments with stable placeholders.
pub fn sanitize_output(s: &str) -> String {
    // Strip ANSI escape codes
    let re_ansi = Regex::new(r"\x1b\[[0-9;]*m").expect("regex");
    let s = re_ansi.replace_all(s, "");

    // Replace temp-dir-style paths: /tmp/..., /var/..., /private/var/...
    // First pass: paths with trailing `/` — preserves any following filename.
    let re_tmp = Regex::new(r#"(/private)?(/var|/tmp)(/[^\s,\]"`]+)*/"#).expect("regex");
    let s = re_tmp.replace_all(&s, "<TEMPDIR>/");
    // Second pass: collapse bare temp-dir names left after the first pass
    // (e.g. `<TEMPDIR>/.tmpXXXXXX` → `<TEMPDIR>`).
    let re_tmp_bare = Regex::new(r"<TEMPDIR>/\.tmp[a-zA-Z0-9]+").expect("regex");
    let s = re_tmp_bare.replace_all(&s, "<TEMPDIR>");

    // Replace CARGO_MANIFEST_DIR paths
    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    let s = s.replace(manifest_dir, "<FLOW_DIR>");

    // Replace home-dir paths
    let re_home = Regex::new(r#"/Users/[^\s/]+/"#).expect("regex");
    let s = re_home.replace_all(&s, "<HOME>/");

    s.to_string()
}

/// Compute the `.exp` baseline path for a test source file.
///
/// Pass `file!()` from the test module.
pub fn exp_path(test_file: &str) -> PathBuf {
    let workspace_root = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .expect("workspace root must exist");
    workspace_root.join(test_file).with_extension("exp")
}

/// Compare output against the `.exp` baseline file next to the given test
/// source. When `UB` (or `UPBL`) is set, updates the baseline instead.
pub fn check_baseline(test_file: &str, output: &str) {
    let baseline = exp_path(test_file);
    let sanitized = sanitize_output(output);
    verify_or_update_baseline(&baseline, &sanitized)
        .unwrap_or_else(|e| panic!("baseline mismatch for {}: {}", test_file, e));
}
