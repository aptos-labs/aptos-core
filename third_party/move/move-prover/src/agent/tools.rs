// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

//! Tool infrastructure for the agentic spec inference loop.
//!
//! Provides a text-based tool protocol: Claude emits tool calls as XML tags in its
//! response, we parse them, execute the tool, and send results back as a user message.
//! This avoids changes to the `Message` struct or API integration.

use super::common::{has_timeout, validate_source};
use crate::{cli::Options, inference};
use codespan_reporting::term::termcolor::Buffer;
use log::info;
use std::{
    collections::BTreeMap,
    path::Path,
    sync::{
        atomic::{AtomicUsize, Ordering},
        Arc, Mutex,
    },
    time::Instant,
};

// ---------------------------------------------------------------------------
// Tool trait and data types
// ---------------------------------------------------------------------------

/// A tool the model can invoke during the conversation.
pub trait AgentTool: Send + Sync {
    /// Tool name (used in `<tool_call><name>...</name>` tags).
    fn name(&self) -> &str;
    /// One-line description for the system prompt.
    fn description(&self) -> &str;
    /// Detailed usage instructions for the system prompt.
    fn usage(&self) -> &str;
    /// Execute the tool given named parameters. Returns output text.
    fn execute(&self, params: &BTreeMap<String, String>) -> anyhow::Result<String>;
}

/// A parsed tool call from Claude's response.
pub struct ToolCall {
    /// Sequential ID assigned during parsing (e.g. `"call_1"`), used to correlate results.
    pub id: String,
    pub name: String,
    /// Short reason the model gave for making this call (from `<reason>` tag).
    pub reason: String,
    pub params: BTreeMap<String, String>,
}

// ---------------------------------------------------------------------------
// Parsing and formatting
// ---------------------------------------------------------------------------

/// Parse tool calls from a response text.
///
/// Returns `(text_outside_tool_calls, Vec<ToolCall>)`. The text portion is
/// everything outside `<tool_call>...</tool_call>` blocks concatenated together.
pub fn parse_tool_calls(response: &str) -> (String, Vec<ToolCall>) {
    let mut calls = Vec::new();
    let mut text = String::new();
    let mut rest = response;

    loop {
        match rest.find("<tool_call>") {
            Some(start) => {
                text.push_str(&rest[..start]);
                let after_open = start + "<tool_call>".len();
                match rest[after_open..].find("</tool_call>") {
                    Some(end) => {
                        let block = &rest[after_open..after_open + end];
                        if let Some(call) = parse_single_tool_call(block) {
                            calls.push(call);
                        }
                        rest = &rest[after_open + end + "</tool_call>".len()..];
                    },
                    None => {
                        // Unclosed tag — treat remaining text as plain text.
                        text.push_str(&rest[start..]);
                        break;
                    },
                }
            },
            None => {
                text.push_str(rest);
                break;
            },
        }
    }

    // Assign sequential IDs so callers can correlate results to invocations.
    for (i, call) in calls.iter_mut().enumerate() {
        call.id = format!("call_{}", i + 1);
    }

    (text, calls)
}

/// Parse the interior of a single `<tool_call>` block.
fn parse_single_tool_call(block: &str) -> Option<ToolCall> {
    let name = extract_xml_tag(block, "name")?;
    let reason = extract_xml_tag(block, "reason").unwrap_or_default();
    let mut params = BTreeMap::new();

    let mut search_from = 0;
    while let Some(param_start) = block[search_from..].find("<param name=\"") {
        let abs_start = search_from + param_start;
        let attr_start = abs_start + "<param name=\"".len();
        let attr_end = block[attr_start..].find('"')?;
        let param_name = block[attr_start..attr_start + attr_end].to_string();

        // Find the closing `>` of the opening tag.
        let content_start = block[attr_start + attr_end..].find('>')? + attr_start + attr_end + 1;
        let content_end = block[content_start..].find("</param>")?;
        let param_value = block[content_start..content_start + content_end].to_string();

        params.insert(param_name, param_value);
        search_from = content_start + content_end + "</param>".len();
    }

    Some(ToolCall {
        id: String::new(), // assigned later by parse_tool_calls
        name,
        reason,
        params,
    })
}

