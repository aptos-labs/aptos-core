// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

//! Model-driven loop for AI-powered spec inference.
//!
//! Unlike the agent-driven strategy where Rust orchestrates a multi-phase loop with
//! specific prompts, this strategy sends a single comprehensive prompt to the model
//! and lets it autonomously drive the workflow by calling `verify` and `wp_inference`
//! tools as needed.

use crate::{
    agent::{
        client::{ClaudeClient, Message},
        common::{compile_check, has_vacuous_conditions, run_inference, validate_source},
        response_parser,
        tools::{self, AgentTool, LastSourceTracker, VerifyTool, WPInferenceTool},
    },
    cli::Options,
};
use log::{debug, info};
use std::{
    path::{Path, PathBuf},
    sync::{Arc, Mutex},
    time::{Duration, Instant},
};

const MODEL_DRIVEN_PROMPT: &str = include_str!("model_driven_prompt.md");

/// Run the model-driven inference loop.
///
/// 1. Run initial WP inference to get enriched source.
/// 2. Build a single comprehensive system prompt.
/// 3. Send one user message with the WP-enriched source.
/// 4. Loop: send to Claude → handle tool calls → until final text response.
/// 5. Extract final source from response, compile-check, write output.
pub fn run_model_driven_loop(options: Options, experiments: Vec<String>) -> anyhow::Result<()> {
    let agent_opts = &options.agent;
    let max_iterations = agent_opts.ai_max_iterations;

    let client = ClaudeClient::new(agent_opts.ai_model.clone(), agent_opts.ai_max_tokens)?;

    // Run initial WP-based inference to get enriched source.
    info!("[model-driven] Running initial WP-based spec inference...");
    let initial_source = match run_inference(options.clone(), &experiments)? {
        Some(s) => s,
        None => {
            info!("[model-driven] No functions with inferred specs found. Nothing to do.");
            return Ok(());
        },
    };

    // Shared tracker: tools record every source they receive so we can
    // recover the last attempt when the iteration budget is exhausted.
    let last_source: LastSourceTracker = Arc::new(Mutex::new(None));

    // Initialize tools.
    let tools: Vec<Box<dyn AgentTool>> = vec![
        Box::new(VerifyTool::new(
            options.clone(),
            experiments.clone(),
            Arc::clone(&last_source),
        )),
        Box::new(WPInferenceTool::new(
            options.clone(),
            experiments.clone(),
            Arc::clone(&last_source),
        )),
    ];

    // Build system prompt: base prompt + tool documentation.
    let system_prompt = if let Some(ref path) = agent_opts.ai_system_prompt {
        std::fs::read_to_string(path)?
    } else {
        MODEL_DRIVEN_PROMPT.to_string()
    } + &tools::tool_prompt_section(&tools);
    debug!(
        "\n{}\n[model-driven] SYSTEM PROMPT\n{}\n{}",
        "=".repeat(80),
        "=".repeat(80),
        system_prompt
    );

    // Build initial user message with the WP-enriched source.
    let has_vacuous = has_vacuous_conditions(&initial_source);
    let user_message = if has_vacuous {
        format!(
            "Here is the Move source with WP-inferred specifications. Note that some conditions \
             are marked `[inferred = vacuous]`, indicating loops that need invariants. Please \
             follow the workflow: add loop invariants, re-run WP inference via the `wp_inference` \
             tool, simplify, verify, and iterate until verification succeeds.\n\n\
             ```move\n{}\n```",
            initial_source
        )
    } else {
        format!(
            "Here is the Move source with WP-inferred specifications. Please simplify the \
             specifications, verify using the `verify` tool, and iterate until verification \
             succeeds.\n\n\
             ```move\n{}\n```",
            initial_source
        )
    };

    let mut messages: Vec<Message> = vec![Message {
        role: "user",
        content: user_message.clone(),
    }];
    debug!(
        "\n{}\n[model-driven] >>> USER MESSAGE\n{}\n{}",
        "-".repeat(80),
        "-".repeat(80),
        user_message
    );

    // Main loop: send to Claude, handle tool calls, repeat.
    let total_start = Instant::now();
    let mut iterations = 0;
    let mut claude_duration = Duration::ZERO;
    let mut tool_duration = Duration::ZERO;
    let final_source = loop {
        if iterations >= max_iterations {
            info!(
                "[model-driven] Exhausted {} iterations without final response.",
                max_iterations
            );
            break None;
        }
        iterations += 1;

        info!(
            "[model-driven] Iteration {}/{}: sending to Claude...",
            iterations, max_iterations
        );

        let now = Instant::now();
        let response = client.send(&system_prompt, &messages)?;
        claude_duration += now.elapsed();
        debug!(
            "\n{}\n[model-driven] <<< ASSISTANT MESSAGE\n{}\n{}",
            "=".repeat(80),
            "=".repeat(80),
            response
        );
        messages.push(Message {
            role: "assistant",
            content: response.clone(),
        });

        let (text, tool_calls) = tools::parse_tool_calls(&response);

        if tool_calls.is_empty() {
            // No tool calls — this is the final response.
            info!("[model-driven] Received final response (no tool calls).");
            break extract_and_check_source(&text, &options, &experiments);
        }

        // Execute only the first tool call. When the model emits multiple
        // tool calls in one response it hasn't seen intermediate results, so
        // subsequent calls are based on guesses. Execute the first, ignore the
        // rest, and let the model decide its next action after seeing the result.
        let call = &tool_calls[0];
        if call.reason.is_empty() {
            info!("[model-driven] Executing tool: {}", call.name);
        } else {
            info!(
                "[model-driven] Executing tool: {} ({})",
                call.name, call.reason
            );
        }
        let now = Instant::now();
        let result = match tools.iter().find(|t| t.name() == call.name) {
            Some(tool) => match tool.execute(&call.params) {
                Ok(output) => {
                    tools::format_tool_result(&call.id, &call.name, &call.reason, true, &output)
                },
                Err(e) => tools::format_tool_result(
                    &call.id,
                    &call.name,
                    &call.reason,
                    false,
                    &e.to_string(),
                ),
            },
            None => tools::format_tool_result(
                &call.id,
                &call.name,
                &call.reason,
                false,
                &format!("Unknown tool: {}", call.name),
            ),
        };
        tool_duration += now.elapsed();
        let mut results = result;
        if tool_calls.len() > 1 {
            info!(
                "[model-driven] Ignoring {} additional tool call(s) in same response.",
                tool_calls.len() - 1
            );
            results.push_str(
                "\n\nNote: You made multiple tool calls in one response. \
                 Only the first was executed; the rest were ignored. \
                 Please make one tool call per response and wait for the result.",
            );
        }

        // Send tool results back as a user message.
        debug!(
            "\n{}\n[model-driven] >>> TOOL RESULTS\n{}\n{}",
            "-".repeat(80),
            "-".repeat(80),
            results
        );
        messages.push(Message {
            role: "user",
            content: results,
        });
    };

    info!(
        "[model-driven] {} iterations, {:.1}s total, {:.1}s claude, {:.1}s tools",
        iterations,
        total_start.elapsed().as_secs_f64(),
        claude_duration.as_secs_f64(),
        tool_duration.as_secs_f64(),
    );

    // Write output. Prefer: final extracted source > last tool-submitted source > initial WP source.
    let last_tool_source = last_source.lock().unwrap().clone();
    let source_to_write = final_source
        .or(last_tool_source)
        .unwrap_or(initial_source);
    write_output(&source_to_write, &options)?;

    Ok(())
}

