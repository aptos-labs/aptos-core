// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use crate::{CliResult, Tool};
use clap::{error::ErrorKind, CommandFactory, Parser};
use std::collections::HashSet;

/// Run a CLI invocation. For `--help` invocations, only parse (do not execute).
pub async fn run_cmd(args: &[&str]) -> CliResult {
    if args.last() == Some(&"--help") {
        return match Tool::try_parse_from(args) {
            Ok(_) => Ok(String::new()),
            Err(err) if err.kind() == ErrorKind::DisplayHelp => Ok(String::new()),
            Err(err) => Err(err.to_string()),
        };
    }

    let tool: Tool = Tool::try_parse_from(args).map_err(|msg| msg.to_string())?;
    tool.execute().await
}

/// Assert that a CLI invocation does not panic (duplicate clap args, etc.).
pub async fn assert_cmd_not_panic(args: &[&str]) {
    match run_cmd(args).await {
        Ok(inner) => assert!(
            !inner.contains("panic"),
            "Command should not panic: {}: {}",
            args.join(" "),
            inner
        ),
        Err(inner) => assert!(
            !inner.contains("panic"),
            "Command should not panic: {}: {}",
            args.join(" "),
            inner
        ),
    }
}

/// Collect CLI invocations to exercise help / group listings for every visible subcommand.
pub fn collect_command_invocations() -> Vec<Vec<String>> {
    let mut invocations: Vec<Vec<String>> = Vec::new();
    let mut seen = HashSet::new();
    walk_command(
        &Tool::command().disable_help_flag(true),
        vec!["aptos".to_string()],
        &mut invocations,
        &mut seen,
    );
    invocations
}

fn walk_command(
    cmd: &clap::Command,
    path: Vec<String>,
    invocations: &mut Vec<Vec<String>>,
    seen: &mut HashSet<Vec<String>>,
) {
    let subcommands: Vec<_> = cmd
        .get_subcommands()
        .filter(|sub| !sub.is_hide_set())
        .collect();

    if subcommands.is_empty() {
        // Leaf commands: only exercise `--help` to avoid starting servers or network I/O.
        if path.len() > 1 {
            let mut help_path = path.clone();
            help_path.push("--help".to_string());
            push_invocation(&help_path, invocations, seen);
        }
        if is_safe_to_execute_leaf(&path) {
            push_invocation(&path, invocations, seen);
        }
        return;
    }

    // Subcommand groups: listing help for the group is safe.
    push_invocation(&path, invocations, seen);

    for sub in subcommands {
        let mut sub_path = path.clone();
        sub_path.push(sub.get_name().to_string());
        walk_command(sub, sub_path, invocations, seen);
    }
}

/// Leaf commands that are safe to execute in tests (read-only, no network servers).
fn is_safe_to_execute_leaf(path: &[String]) -> bool {
    let path: Vec<&str> = path.iter().map(String::as_str).collect();
    path == ["aptos", "info"] || path == ["aptos", "config", "show-global-config"]
}

fn push_invocation(
    path: &[String],
    invocations: &mut Vec<Vec<String>>,
    seen: &mut HashSet<Vec<String>>,
) {
    if seen.insert(path.to_vec()) {
        invocations.push(path.to_vec());
    }
}
