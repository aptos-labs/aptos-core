// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

pub mod package_path;
pub mod source_check;

use anyhow::Result;
use clap::Subcommand;

/// Hook subcommands (called from AI platform hooks).
#[derive(Subcommand, Debug)]
pub enum HookCommand {
    /// PostToolUse hook for Edit/Write operations on Move files.
    Edit,
    /// UserPromptSubmit hook that detects the current Move package.
    PackagePath,
}

pub fn run(cmd: &HookCommand) -> Result<()> {
    match cmd {
        HookCommand::Edit => source_check::run(),
        HookCommand::PackagePath => package_path::run(),
    }
}
