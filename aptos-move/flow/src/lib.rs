// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

pub mod hooks;
pub mod mcp;
pub mod plugin;

use anyhow::Result;
use clap::{Args, Parser, Subcommand, ValueEnum};
use std::path::PathBuf;

/// MoveFlow: generates AI platform configurations for Move development.
#[derive(Parser, Debug)]
#[command(
    name = "move-flow",
    about = "AI platform config generator for Move development"
)]
pub struct FlowCli {
    #[command(flatten)]
    pub global: GlobalOpts,

    #[command(subcommand)]
    pub command: FlowCommand,
}

/// Global options shared across all subcommands.
#[derive(Args, Debug, Clone, serde::Serialize)]
pub struct GlobalOpts {
    /// Target platform to generate configuration for.
    #[arg(short = 'p', long, value_enum, default_value_t = Platform::Claude, global = true)]
    pub platform: Platform,

    /// Directory containing content templates. Defaults to the crate's
    /// built-in content directory.
    #[arg(long, global = true)]
    pub content_dir: Option<PathBuf>,
}

/// Subcommands.
#[derive(Subcommand, Debug)]
pub enum FlowCommand {
    /// Generate plugin files for an AI platform.
    Plugin(plugin::PluginArgs),
    /// Start an MCP server.
    Mcp(mcp::McpArgs),
    /// Hook subcommands (called from AI platform hooks).
    #[command(subcommand)]
    Hook(hooks::HookCommand),
}

/// Supported AI platform targets.
#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum, serde::Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Platform {
    Claude,
    // Future: Cursor, Codex
}

impl Platform {
    /// Human-readable display name for the platform.
    pub fn display_name(self) -> &'static str {
        match self {
            Platform::Claude => "Claude Code",
        }
    }
}

impl FlowCli {
    pub async fn run(&self) -> Result<()> {
        match &self.command {
            FlowCommand::Plugin(args) => plugin::run(args, &self.global),
            FlowCommand::Mcp(args) => mcp::run(args, &self.global).await,
            FlowCommand::Hook(cmd) => hooks::run(cmd),
        }
    }
}

#[cfg(test)]
mod tests;
