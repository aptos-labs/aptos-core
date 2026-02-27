// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

pub(crate) mod file_watcher;
mod package_data;
pub(crate) mod session;
mod tools;

use crate::GlobalOpts;
use anyhow::Result;
use clap::Parser;
use legacy_move_compiler::shared::{parse_named_address, NumericalAddress};
use move_model::metadata::LanguageVersion;
use rmcp::{transport::stdio, ServiceExt};
use session::FlowSession;

/// Arguments for the `mcp` subcommand.
#[derive(Parser, Debug, Clone)]
pub struct McpArgs {
    /// Build in dev mode (enables dev-only dependencies and addresses).
    #[arg(long)]
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
    #[arg(long, value_parser = clap::value_parser!(LanguageVersion))]
    pub language_version: Option<LanguageVersion>,

    /// Compiler experiments to enable.
    #[arg(long)]
    pub experiments: Vec<String>,
}

/// Start the MCP stdio server.
pub async fn run(args: &McpArgs, global: &GlobalOpts) -> Result<()> {
    move_compiler_v2::logging::setup_logging_with_timestamps(None);

    // Bridge `tracing` events (used by rmcp) into the `log` framework so that
    // flexi_logger captures transport-level diagnostics (e.g. "input stream
    // terminated") in /tmp/flow.err.log.
    let _ = tracing_log::LogTracer::init();

    // Install a panic hook that logs panics before the default handler runs.
    // This captures panics from any thread (file-watcher, spawn_blocking) in the
    // log file with location info rather than silently crashing the process.
    let default_hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        log::error!("panic: {}", info);
        default_hook(info);
    }));

    log::info!(
        "move-flow MCP server v{} starting (tools: {})",
        env!("CARGO_PKG_VERSION"),
        FlowSession::tool_names().join(", ")
    );

    let session = FlowSession::new(args.clone(), global.clone());
    let service = session.serve(stdio()).await?;
    service.waiting().await?;
    Ok(())
}