/// Extract text content of a simple XML tag like `<name>...</name>`.
fn extract_xml_tag(text: &str, tag: &str) -> Option<String> {
    let open = format!("<{}>", tag);
    let close = format!("</{}>", tag);
    let start = text.find(&open)? + open.len();
    let end = text[start..].find(&close)? + start;
    Some(text[start..end].trim().to_string())
}

/// Format a tool result as text for sending back to Claude.
pub fn format_tool_result(
    id: &str,
    name: &str,
    reason: &str,
    success: bool,
    output: &str,
) -> String {
    let status = if success { "success" } else { "error" };
    let reason_tag = if reason.is_empty() {
        String::new()
    } else {
        format!("\n<reason>{}</reason>", reason)
    };
    format!(
        "<tool_result>\n<id>{}</id>\n<name>{}</name>{}\n<status>{}</status>\n<output>\n{}\n</output>\n</tool_result>",
        id, name, reason_tag, status, output
    )
}

/// Generate the tool documentation section for the system prompt.
pub fn tool_prompt_section(tools: &[Box<dyn AgentTool>]) -> String {
    if tools.is_empty() {
        return String::new();
    }

    let mut section = String::from(
        "\n\n## Tools\n\n\
         You have access to the following tools. To call a tool, include a `<tool_call>` block \
         in your response. **Make only one tool call per response.** Wait for the result before \
         deciding on the next action — do not batch dependent calls.\n\n\
         **Tool call format:**\n\
         ```\n\
         <tool_call>\n\
         <name>tool_name</name>\n\
         <reason>short note why you are making this call</reason>\n\
         <param name=\"param_name\">param_value</param>\n\
         </tool_call>\n\
         ```\n\n\
         Each tool result includes a sequential `<id>` (e.g. `call_1`, `call_2`) matching the \
         order of your tool calls, so you can correlate results to invocations.\n\n\
         **Important:** When you use a tool, you will receive the result in a `<tool_result>` \
         block. Always read the result before deciding your next action. Call one tool at a time \
         across successive turns — each step typically depends on the previous result.\n\n\
         ### Available Tools\n\n",
    );

    for tool in tools {
        section.push_str(&format!("#### `{}`\n\n", tool.name()));
        section.push_str(tool.description());
        section.push('\n');
        section.push('\n');
        section.push_str(tool.usage());
        section.push_str("\n\n");
    }

    section
}

// ---------------------------------------------------------------------------
// Tool implementations
// ---------------------------------------------------------------------------

/// Maximum number of consecutive timeout reformulations before forcing `pragma verify = false`.
const MAX_TIMEOUT_REFORMULATIONS: usize = 2;

/// Shared tracker for the last source submitted to a tool.
///
/// Used by `VerifyTool` and `WPInferenceTool` to record each source they receive,
/// so the caller can recover the most recent attempt when the iteration budget
/// is exhausted before the model produces a final response.
pub type LastSourceTracker = Arc<Mutex<Option<String>>>;

/// Tool: verify all functions in a Move source against their specifications.
pub struct VerifyTool {
    options: Options,
    experiments: Vec<String>,
    consecutive_timeouts: AtomicUsize,
    last_source: LastSourceTracker,
}

impl VerifyTool {
    pub fn new(
        options: Options,
        experiments: Vec<String>,
        last_source: LastSourceTracker,
    ) -> Self {
        Self {
            options,
            experiments,
            consecutive_timeouts: AtomicUsize::new(0),
            last_source,
        }
    }
}

impl AgentTool for VerifyTool {
    fn name(&self) -> &str {
        "verify"
    }

    fn description(&self) -> &str {
        "Verify all functions in a Move source against their specifications using the Move Prover."
    }

    fn usage(&self) -> &str {
        "Parameters:\n\
         - `source`: Complete Move module source code to verify.\n\n\
         Returns verification diagnostics on failure, or \"Verification succeeded.\" on success."
    }

