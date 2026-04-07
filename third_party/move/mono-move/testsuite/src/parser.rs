// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Parser for differential test files.
//!
//! # Directive semantics
//!
//! Directives are embedded as inline comments (`// DIRECTIVE: ...`).
//! A test file is a sequence of **publish** and **execute** steps processed
//! in order. Each step may have **check** directives attached.
//!
//! ## `// RUN: publish`
//!
//! All non-directive lines following this marker (until the next `// RUN:`
//! directive or EOF) are collected verbatim as Move source text, compiled
//! into one or more modules, and published into the test storage for both
//! VMs. Multiple publish blocks accumulate modules across the test.
//!
//! ## `// RUN: execute <addr>::<module>::<func> [--args <v1>, <v2>, ...]`
//!
//! Invokes `<func>` in the given module on both VMs. Arguments are
//! comma-separated literal values (currently only `u64` is supported).
//! The execution produces a result string of the form `results: v1, v2`
//! on success, or `error: <message>` on failure (e.g., abort).
//!
//! ## `// CHECK: <literal>`
//!
//! Must immediately follow an execute directive. The literal string is
//! compared **exactly** (after trimming) against the normalized output of
//! **both** VMs. Use this when V1 and V2 should agree.
//!
//! ## `// CHECK-V1: <literal>` / `// CHECK-V2: <literal>`
//!
//! Like `CHECK`, but applies to only the legacy Move VM (V1) or MonoMove
//! VM (V2) respectively. Use these when the two VMs intentionally diverge
//! (e.g., different error messages for aborts).
//!
//! Multiple check directives may follow a single execute step; each is
//! verified independently.
//!
//! # Future extensions
//!
//! - Regex or substring matching for CHECK patterns.
//! - Abort-specific checks (e.g., `// CHECK: error: ... ABORTED ...`).

use anyhow::{anyhow, bail};
use move_core_types::{account_address::AccountAddress, identifier::Identifier};

/// A check directive attached to an execution step.
#[derive(Debug)]
pub enum Check {
    /// Legacy Move VM should produce this output.
    V1(String),
    /// MonoMove VM should produce this output.
    V2(String),
}

/// A single step in a differential test.
#[derive(Debug)]
pub enum Step {
    Publish {
        sources: String,
    },
    Execute {
        address: AccountAddress,
        module_name: Identifier,
        function_name: Identifier,
        args: Vec<String>,
        checks: Vec<Check>,
    },
}

/// Parse a `.move` test file into a sequence of steps.
pub fn parse(content: &str) -> anyhow::Result<Vec<Step>> {
    let mut steps = vec![];
    let mut sources = vec![];
    let mut in_publish = false;

    for line in content.lines() {
        let line = line.trim();

        if let Some(directive) = line.strip_prefix("// RUN:") {
            let directive = directive.trim();

            if in_publish {
                steps.push(Step::Publish {
                    sources: sources.join("\n"),
                });
                sources.clear();
                in_publish = false;
            }

            if directive == "publish" {
                in_publish = true;
            } else if let Some(rest) = directive.strip_prefix("execute") {
                let rest = rest.trim();
                let step = parse_execute_step(rest)
                    .map_err(|err| anyhow!("Failed to parse execute step: {}", err))?;
                steps.push(step);
            } else {
                bail!("Unknown RUN directive: {}", directive);
            }
        } else if let Some(pattern) = line.strip_prefix("// CHECK-V1:") {
            attach_check(&mut steps, Check::V1(pattern.trim().to_string()))?;
        } else if let Some(pattern) = line.strip_prefix("// CHECK-V2:") {
            attach_check(&mut steps, Check::V2(pattern.trim().to_string()))?;
        } else if let Some(pattern) = line.strip_prefix("// CHECK:") {
            let pattern = pattern.trim().to_string();
            attach_check(&mut steps, Check::V1(pattern.clone()))?;
            attach_check(&mut steps, Check::V2(pattern))?;
        } else if in_publish {
            sources.push(line);
        }
    }

    if in_publish {
        steps.push(Step::Publish {
            sources: sources.join("\n"),
        });
    }

    Ok(steps)
}

/// Parses execution step.
fn parse_execute_step(line: &str) -> anyhow::Result<Step> {
    let (id, args) = match line.split_once("--args") {
        None => (line.trim(), vec![]),
        Some((id, args)) => {
            let args: Vec<String> = args
                .split(',')
                .map(|arg| arg.trim().to_string())
                .filter(|arg| !arg.is_empty())
                .collect();
            (id.trim(), args)
        },
    };

    let id = id.split("::").collect::<Vec<_>>();
    if id.len() != 3 {
        bail!("Expected <addr>::<module>::<func>, got {}", id.join("::"));
    }

    let address = AccountAddress::from_hex_literal(id[0])
        .map_err(|err| anyhow!("Failed to parse address: {}", err))?;
    let module_name =
        Identifier::new(id[1]).map_err(|err| anyhow!("Failed to parse module name: {}", err))?;
    let function_name =
        Identifier::new(id[2]).map_err(|err| anyhow!("Failed to parse function name: {}", err))?;

    Ok(Step::Execute {
        address,
        module_name,
        function_name,
        args,
        checks: vec![],
    })
}

/// Attach a check to the last execution step.
fn attach_check(steps: &mut [Step], check: Check) -> anyhow::Result<()> {
    match steps.last_mut() {
        Some(Step::Execute { checks, .. }) => {
            checks.push(check);
            Ok(())
        },
        _ => bail!("CHECK directive must follow // RUN: execute"),
    }
}
