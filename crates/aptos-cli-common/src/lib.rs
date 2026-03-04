// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Common CLI types, configuration, and utilities shared between the full Aptos CLI
//! and the standalone Move CLI.

mod config;
mod options;
mod types;
mod utils;

// Re-export everything for convenient use
pub use config::*;
pub use options::*;
pub use types::*;
pub use utils::*;

// ── CLI styling and shell completions ──

/// A style for the CLI that closely resembles the Clap v3 color scheme
pub fn aptos_cli_style() -> clap::builder::Styles {
    use anstyle::{AnsiColor, Color::Ansi, Style};
    use clap::builder::Styles;

    Styles::styled()
        // Help headers
        // To test: `aptos help`
        .header(Style::new().bold().fg_color(Some(Ansi(AnsiColor::Yellow))))
        // The word Usage, which should match the help headers for consistency
        // To test: `aptos help` and `aptos account create`
        .usage(Style::new().bold().fg_color(Some(Ansi(AnsiColor::Yellow))))
        // Most literals like command names and other pieces
        // To test: `aptos help` and `aptos account create`
        .literal(Style::new().fg_color(Some(Ansi(AnsiColor::Green))))
        // The word error when an error occurs
        // This is listed as "bright red" to help with red / green colorblindness
        // To test: `aptos account create`
        .error(Style::new().fg_color(Some(Ansi(AnsiColor::BrightRed))))
        // Placeholder eg. <ACCOUNT>
        // To test: `aptos account create` or `aptos account create --help`
        .placeholder(Style::new().fg_color(Some(Ansi(AnsiColor::Green))))
        // Valid when providing help for missing arguments
        // To test: `aptos account create`
        .valid(Style::new().fg_color(Some(Ansi(AnsiColor::Green))))
        // Invalid value during parsing
        // To test: `aptos account create --account not-a-number`
        .invalid(Style::new().fg_color(Some(Ansi(AnsiColor::Yellow))))
}

/// Easy way to add CLI completions
pub fn generate_cli_completions<Tool: clap::CommandFactory>(
    tool_name: &str,
    shell: clap_complete::shells::Shell,
    output_file: &std::path::Path,
) -> std::io::Result<()> {
    let mut command = Tool::command();
    let mut file = std::fs::File::create(output_file)?;
    clap_complete::generate(shell, &mut command, tool_name, &mut file);
    Ok(())
}

// ── Pluggable telemetry ──

use std::sync::OnceLock;

/// Telemetry callback for reporting CLI command metrics.
pub trait TelemetryCallback: Send + Sync + 'static {
    fn send_event(&self, command_name: &str, latency_secs: f64, success: bool);
    fn is_disabled(&self) -> bool;
}

static TELEMETRY_CALLBACK: OnceLock<Box<dyn TelemetryCallback>> = OnceLock::new();

/// Register a telemetry callback. Called once at startup by the full Aptos CLI.
pub fn register_telemetry(callback: Box<dyn TelemetryCallback>) {
    let _ = TELEMETRY_CALLBACK.set(callback);
}

/// Get the registered telemetry callback, if any.
pub fn telemetry_callback() -> Option<&'static dyn TelemetryCallback> {
    TELEMETRY_CALLBACK.get().map(|c| c.as_ref())
}