    fn execute(&self, params: &BTreeMap<String, String>) -> anyhow::Result<String> {
        let source = params
            .get("source")
            .ok_or_else(|| anyhow::anyhow!("missing required parameter: source"))?;

        info!("[agent] Executing tool: verify");

        // Validate source before running verification
        if let Err(msg) = validate_source(source) {
            return Err(anyhow::anyhow!("Source validation failed:\n\n{}", msg));
        }

        // Record this source as the last submitted attempt (for budget-exhaustion fallback).
        *self.last_source.lock().unwrap() = Some(source.clone());

        let temp_dir = tempfile::TempDir::new()?;
        let temp_source = temp_dir.path().join("tool_verify.move");
        std::fs::write(&temp_source, source)?;

        let (success, diags) = run_verification(&temp_source, &self.options, &self.experiments)?;

        if success {
            self.consecutive_timeouts.store(0, Ordering::Relaxed);
            Ok("Verification succeeded.".to_string())
        } else if has_timeout(&diags) {
            let count = self.consecutive_timeouts.fetch_add(1, Ordering::Relaxed) + 1;
            if count >= MAX_TIMEOUT_REFORMULATIONS {
                self.consecutive_timeouts.store(0, Ordering::Relaxed);
                Ok(format!(
                    "{}\n\n\
                     **Timeout reformulation limit reached ({} attempts).** \
                     You MUST now add `pragma verify = false;` to the spec block of each \
                     timed-out function.\n\n\
                     **CRITICAL: Keep all existing `ensures`, `aborts_if`, and other spec \
                     conditions exactly as they are.** Do NOT remove, weaken, or empty any \
                     conditions. The only change is adding `pragma verify = false;` and a \
                     comment explaining why. Use hedged language (\"possibly due to\", not \
                     \"due to\").\n\n\
                     Only disable verification for functions that actually timed out — keep \
                     all other specs intact.",
                    diags, count
                ))
            } else {
                Ok(diags)
            }
        } else {
            self.consecutive_timeouts.store(0, Ordering::Relaxed);
            Ok(diags)
        }
    }
}

/// Tool: run WP inference and return the inferred spec block for a function.
pub struct WPInferenceTool {
    options: Options,
    experiments: Vec<String>,
    last_source: LastSourceTracker,
}

impl WPInferenceTool {
    pub fn new(
        options: Options,
        experiments: Vec<String>,
        last_source: LastSourceTracker,
    ) -> Self {
        Self {
            options,
            experiments,
            last_source,
        }
    }
}

impl AgentTool for WPInferenceTool {
    fn name(&self) -> &str {
        "wp_inference"
    }

    fn description(&self) -> &str {
        "Run weakest-precondition inference on Move source and return the enriched source with inferred specs."
    }

    fn usage(&self) -> &str {
        "Parameters:\n\
         - `source`: Complete Move module source code.\n\n\
         Returns the WP-enriched source with inferred specification blocks."
    }

    fn execute(&self, params: &BTreeMap<String, String>) -> anyhow::Result<String> {
        let source = params
            .get("source")
            .ok_or_else(|| anyhow::anyhow!("missing required parameter: source"))?;

        info!("[agent] Executing tool: wp_inference");

        // Validate source before running WP inference
        if let Err(msg) = validate_source(source) {
            return Err(anyhow::anyhow!("Source validation failed:\n\n{}", msg));
        }

        let temp_dir = tempfile::TempDir::new()?;
        let temp_source = temp_dir.path().join("tool_wp.move");
        std::fs::write(&temp_source, source)?;

        let result = run_wp_on_source(&temp_source, &self.options, &self.experiments)?;
        // Record the WP-enriched output as the last known source.
        *self.last_source.lock().unwrap() = Some(result.clone());
        Ok(result)
    }
}

// ---------------------------------------------------------------------------
// Helper functions (adapted from loop_driver module-level fns)
// ---------------------------------------------------------------------------

const COMPILATION_ERROR_PREFIX: &str =
    "COMPILATION ERROR — the source has syntax or type errors that must be fixed \
     before verification can run.\n\
     \n\
     HINT: A common cause is misuse of `old()` expressions. Review the \
     \"`old()` usage rules\" section (the table) in your instructions — `old()` is \
     forbidden in `aborts_if`, `requires`, and on non-parameter variables in loop \
     invariants.\n";

