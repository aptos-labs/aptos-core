// Copyright © Velor Foundation
// SPDX-License-Identifier: Apache-2.0

/// A style for the CLI that closely resembles the Clap v3 color scheme
pub fn velor_cli_style() -> clap::builder::Styles {
    use anstyle::{AnsiColor, Color::Ansi, Style};
    use clap::builder::Styles;

    Styles::styled()
        // Help headers
        // To test: `velor help`
        .header(Style::new().bold().fg_color(Some(Ansi(AnsiColor::Yellow))))
        // The word Usage, which should match the help headers for consistency
        // To test: `velor help` and `velor account create`
        .usage(Style::new().bold().fg_color(Some(Ansi(AnsiColor::Yellow))))
        // Most literals like command names and other pieces
        // To test: `velor help` and `velor account create`
        .literal(Style::new().fg_color(Some(Ansi(AnsiColor::Green))))
        // The word error when an error occurs
        // This is listed as "bright red" to help with red / green colorblindness
        // To test: `velor account create`
        .error(Style::new().fg_color(Some(Ansi(AnsiColor::BrightRed))))
        // Placeholder eg. <ACCOUNT>
        // To test: `velor account create` or `velor account create --help`
        .placeholder(Style::new().fg_color(Some(Ansi(AnsiColor::Green))))
        // Valid when providing help for missing arguments
        // To test: `velor account create`
        .valid(Style::new().fg_color(Some(Ansi(AnsiColor::Green))))
        // Invalid value during parsing
        // To test: `velor account create --account not-a-number`
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
