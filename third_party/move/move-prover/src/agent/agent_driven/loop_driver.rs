// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

//! Core agent loop orchestration for AI-powered spec inference.

use super::prompt::{PromptBuilder, PromptTemplate};
use crate::{
    agent::{
        client::{ClaudeClient, Message},
        common::{compile_check, has_timeout, has_vacuous_conditions, run_inference},
        response_parser,
        tools::{self, AgentTool},
    },
    cli::Options,
};
use codespan_reporting::term::termcolor::Buffer;
use log::{debug, info, warn};
use serde::Serialize;
use std::{
    path::{Path, PathBuf},
    time::{Duration, Instant},
};

/// Maximum number of timeout reformulation attempts before allowing `pragma verify = false`.
const MAX_TIMEOUT_REFORMULATIONS: usize = 2;

/// Maximum number of verification failure retries before giving up.
const MAX_VERIFY_FAILURE_RETRIES: usize = 3;

/// Orchestrates the AI-powered spec inference loop.
///
/// Holds the mutable conversation state (source under refinement, message
/// history, diagnostics) and the immutable configuration (options, client,
/// prompt builder). Each phase of the loop is a separate `&mut self` method.
///
/// The struct derives `Serialize` so it can be passed directly to Tera
/// templates as the `agent` context variable (e.g. `{{ agent.current_source }}`).
/// Fields that should not be visible in templates are marked `#[serde(skip)]`.
#[derive(Serialize)]
pub(super) struct Agent {
    /// CLI options (for deps, output paths, prover/backend config).
    options: Options,
    /// Experiment flags forwarded to prover/model creation.
    experiments: Vec<String>,
    /// Current Move source being refined.
    current_source: String,
    /// Prover diagnostics from the last failed verification attempt.
    last_diagnostics: Option<String>,
    /// Move compilation diagnostics from the last failed compile check.
    last_move_diagnostics: Option<String>,
    /// WP-enriched source from re-running inference on the last failed source.
    last_wp_source: String,
    /// Total number of iterations consumed across all phases.
    iterations_used: usize,
    /// Maximum iterations budget.
    max_iterations: usize,
    /// System prompt string (resolved once at construction).
    system_prompt: String,
    /// Current iteration counter (1-based), set before each prompt render.
    iteration: usize,
    /// Clamped timeout attempt counter for display in templates.
    timeout_attempt: usize,
    /// Maximum timeout reformulation attempts (constant).
    max_timeout_attempts: usize,
    // --- Fields not accessible from templates ---
    /// Consecutive timeout reformulation attempts (internal counter).
    #[serde(skip)]
    timeout_attempts: usize,
    /// Consecutive verification failure retries (internal counter).
    #[serde(skip)]
    verify_failure_attempts: usize,
    /// Conversation history sent to the Claude API.
    #[serde(skip)]
    messages: Vec<Message>,
    /// Claude API client.
    #[serde(skip)]
    client: ClaudeClient,
    /// Prompt template builder.
    #[serde(skip)]
    prompt_builder: PromptBuilder,
    /// Tools available to the model during conversation.
    #[serde(skip)]
    tools: Vec<Box<dyn AgentTool>>,
    /// Accumulated time spent in Claude API calls.
    #[serde(skip)]
    claude_duration: Duration,
    /// Accumulated time spent in tool execution (verify, WP inference).
    #[serde(skip)]
    tool_duration: Duration,
}