/// Run the Move Prover on a source file, verifying all functions.
///
/// Compilation and verification are split so that compilation errors can be
/// labeled distinctly from verification failures.
fn run_verification(
    source_path: &Path,
    original_options: &Options,
    experiments: &[String],
) -> anyhow::Result<(bool, String)> {
    let temp_dir = tempfile::TempDir::new()?;
    let base_name = format!("{}.bpl", source_path.file_stem().unwrap().to_str().unwrap());
    let output = temp_dir
        .path()
        .join(base_name)
        .to_str()
        .unwrap()
        .to_string();

    let mut verify_options = Options::default();
    verify_options.output_path = output;
    verify_options.move_sources = vec![source_path.to_string_lossy().to_string()];
    verify_options.move_deps = original_options.move_deps.clone();
    verify_options.move_named_address_values = original_options.move_named_address_values.clone();
    verify_options.language_version = original_options.language_version;
    verify_options.prover = original_options.prover.clone();
    verify_options.backend = original_options.backend.clone();
    verify_options.backend.check_tool_versions()?;

    // Step 1: Compilation
    let mut error_writer = Buffer::no_color();
    let env = crate::create_move_prover_v2_model(
        &mut error_writer,
        verify_options.clone(),
        experiments.to_vec(),
    );
    let compile_diags = String::from_utf8_lossy(&error_writer.into_inner()).to_string();

    let mut env = match env {
        Ok(env) if !env.has_errors() => env,
        Ok(_env) => {
            // Model created but has errors — compilation failure.
            return Ok((
                false,
                format!("{}\n\n{}", COMPILATION_ERROR_PREFIX, compile_diags),
            ));
        },
        Err(e) => {
            let mut msg = format!("{}\n", e);
            if !compile_diags.is_empty() {
                msg.push_str(&compile_diags);
            }
            return Ok((false, format!("{}\n\n{}", COMPILATION_ERROR_PREFIX, msg)));
        },
    };

    // Step 2: Verification (only runs if compilation succeeded)
    let now = Instant::now();
    let mut error_writer = Buffer::no_color();
    let result =
        crate::run_move_prover_with_model_v2(&mut env, &mut error_writer, verify_options, now);

    let diags = String::from_utf8_lossy(&error_writer.into_inner()).to_string();
    match result {
        Ok(()) if diags.is_empty() => Ok((true, String::new())),
        Ok(()) => Ok((true, diags)),
        Err(err) => {
            let err_str = err.to_string();
            let mut full_diags = format!("{}\n", err_str);
            if !diags.is_empty() {
                full_diags += &diags;
            }
            // Bytecode transformation and condition generation errors are
            // compilation errors (e.g. old() misuse), not verification failures.
            if err_str.contains("bytecode transformation errors")
                || err_str.contains("model building errors")
                || err_str.contains("condition generation errors")
            {
                Ok((
                    false,
                    format!("{}\n\n{}", COMPILATION_ERROR_PREFIX, full_diags),
                ))
            } else {
                Ok((false, full_diags))
            }
        },
    }
}

