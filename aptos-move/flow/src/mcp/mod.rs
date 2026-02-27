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
    move_compiler_v2::logging::setup_logging(None);
    let session = FlowSession::new(args.clone(), global.clone());
    let service = session.serve(stdio()).await?;
    service.waiting().await?;
    Ok(())
}