impl Agent {
    /// Create the agent: build the model, run initial WP inference, initialize
    /// conversation state. Returns `Ok(None)` if no functions have inferred specs.
    fn new(options: Options, experiments: Vec<String>) -> anyhow::Result<Option<Self>> {
        let agent_opts = &options.agent;
        let max_iterations = agent_opts.ai_max_iterations;

        let client = ClaudeClient::new(agent_opts.ai_model.clone(), agent_opts.ai_max_tokens)?;
        let prompt_builder = PromptBuilder::new(agent_opts.ai_system_prompt.as_deref())?;

        // Run initial WP-based inference to get enriched source.
        info!("[agent] Running initial WP-based spec inference...");
        let current_source = match run_inference(options.clone(), &experiments)? {
            Some(s) => s,
            None => {
                info!("[agent] No functions with inferred specs found. Nothing to do.");
                return Ok(None);
            },
        };

        // Tools are available but not registered for now — the agent-driven
        // loop is purely prompt-driven.
        let agent_tools: Vec<Box<dyn AgentTool>> = vec![];

        let mut system_prompt = prompt_builder.system_prompt().to_string();
        system_prompt.push_str(&tools::tool_prompt_section(&agent_tools));
        debug!(
            "\n{}\n[agent] SYSTEM PROMPT\n{}\n{}",
            "=".repeat(80),
            "=".repeat(80),
            system_prompt
        );

        Ok(Some(Self {
            current_source,
            messages: Vec::new(),
            last_diagnostics: None,
            last_move_diagnostics: None,
            last_wp_source: String::new(),
            iterations_used: 0,
            timeout_attempts: 0,
            verify_failure_attempts: 0,
            max_iterations,
            iteration: 0,
            timeout_attempt: 0,
            max_timeout_attempts: MAX_TIMEOUT_REFORMULATIONS,
            system_prompt,
            client,
            prompt_builder,
            options,
            experiments,
            tools: agent_tools,
            claude_duration: Duration::ZERO,
            tool_duration: Duration::ZERO,
        }))
    }

    /// Check whether the iteration budget has remaining capacity.
    fn has_budget(&self) -> bool {
        self.iterations_used < self.max_iterations
    }

    /// Append a user message to the conversation history.
    fn push_user_msg(&mut self, content: String) {
        debug!(
            "\n{}\n[agent] >>> USER MESSAGE\n{}\n{}",
            "-".repeat(80),
            "-".repeat(80),
            content
        );
        self.messages.push(Message {
            role: "user",
            content,
        });
    }

    /// Append an assistant message to the conversation history.
    fn push_assistant_msg(&mut self, content: String) {
        debug!(
            "\n{}\n[agent] <<< ASSISTANT MESSAGE\n{}\n{}",
            "=".repeat(80),
            "=".repeat(80),
            content
        );
        self.messages.push(Message {
            role: "assistant",
            content,
        });
    }