/// Run WP-based spec inference on a source file and return the enriched source string.
fn run_wp_on_source(
    source_path: &Path,
    original_options: &Options,
    experiments: &[String],
) -> anyhow::Result<String> {
    let temp_dir = tempfile::TempDir::new()?;
    let base_name = format!("{}.bpl", source_path.file_stem().unwrap().to_str().unwrap());
    let output = temp_dir
        .path()
        .join(base_name)
        .to_str()
        .unwrap()
        .to_string();

    let mut inf_options = Options::default();
    inf_options.output_path = output;
    inf_options.move_sources = vec![source_path.to_string_lossy().to_string()];
    inf_options.move_deps = original_options.move_deps.clone();
    inf_options.move_named_address_values = original_options.move_named_address_values.clone();
    inf_options.language_version = original_options.language_version;
    inf_options.prover = original_options.prover.clone();
    inf_options.backend = original_options.backend.clone();
    inf_options.inference.inference = true;

    let now = Instant::now();
    let mut error_writer = Buffer::no_color();
    let env = crate::create_move_prover_v2_model(
        &mut error_writer,
        inf_options.clone(),
        experiments.to_vec(),
    );
    let compile_diags = String::from_utf8_lossy(&error_writer.into_inner()).to_string();
    let mut env = match env {
        Ok(env) if !env.has_errors() => env,
        Ok(_env) => {
            return Err(anyhow::anyhow!(
                "{}\n\n{}",
                COMPILATION_ERROR_PREFIX,
                compile_diags
            ));
        },
        Err(e) => {
            let mut msg = format!("{}\n", e);
            if !compile_diags.is_empty() {
                msg.push_str(&compile_diags);
            }
            return Err(anyhow::anyhow!("{}\n\n{}", COMPILATION_ERROR_PREFIX, msg));
        },
    };
    let mut error_writer = Buffer::no_color();
    let pairs =
        match inference::run_inference_to_strings(&mut env, &mut error_writer, inf_options, now) {
            Ok(pairs) => pairs,
            Err(e) => {
                let diags = String::from_utf8_lossy(&error_writer.into_inner()).to_string();
                let mut msg = format!("{}\n", e);
                if !diags.is_empty() {
                    msg.push_str(&diags);
                }
                return Err(anyhow::anyhow!("{}", msg));
            },
        };

    if pairs.is_empty() {
        return Err(anyhow::anyhow!("WP produced no output"));
    }
    Ok(pairs
        .into_iter()
        .map(|(_, s)| s)
        .collect::<Vec<_>>()
        .join("\n"))
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_no_tool_calls() {
        let response = "Here is my analysis.\n\n```move\nmodule 0x1::m {}\n```";
        let (text, calls) = parse_tool_calls(response);
        assert_eq!(text, response);
        assert!(calls.is_empty());
    }

    #[test]
    fn test_parse_single_tool_call() {
        let response = "Let me verify this.\n\
            <tool_call>\n\
            <name>verify</name>\n\
            <reason>check postconditions</reason>\n\
            <param name=\"source\">module 0x1::m { fun foo() {} }</param>\n\
            </tool_call>\n\
            Done.";
        let (text, calls) = parse_tool_calls(response);
        assert_eq!(text.trim(), "Let me verify this.\n\nDone.");
        assert_eq!(calls.len(), 1);
        assert_eq!(calls[0].id, "call_1");
        assert_eq!(calls[0].name, "verify");
        assert_eq!(calls[0].reason, "check postconditions");
        assert_eq!(
            calls[0].params.get("source").unwrap(),
            "module 0x1::m { fun foo() {} }"
        );
    }

    #[test]
    fn test_parse_tool_call_missing_reason() {
        let response = "\
            <tool_call>\n\
            <name>verify</name>\n\
            <param name=\"source\">src</param>\n\
            </tool_call>";
        let (_, calls) = parse_tool_calls(response);
        assert_eq!(calls.len(), 1);
        assert_eq!(calls[0].reason, "");
    }

    #[test]
    fn test_parse_multiple_tool_calls() {
        let response = "\
            <tool_call>\n\
            <name>verify</name>\n\
            <param name=\"source\">src1</param>\n\
            </tool_call>\n\
            <tool_call>\n\
            <name>wp_inference</name>\n\
            <param name=\"source\">src2</param>\n\
            </tool_call>";
        let (_, calls) = parse_tool_calls(response);
        assert_eq!(calls.len(), 2);
        assert_eq!(calls[0].id, "call_1");
        assert_eq!(calls[0].name, "verify");
        assert_eq!(calls[1].id, "call_2");
        assert_eq!(calls[1].name, "wp_inference");
    }

    #[test]
    fn test_format_tool_result_success() {
        let result = format_tool_result(
            "call_1",
            "verify",
            "check postconditions",
            true,
            "Verification succeeded.",
        );
        assert!(result.contains("<id>call_1</id>"));
        assert!(result.contains("<name>verify</name>"));
        assert!(result.contains("<reason>check postconditions</reason>"));
        assert!(result.contains("<status>success</status>"));
        assert!(result.contains("Verification succeeded."));
    }

    #[test]
    fn test_format_tool_result_error() {
        let result = format_tool_result(
            "call_2",
            "verify",
            "",
            false,
            "error: post-condition failed",
        );
        assert!(result.contains("<id>call_2</id>"));
        assert!(!result.contains("<reason>"));
        assert!(result.contains("<status>error</status>"));
        assert!(result.contains("post-condition failed"));
    }

    #[test]
    fn test_unclosed_tool_call_treated_as_text() {
        let response = "text before <tool_call>\n<name>foo</name>\nno closing tag";
        let (text, calls) = parse_tool_calls(response);
        assert!(calls.is_empty());
        assert!(text.contains("<tool_call>"));
    }
}
