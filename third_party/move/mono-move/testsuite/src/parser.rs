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
//! ## `// RUN: publish [--print(<sections>)]`
//!
//! All non-directive lines following this marker (until the next `// RUN:`
//! directive or EOF) are collected verbatim as Move source text, compiled
//! into one or more modules, and published into the test storage for both
//! VMs. Multiple publish blocks accumulate modules across the test.
//!
//! The optional `--print(<csv>)` modifier requests that an `.exp` snapshot
//! be produced alongside the test file. Recognized sections:
//!
//! - `bytecode`  — Move bytecode disassembly (rejected for `.masm` inputs,
//!                 since the bytecode *is* the input).
//! - `stackless` — stackless execution IR.
//! - `micro-ops` — lowered micro-ops.
//!
//! Sections are emitted in the order above regardless of the order written
//! in `--print(...)`. Unknown tokens or an empty section list cause the
//! test to fail at parse time.
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

/// A snapshot section requested via `// RUN: publish --print(...)`.
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum PrintSection {
    Bytecode,
    Stackless,
    MicroOps,
}

/// A single step in a differential test.
#[derive(Debug)]
pub enum Step {
    Publish {
        sources: String,
        print: Vec<PrintSection>,
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
    let mut publish_print: Option<Vec<PrintSection>> = None;

    for raw_line in content.lines() {
        let line = raw_line.trim();

        if let Some(directive) = line.strip_prefix("// RUN:") {
            let directive = directive.trim();

            if let Some(print) = publish_print.take() {
                steps.push(Step::Publish {
                    sources: sources.join("\n"),
                    print,
                });
                sources.clear();
            }

            if let Some(rest) = directive.strip_prefix("publish") {
                publish_print = Some(parse_publish_modifiers(rest.trim())?);
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
        } else if publish_print.is_some() {
            sources.push(raw_line);
        }
    }

    if let Some(print) = publish_print.take() {
        steps.push(Step::Publish {
            sources: sources.join("\n"),
            print,
        });
    }

    Ok(steps)
}

/// Parse the modifiers after the `publish` keyword. Currently only
/// `--print(<sections>)` is supported.
fn parse_publish_modifiers(rest: &str) -> anyhow::Result<Vec<PrintSection>> {
    if rest.is_empty() {
        return Ok(vec![]);
    }
    let inner = rest
        .strip_prefix("--print(")
        .and_then(|s| s.strip_suffix(')'))
        .ok_or_else(|| anyhow!("Unrecognized publish modifier: {}", rest))?;
    if inner.trim().is_empty() {
        bail!("`--print()` requires at least one section");
    }
    let mut sections = vec![];
    for raw in inner.split(',') {
        let token = raw.trim();
        let section = match token {
            "bytecode" => PrintSection::Bytecode,
            "stackless" => PrintSection::Stackless,
            "micro-ops" => PrintSection::MicroOps,
            _ => bail!("Unknown print section: {:?}", token),
        };
        if sections.contains(&section) {
            bail!("Duplicate print section: {:?}", token);
        }
        sections.push(section);
    }
    Ok(sections)
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