/// Extract Move source from a response and validate it.
/// Returns `Some(source)` if extraction and validation succeed, `None` otherwise.
fn extract_and_check_source(
    response: &str,
    options: &Options,
    experiments: &[String],
) -> Option<String> {
    let source = match response_parser::extract_move_source(response) {
        Ok(s) => s,
        Err(e) => {
            info!(
                "[model-driven] Failed to extract source from response: {}",
                e
            );
            return None;
        },
    };

    // Text-only policy check (aborts_if_is_partial)
    if let Err(msg) = validate_source(&source) {
        info!("[model-driven] Final source validation failed: {}", msg);
        return None;
    }

    // Compile check for the final source gate
    match compile_check(&source, options, experiments) {
        Ok(Ok(())) => {
            info!("[model-driven] Final source compiles and validates successfully.");
            Some(source)
        },
        Ok(Err(diags)) => {
            info!(
                "[model-driven] Final source has compilation errors: {}",
                diags
            );
            None
        },
        Err(e) => {
            info!("[model-driven] Compile check failed unexpectedly: {}", e);
            None
        },
    }
}

/// Write the final source to the output location.
fn write_output(source: &str, options: &Options) -> anyhow::Result<()> {
    if let Some(source_file) = options.move_sources.first() {
        let source_path = Path::new(source_file);
        let stem = source_path
            .file_stem()
            .expect("source file should have a stem");
        let output_path = if let Some(ref dir) = options.inference.inference_output_dir {
            PathBuf::from(dir).join(format!("{}.enriched.move", stem.to_string_lossy()))
        } else {
            let source_dir = source_path
                .parent()
                .expect("source file should have a parent directory");
            source_dir.join(format!("{}.enriched.move", stem.to_string_lossy()))
        };

        if let Some(parent) = output_path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        std::fs::write(&output_path, source)?;
        info!(
            "[model-driven] Wrote refined source to {}",
            output_path.display()
        );
    }
    Ok(())
}