    /// Send the current conversation to Claude, execute any tool calls in the
    /// response, and return the final text response (with tool call blocks removed).
    ///
    /// If Claude's response contains `<tool_call>` blocks, we execute each tool,
    /// send the results back as a user message, and continue until Claude produces
    /// a response with no tool calls.
    fn send_to_claude(&mut self) -> anyhow::Result<String> {
        loop {
            let now = Instant::now();
            let response = self.client.send(&self.system_prompt, &self.messages)?;
            self.claude_duration += now.elapsed();
            self.push_assistant_msg(response.clone());

            let (text, tool_calls) = tools::parse_tool_calls(&response);

            if tool_calls.is_empty() {
                // No tool calls — return the final text response.
                return Ok(text);
            }

            // Execute each tool call and collect results.
            let mut results = String::new();
            for call in &tool_calls {
                if call.reason.is_empty() {
                    info!("[agent] Executing tool: {}", call.name);
                } else {
                    info!("[agent] Executing tool: {} ({})", call.name, call.reason);
                }
                let now = Instant::now();
                let result = match self.tools.iter().find(|t| t.name() == call.name) {
                    Some(tool) => match tool.execute(&call.params) {
                        Ok(output) => tools::format_tool_result(
                            &call.id,
                            &call.name,
                            &call.reason,
                            true,
                            &output,
                        ),
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
                self.tool_duration += now.elapsed();
                results.push_str(&result);
                results.push('\n');
            }

            // Send tool results back as a user message.
            self.push_user_msg(results);
            // Continue the loop — Claude will respond to the tool results.
        }
    }

    /// Render a prompt template with `self` bound as the `agent` context variable.
    fn render_prompt(&self, template: PromptTemplate) -> anyhow::Result<String> {
        self.prompt_builder.render(template, self)
    }

    /// Run verification on `current_source`, update `last_diagnostics`.
    /// Returns true if verification succeeded.
    fn verify_current(&mut self) -> anyhow::Result<bool> {
        let temp_dir = tempfile::TempDir::new()?;
        let temp_source = temp_dir.path().join("agent_source.move");
        std::fs::write(&temp_source, &self.current_source)?;

        info!(
            "[agent] Iteration {}/{}: verifying current source...",
            self.iterations_used, self.max_iterations
        );

        let now = Instant::now();
        let verification_result = run_verification(&temp_source, &self.options, &self.experiments);
        self.tool_duration += now.elapsed();
        match verification_result {
            Ok((true, diags)) => {
                info!("[agent] Verification succeeded.");
                if !diags.is_empty() {
                    info!("[agent] Diagnostics: {}", diags);
                }
                self.last_diagnostics = None;
                self.timeout_attempts = 0;
                self.verify_failure_attempts = 0;
                Ok(true)
            },
            Ok((false, diags)) => {
                if diags.starts_with(COMPILATION_ERROR_PREFIX) {
                    // Compilation error (e.g. old() misuse) — route to
                    // MoveFailure prompt, not VerifyFailure.
                    self.timeout_attempts = 0;
                    self.verify_failure_attempts = 0;
                    let error_count = diags.lines().filter(|l| l.contains("error")).count();
                    info!("[agent] Compilation error ({} error line(s)).", error_count);
                    self.last_move_diagnostics = Some(diags);
                    self.last_diagnostics = None;
                } else if has_timeout(&diags) {
                    self.timeout_attempts += 1;
                    self.verify_failure_attempts = 0;
                    info!(
                        "[agent] Verification timed out (timeout attempt {}).",
                        self.timeout_attempts
                    );
                    self.last_diagnostics = Some(diags);
                    self.last_move_diagnostics = None;
                } else {
                    self.timeout_attempts = 0;
                    self.verify_failure_attempts += 1;
                    let error_count = diags.lines().filter(|l| l.contains("error")).count();
                    info!(
                        "[agent] Verification failed ({} error line(s)), attempt {}/{}.",
                        error_count, self.verify_failure_attempts, MAX_VERIFY_FAILURE_RETRIES,
                    );
                    self.last_diagnostics = Some(diags);
                    self.last_move_diagnostics = None;
                }
                // Re-run WP for context on the failed source.
                let now = Instant::now();
                self.last_wp_source =
                    match run_wp_on_source(&temp_source, &self.options, &self.experiments) {
                        Ok(s) => s,
                        Err(e) => {
                            warn!("[agent] Unexpected WP failure for context: {:#}", e);
                            String::new()
                        },
                    };
                self.tool_duration += now.elapsed();
                Ok(false)
            },
            Err(e) => {
                info!("[agent] Verification error: {}", e);
                self.last_diagnostics = Some(format!("Verification crashed: {}", e));
                let now = Instant::now();
                self.last_wp_source =
                    match run_wp_on_source(&temp_source, &self.options, &self.experiments) {
                        Ok(s) => s,
                        Err(e) => {
                            warn!("[agent] Unexpected WP failure for context: {:#}", e);
                            String::new()
                        },
                    };
                self.tool_duration += now.elapsed();
                Ok(false)
            },
        }
    }

    /// Extract Move source from a Claude response and compile-check it.
    /// If compilation fails, iteratively ask Claude to fix Move errors.
    /// Returns `None` if budget exhausted or parsing repeatedly fails.
    fn receive_move_source(&mut self, response: &str) -> anyhow::Result<Option<String>> {
        let mut source = match response_parser::extract_move_source(response) {
            Ok(s) => s,
            Err(e) => {
                info!("...failed to parse response: {}", e);
                return Ok(None);
            },
        };

        // Compile-check loop
        loop {
            let now = Instant::now();
            let check_result = compile_check(&source, &self.options, &self.experiments);
            self.tool_duration += now.elapsed();
            match check_result? {
                Ok(()) => {
                    self.last_move_diagnostics = None;
                    return Ok(Some(source));
                },
                Err(diags) => {
                    info!("...Move compilation failed, asking for fix");
                    self.last_move_diagnostics = Some(diags);

                    if !self.has_budget() {
                        return Ok(None);
                    }
                    self.iterations_used += 1;
                    self.iteration = self.iterations_used;

                    let msg = self.render_prompt(PromptTemplate::MoveFailure)?;
                    self.push_user_msg(msg);
                    let resp = self.send_to_claude()?;

                    match response_parser::extract_move_source(&resp) {
                        Ok(s) => source = s,
                        Err(e) => {
                            info!("...failed to parse fix response: {}", e);
                            return Ok(None);
                        },
                    }
                },
            }
        }
    }

    /// Send an invariant prompt, parse response, re-run WP, update current_source.
    /// Returns false if the response could not be parsed.
    fn send_invariant(&mut self) -> anyhow::Result<bool> {
        self.iterations_used += 1;
        self.iteration = self.iterations_used;
        info!(
            "[agent] Iteration {}/{}: sending invariant prompt...",
            self.iterations_used, self.max_iterations
        );

        let user_message = self.render_prompt(PromptTemplate::Invariant)?;
        self.push_user_msg(user_message);

        let response = self.send_to_claude()?;

        let refined_source = match self.receive_move_source(&response)? {
            Some(src) => src,
            None => return Ok(false),
        };

        // Write to temp file and re-run WP to see if vacuous conditions are resolved.
        let temp_dir = tempfile::TempDir::new()?;
        let temp_source = temp_dir.path().join("agent_invariant.move");
        std::fs::write(&temp_source, &refined_source)?;

        info!(
            "[agent] Iteration {}/{}: re-running WP on source with invariants...",
            self.iterations_used, self.max_iterations
        );

        let now = Instant::now();
        let wp_result = run_wp_on_source(&temp_source, &self.options, &self.experiments);
        self.tool_duration += now.elapsed();
        match wp_result {
            Ok(wp_source) => {
                if has_vacuous_conditions(&wp_source) {
                    info!(
                        "[agent] Iteration {}/{}: still has [vacuous] conditions",
                        self.iterations_used, self.max_iterations
                    );
                } else {
                    info!(
                        "[agent] Iteration {}/{}: vacuous conditions resolved!",
                        self.iterations_used, self.max_iterations
                    );
                }
                self.current_source = wp_source;
            },
            Err(e) => {
                // WP failed — likely a compilation error (e.g. old() misuse)
                // that compile_check didn't catch (bytecode pipeline errors).
                // Use the model's source to avoid re-entering invariant probing,
                // and set last_move_diagnostics so the failure loop sends the
                // MoveFailure prompt.
                warn!(
                    "[agent] Iteration {}/{}: WP failed (compilation error).\n\
                     Error: {:#}",
                    self.iterations_used, self.max_iterations, e
                );
                self.current_source = refined_source;
                self.last_move_diagnostics =
                    Some(format!("{}\n\n{:#}", COMPILATION_ERROR_PREFIX, e));
            },
        }
        Ok(true)
    }

    /// Fix a compilation error by sending the MoveFailure prompt, then re-run
    /// WP inference so function-level specs are re-derived (not left empty).
    fn fix_compilation_error(&mut self) -> anyhow::Result<()> {
        if !self.has_budget() {
            return Ok(());
        }
        self.iterations_used += 1;
        self.iteration = self.iterations_used;
        info!(
            "[agent] Iteration {}/{}: sending compilation error prompt...",
            self.iterations_used, self.max_iterations
        );

        let user_message = self.render_prompt(PromptTemplate::MoveFailure)?;
        self.push_user_msg(user_message);
        let response = self.send_to_claude()?;

        match self.receive_move_source(&response)? {
            Some(source) => {
                self.current_source = source;
                self.last_move_diagnostics = None;

                // Re-run WP to get fresh function-level specs.
                let temp_dir = tempfile::TempDir::new()?;
                let temp_source = temp_dir.path().join("agent_fix.move");
                std::fs::write(&temp_source, &self.current_source)?;

                info!(
                    "[agent] Iteration {}/{}: re-running WP after compilation fix...",
                    self.iterations_used, self.max_iterations
                );
                let now = Instant::now();
                let wp_result = run_wp_on_source(&temp_source, &self.options, &self.experiments);
                self.tool_duration += now.elapsed();
                match wp_result {
                    Ok(wp_source) => {
                        self.current_source = wp_source;
                    },
                    Err(e) => {
                        // WP still failing — set compilation error for next iteration.
                        warn!(
                            "[agent] Iteration {}/{}: WP still failing after fix.\n\
                             Error: {:#}",
                            self.iterations_used, self.max_iterations, e
                        );
                        self.last_move_diagnostics =
                            Some(format!("{}\n\n{:#}", COMPILATION_ERROR_PREFIX, e));
                    },
                }
            },
            None => {
                // Parse failed — keep last_move_diagnostics for retry.
                info!("[agent] Failed to parse compilation fix response.");
            },
        }
        Ok(())
    }

    /// Iterate failure/timeout prompts until verification passes or budget exhausted.
    /// Assumes `last_diagnostics` is populated from a prior failed `verify_current`.
    /// Returns true if verification eventually succeeded.
    fn run_failure_loop(&mut self) -> anyhow::Result<bool> {
        while self.has_budget() && self.verify_failure_attempts <= MAX_VERIFY_FAILURE_RETRIES {
            self.iterations_used += 1;
            self.iteration = self.iterations_used;
            self.timeout_attempt = self.timeout_attempts.min(self.max_timeout_attempts);

            // Pick the right prompt: compilation errors first, then
            // timeouts (if any timeout present — model should focus on fixing
            // timeouts first), then verification failures.
            let template = if self.last_move_diagnostics.is_some() {
                info!(
                    "[agent] Iteration {}/{}: sending compilation error prompt...",
                    self.iterations_used, self.max_iterations
                );
                PromptTemplate::MoveFailure
            } else if self.last_diagnostics.as_deref().is_some_and(has_timeout) {
                info!(
                    "[agent] Iteration {}/{}: sending timeout prompt (attempt {}/{})...",
                    self.iterations_used,
                    self.max_iterations,
                    self.timeout_attempt,
                    self.max_timeout_attempts
                );
                PromptTemplate::Timeout
            } else {
                info!(
                    "[agent] Iteration {}/{}: sending failure prompt...",
                    self.iterations_used, self.max_iterations
                );
                PromptTemplate::VerifyFailure
            };

            let user_message = self.render_prompt(template)?;
            self.push_user_msg(user_message);
            let response = self.send_to_claude()?;

            match self.receive_move_source(&response)? {
                Some(source) if source == self.current_source => {
                    let is_timeout = self.last_diagnostics.as_deref().is_some_and(has_timeout);
                    if is_timeout {
                        // Timeout: model couldn't find a better formulation.
                        // Jump to the final timeout iteration so it can add
                        // `pragma verify = false`.
                        info!(
                            "[agent] Model returned unchanged source on timeout — \
                             advancing to final timeout attempt."
                        );
                        self.timeout_attempts = self.max_timeout_attempts;
                        continue;
                    } else {
                        // Verification failure: model couldn't fix it. Give up.
                        info!(
                            "[agent] Model returned unchanged source on verification \
                             failure — giving up."
                        );
                        return Ok(false);
                    }
                },
                Some(source) => {
                    self.current_source = source;
                },
                None => {
                    self.last_diagnostics = Some(
                        "Failed to parse your response. \
                         Please return the complete source file inside a ```move code block."
                            .to_string(),
                    );
                    continue;
                },
            };

            if self.verify_current()? {
                return Ok(true);
            }
        }
        if self.verify_failure_attempts > MAX_VERIFY_FAILURE_RETRIES {
            info!(
                "[agent] Exhausted {} verification failure retries — giving up.",
                MAX_VERIFY_FAILURE_RETRIES
            );
        }
        Ok(false)
    }

    /// Send a simplification prompt to Claude and update `current_source`.
    /// Eliminates sathard quantifiers, removes redundant conditions, and
    /// improves readability before verification.
    /// Returns false if the response could not be parsed (current_source unchanged).
    fn send_simplify(&mut self) -> anyhow::Result<bool> {
        self.iterations_used += 1;
        self.iteration = self.iterations_used;
        info!(
            "[agent] Iteration {}/{}: simplifying specs...",
            self.iterations_used, self.max_iterations
        );

        let user_message = self.render_prompt(PromptTemplate::Simplify)?;
        self.push_user_msg(user_message);
        let response = self.send_to_claude()?;

        match self.receive_move_source(&response)? {
            Some(source) => {
                self.current_source = source;
                Ok(true)
            },
            None => Ok(false),
        }
    }

    /// Write the current source to the output location derived from `options`.
    fn write_output(&self) -> anyhow::Result<()> {
        if let Some(source) = self.options.move_sources.first() {
            let source_path = Path::new(source);
            let stem = source_path
                .file_stem()
                .expect("source file should have a stem");
            let output_path = if let Some(ref dir) = self.options.inference.inference_output_dir {
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
            std::fs::write(&output_path, &self.current_source)?;
            info!("[agent] Wrote refined source to {}", output_path.display());
        }
        Ok(())
    }
}

/// Run the agentic inference loop.
///
/// Strategy:
/// 1. Run WP-based inference to get initial enriched source.
/// 2. Each iteration, pick the right action based on source state:
///    - Vacuous conditions → invariant prompt + re-run WP.
///    - Otherwise → simplify (eliminates sathard quantifiers etc.) → verify.
///    - Verification fails → failure/timeout loop until it passes.
pub fn run_agent_loop(options: Options, experiments: Vec<String>) -> anyhow::Result<()> {
    let total_start = Instant::now();
    let mut agent = match Agent::new(options, experiments)? {
        Some(a) => a,
        None => return Ok(()),
    };

    let success = loop {
        if !agent.has_budget() {
            break false;
        }

        // Compilation errors from a previous step (e.g. WP failure after
        // adding invariants) — ask the model to fix, then re-run WP so
        // function-level specs are re-inferred (not left empty).
        if agent.last_move_diagnostics.is_some() {
            agent.fix_compilation_error()?;
            continue;
        }

        // Vacuous conditions need loop invariants before verification makes sense.
        if has_vacuous_conditions(&agent.current_source) {
            agent.send_invariant()?;
            continue;
        }

        // Simplify before verification: eliminates sathard quantifiers from
        // loop havoc, cleans up redundant conditions, improves readability.
        if agent.has_budget() && !agent.send_simplify()? {
            // Parse failed — proceed to verification with current source.
        }

        // Verify current source; fix failures until verification passes.
        if agent.verify_current()? {
            break true;
        }
        if agent.run_failure_loop()? {
            break true; // verification succeeded after fixing failures
        } else {
            break false; // budget exhausted during failure fixing
        }
    };

    info!(
        "[agent] {} iterations, {:.1}s total, {:.1}s claude, {:.1}s tools",
        agent.iterations_used,
        total_start.elapsed().as_secs_f64(),
        agent.claude_duration.as_secs_f64(),
        agent.tool_duration.as_secs_f64(),
    );

    agent.write_output()?;

    if success {
        Ok(())
    } else {
        let last_diagnostics = agent.last_diagnostics.as_deref().unwrap_or("(none)");
        info!(
            "[agent] Exhausted {} iterations without successful verification.",
            agent.max_iterations
        );
        info!("[agent] Last diagnostics:\n{}", last_diagnostics);
        Err(anyhow::anyhow!(
            "AI agent exhausted {} iterations without successful verification. \
             The last attempt has been written to the output. \
             Last diagnostics:\n{}",
            agent.max_iterations,
            last_diagnostics
        ))
    }
}

const COMPILATION_ERROR_PREFIX: &str =
    "COMPILATION ERROR — the source has syntax or type errors that must be fixed \
     before verification can run.\n\
     \n\
     HINT: A common cause is misuse of `old()` expressions. Review the \
     \"`old()` usage rules\" section (the table) in your instructions — `old()` is \
     forbidden in `aborts_if`, `requires`, and on non-parameter variables in loop \
     invariants.\n";

/// Run the Move Prover on a source file and return (success, diagnostics).
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

    run_inference(inf_options, experiments)?.ok_or_else(|| anyhow::anyhow!("WP produced no output"))
}
