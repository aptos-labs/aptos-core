// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

pub(crate) mod common;
pub(crate) mod file_watcher;
pub(crate) mod package_data;
pub(crate) mod session;
pub(crate) mod supervisor;
pub(crate) mod telemetry;
pub(crate) mod tools;

use crate::{
    evaluation::{sha256_hex, EXPECTED_TOOL_LIST_SHA256_ENV_VAR},
    GlobalOpts,
};
use anyhow::Result;
use aptos_framework::UPGRADE_POLICY_CUSTOM_FIELD;
use clap::Parser;
use legacy_move_compiler::shared::{parse_named_address, NumericalAddress};
use move_model::metadata::LanguageVersion;
use move_package::package_hooks::{register_package_hooks, PackageHooks};
use move_symbol_pool::Symbol;
use rmcp::{transport::stdio, ServiceExt};
use session::FlowSession;
use std::{path::PathBuf, sync::Once};
use telemetry::Telemetry;

pub const TELEMETRY_JSONL_ENV_VAR: &str = "MOVE_FLOW_TELEMETRY_JSONL";

/// Package hooks for move-flow MCP server.
/// Registers Aptos-specific custom fields to suppress unknown field warnings.
struct MoveFlowPackageHooks;

impl PackageHooks for MoveFlowPackageHooks {
    fn custom_package_info_fields(&self) -> Vec<String> {
        vec![UPGRADE_POLICY_CUSTOM_FIELD.to_string()]
    }

    fn custom_dependency_key(&self) -> Option<String> {
        None
    }

    fn resolve_custom_dependency(
        &self,
        _dep_name: Symbol,
        _info: &move_package::source_package::parsed_manifest::CustomDepInfo,
    ) -> anyhow::Result<()> {
        anyhow::bail!("custom dependencies are not supported in move-flow")
    }
}

/// Arguments for the `mcp` subcommand.
#[derive(Parser, Debug, Clone)]
pub struct McpArgs {
    /// Build in dev mode (enables dev-only dependencies and addresses).
    #[arg(long, default_value_t = true, action = clap::ArgAction::Set)]
    pub dev_mode: bool,

    /// Additional named addresses in the form `name=0xADDR`.
    #[arg(long = "named-addresses", value_parser = parse_named_address, num_args = 0..)]
    pub named_addresses: Vec<(String, NumericalAddress)>,

    /// Only compile the specified target module or script.
    #[arg(long)]
    pub target_filter: Option<String>,

    /// Bytecode version to use for compilation.
    #[arg(long)]
    pub bytecode_version: Option<u32>,

    /// Move language version.
    #[arg(long, value_parser = clap::value_parser!(LanguageVersion), default_value_t = LanguageVersion::latest())]
    pub language_version: LanguageVersion,

    /// Compiler experiments to enable.
    #[arg(long)]
    pub experiments: Vec<String>,

    /// Global timeout (seconds) for any single MCP tool call. Default: 120.
    #[arg(long, default_value_t = 120)]
    pub tool_timeout: u64,

    /// Append structured experiment telemetry to this JSONL file.
    #[arg(long)]
    pub telemetry_jsonl: Option<PathBuf>,

    /// Candidate acceptance criteria for an evaluation task.
    ///
    /// The experiment controller writes this file outside the agent's writable
    /// workspace; it names the pristine baseline, the target, the editable
    /// paths, and the required contract categories.
    #[arg(long)]
    pub candidate_check: Option<PathBuf>,
}

impl McpArgs {
    fn telemetry_path(&self) -> Option<PathBuf> {
        self.telemetry_jsonl
            .clone()
            .or_else(|| std::env::var_os(TELEMETRY_JSONL_ENV_VAR).map(PathBuf::from))
    }
}

pub(crate) fn register_move_flow_package_hooks() {
    static REGISTER: Once = Once::new();
    REGISTER.call_once(|| register_package_hooks(Box::new(MoveFlowPackageHooks)));
}

fn setup() {
    move_compiler_v2::logging::setup_logging_with_timestamps(None);

    // Register Aptos package hooks to recognize custom fields like upgrade_policy.
    register_move_flow_package_hooks();

    // Bridge `tracing` events (used by rmcp) into the `log` framework so that
    // flexi_logger captures transport-level diagnostics (e.g. "input stream
    // terminated").
    let _ = tracing_log::LogTracer::init();

    // Install a panic hook that logs panics before the default handler runs.
    // This captures panics from any thread (file-watcher, spawn_blocking) in the
    // log file with location info rather than silently crashing the process.
    let default_hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        let bt = std::backtrace::Backtrace::force_capture();
        log::error!("panic: {}\nBacktrace:\n{}", info, bt);
        default_hook(info);
    }));
}

/// Start the MCP stdio server.
///
/// When `restart` is true, skips the MCP `initialize` handshake because the
/// client already completed it with the previous child process. After a crash
/// the client is waiting for a response, not sending `initialize`, so a new
/// handshake would deadlock.
pub async fn run(args: &McpArgs, global: &GlobalOpts, restart: bool) -> Result<()> {
    setup();

    let evaluation = global.evaluation_config()?;
    evaluation.validate_expected()?;
    let mut tool_names = FlowSession::tool_names(evaluation);
    tool_names.sort();
    if let Some(expected_hash) = std::env::var_os(EXPECTED_TOOL_LIST_SHA256_ENV_VAR) {
        let expected_hash = expected_hash.into_string().map_err(|_| {
            anyhow::anyhow!("{EXPECTED_TOOL_LIST_SHA256_ENV_VAR} is not valid UTF-8")
        })?;
        let actual_hash = sha256_hex(tool_names.join("\n").as_bytes());
        anyhow::ensure!(
            expected_hash == actual_hash,
            "MCP tool inventory mismatch: generated plugin expects `{expected_hash}`, \
             runtime produced `{actual_hash}`"
        );
    }
    let telemetry_path = args.telemetry_path();
    let telemetry = Telemetry::new(telemetry_path.as_deref(), evaluation)?;

    log::info!(
        "move-flow MCP server v{} starting (tactic: {}, evaluation_mode: {}, tools: {})",
        env!("CARGO_PKG_VERSION"),
        evaluation.inference_tactic,
        evaluation.evaluation_mode,
        tool_names.join(", ")
    );
    // Record the configuration the server actually resolved. A flag lost on the
    // way in is otherwise indistinguishable from one whose default happens to
    // match what the caller expected, which has already hidden a whole
    // evaluation round's treatment.
    telemetry.emit(
        "session_start",
        serde_json::json!({
            "restart": restart,
            "feedback_level": evaluation.feedback_level.to_string(),
            "candidate_check_configured": args.candidate_check.is_some(),
        }),
    );

    let session = FlowSession::new(args.clone(), global.clone(), evaluation, telemetry.clone());
    let result = if restart {
        let service = rmcp::service::serve_directly(session, stdio(), None);
        service.waiting().await
    } else {
        match session.serve(stdio()).await {
            Ok(service) => service.waiting().await,
            Err(error) => {
                telemetry.emit(
                    "session_end",
                    serde_json::json!({"outcome": "startup_error", "error": error.to_string()}),
                );
                return Err(error.into());
            },
        }
    };
    let outcome = if result.is_ok() { "success" } else { "error" };
    telemetry.emit("session_end", serde_json::json!({"outcome": outcome}));
    result?;
    Ok(())
}
