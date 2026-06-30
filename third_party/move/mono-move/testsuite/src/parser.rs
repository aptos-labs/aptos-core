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
//! ## `// RUN: execute <addr>::<module>::<func> [--args <v1>, <v2>, ...] [--heap-size <n>]`
//!
//! Invokes `<func>` in the given module on both VMs. Arguments are
//! comma-separated decimal literals, parsed according to the function's
//! parameter types (all integer types `u8..u256` / `i8..i256` are
//! supported). The execution produces a result string of the form
//! `results: v1, v2` on success, or `error: <message>` on failure (e.g.,
//! abort).
//!
//! `--heap-size <n>` sets the MonoMove heap to `n` bytes (default otherwise).
//! A small heap forces garbage collection under allocation pressure. The
//! legacy VM has no heap-size knob and ignores it. Modifiers may appear in
//! any order.
//!
//! ## `// CHECK-GC-COUNT: <n>`
//!
//! Must follow an execute directive. Asserts the MonoMove VM ran exactly `n`
//! garbage collections during that step. V2-only — the legacy VM has no GC.
//! Pair with `--heap-size` to drive collections deterministically.
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
//! ## `// CHECK-SUBSTR: <pattern>` / `// CHECK-V1-SUBSTR: <pattern>` / `// CHECK-V2-SUBSTR: <pattern>`
//!
//! Like the exact-match variants above, but the pattern only needs to
//! appear as a substring of the actual output. Use this for abort
//! messages whose exact form includes volatile bits like a function-def
//! index or a code offset.
//!
//! Multiple check directives may follow a single execute step; each is
//! verified independently.
//!
//! # Future extensions
//!
//! - Regex matching for CHECK patterns.

use anyhow::{anyhow, bail};
use move_core_types::{account_address::AccountAddress, identifier::Identifier};

/// How an expected pattern is matched against actual VM output.
#[derive(Debug, Copy, Clone)]
pub enum MatchKind {
    /// The actual output (trimmed) must equal the pattern exactly.
    Exact,
    /// The actual output must contain the pattern as a substring. Useful
    /// for abort messages whose exact form includes volatile bits like a
    /// function-def index.
    Substring,
}

/// A check directive attached to an execution step.
#[derive(Debug)]
pub enum Check {
    /// Legacy Move VM should produce this output.
    V1(String, MatchKind),
    /// MonoMove VM should produce this output.
    V2(String, MatchKind),
    /// MonoMove VM must have run exactly this many garbage collections.
    /// V2-only: the legacy VM has no GC. Pair with `--heap-size` to drive
    /// collections deterministically.
    GcCount(usize),
}

/// A snapshot section requested via `// RUN: publish --print(...)`.
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum PrintSection {
    /// Move bytecode disassembly. Selected by `--print(bytecode)`.
    Bytecode,
    /// Stackless execution IR. Selected by `--print(stackless)`.
    Stackless,
    /// Lowered micro-ops. Selected by `--print(micro-ops)`.
    MicroOps,
    /// Per-function GC frame layout.
    /// Selected by `--print(frame-layout)`.
    FrameLayout,
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
        /// MonoMove heap size in bytes (`--heap-size`). `None` uses the
        /// default. A small heap forces GC under allocation pressure. Has no
        /// effect on the legacy VM.
        heap_size: Option<usize>,
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
        } else if let Some(pattern) = line.strip_prefix("// CHECK-V1-SUBSTR:") {
            attach_check(
                &mut steps,
                Check::V1(pattern.trim().to_string(), MatchKind::Substring),
            )?;
        } else if let Some(pattern) = line.strip_prefix("// CHECK-V2-SUBSTR:") {
            attach_check(
                &mut steps,
                Check::V2(pattern.trim().to_string(), MatchKind::Substring),
            )?;
        } else if let Some(pattern) = line.strip_prefix("// CHECK-SUBSTR:") {
            let pattern = pattern.trim().to_string();
            attach_check(&mut steps, Check::V1(pattern.clone(), MatchKind::Substring))?;
            attach_check(&mut steps, Check::V2(pattern, MatchKind::Substring))?;
        } else if let Some(pattern) = line.strip_prefix("// CHECK-V1:") {
            attach_check(
                &mut steps,
                Check::V1(pattern.trim().to_string(), MatchKind::Exact),
            )?;
        } else if let Some(pattern) = line.strip_prefix("// CHECK-V2:") {
            attach_check(
                &mut steps,
                Check::V2(pattern.trim().to_string(), MatchKind::Exact),
            )?;
        } else if let Some(pattern) = line.strip_prefix("// CHECK:") {
            let pattern = pattern.trim().to_string();
            attach_check(&mut steps, Check::V1(pattern.clone(), MatchKind::Exact))?;
            attach_check(&mut steps, Check::V2(pattern, MatchKind::Exact))?;
        } else if let Some(count) = line.strip_prefix("// CHECK-GC-COUNT:") {
            let count = count.trim();
            let count = count
                .parse::<usize>()
                .map_err(|err| anyhow!("invalid CHECK-GC-COUNT {:?}: {}", count, err))?;
            attach_check(&mut steps, Check::GcCount(count))?;
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
            "frame-layout" => PrintSection::FrameLayout,
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
    // The function id comes first; `--flag` modifiers follow in any order.
    let (id, modifiers) = match line.find("--") {
        Some(i) => (line[..i].trim(), line[i..].trim()),
        None => (line.trim(), ""),
    };

    let mut args = vec![];
    let mut heap_size = None;
    // Each modifier is `--<flag> <value>`. Split on the ` --` boundary (not bare
    // `--`) so a value containing `--` is not shredded, and match the flag name
    // as a whole token so a typo like `--heap-size9` is rejected rather than
    // silently parsed as `--heap-size 9`.
    if let Some(rest) = modifiers.strip_prefix("--") {
        for group in rest.split(" --").map(str::trim).filter(|g| !g.is_empty()) {
            let (flag, value) = match group.split_once(char::is_whitespace) {
                Some((flag, value)) => (flag, value.trim()),
                None => (group, ""),
            };
            match flag {
                "args" => {
                    args = value
                        .split(',')
                        .map(|arg| arg.trim().to_string())
                        .filter(|arg| !arg.is_empty())
                        .collect();
                },
                "heap-size" => {
                    heap_size = Some(
                        value
                            .parse::<usize>()
                            .map_err(|err| anyhow!("invalid --heap-size {:?}: {}", value, err))?,
                    );
                },
                other => bail!("Unknown execute modifier: --{}", other),
            }
        }
    }

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
        heap_size,
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
