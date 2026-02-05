// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! MCP server for AI agent integration with Move packages.
//!
//! This module provides CLI access to the Move Query MCP server, which allows
//! AI agents (Claude Code, Cursor, etc.) to programmatically inspect Move packages.
//!
//! # Usage
//! ```bash
//! aptos move query serve
//! ```
//!
//! The enum structure allows future subcommands if needed.

use crate::common::types::{CliCommand, CliError, CliResult, CliTypedResult};
use async_trait::async_trait;
use clap::{Parser, Subcommand};

/// Start MCP server for AI agent integration.
///
/// The server starts with no model loaded. Use the `build_model` MCP tool to
/// load a Move package, then query the cached model with other tools.
///
/// See `third_party/move/tools/move-query/src/mcp/server.rs` for available tools.
#[derive(Parser)]
pub struct Serve {}

#[async_trait]
impl CliCommand<String> for Serve {
    fn command_name(&self) -> &'static str {
        "Serve"
    }

    async fn execute(self) -> CliTypedResult<String> {
        move_query::mcp::run_server()
            .map_err(|e| CliError::UnexpectedError(format!("MCP server error: {}", e)))?;

        Ok("Server stopped".to_string())
    }
}

/// Query tool for inspecting Move packages via MCP.
///
/// Currently only supports `serve` subcommand. The enum structure allows
/// future extensions without breaking changes.
#[derive(Subcommand)]
pub enum QueryTool {
    /// Start MCP server
    Serve(Serve),
}

impl QueryTool {
    pub async fn execute(self) -> CliResult {
        match self {
            QueryTool::Serve(cmd) => cmd.execute_serialized().await,
        }
    }
}
