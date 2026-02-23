// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

//! AI-powered agentic spec inference for the Move Prover.
//!
//! When `--ai` is combined with `--inference`, the prover enters an agentic loop:
//! run WP-based inference to get initial specs, send source+specs+diagnostics to
//! Claude for refinement, verify the result, and iterate until success or exhaustion.
//!
//! Two strategies are available:
//! - **Model-driven** (default): A single comprehensive prompt is sent to the model,
//!   which autonomously drives the workflow by calling `verify` and `wp_inference` tools.
//! - **Agent-driven**: Rust code orchestrates a multi-phase loop with specific prompts
//!   at each phase.

pub mod agent_driven;
pub mod client;
pub mod common;
pub mod model_driven;
pub mod response_parser;
pub mod tools;

use clap::{Parser, ValueEnum};
use std::fmt;

/// Strategy for AI-powered spec inference.
#[derive(Debug, Clone, Default, ValueEnum)]
pub enum Strategy {
    /// Rust code orchestrates a multi-phase loop with specific prompts.
    AgentDriven,
    /// Model receives a single comprehensive prompt and drives the workflow via tools.
    #[default]
    ModelDriven,
}

impl fmt::Display for Strategy {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::AgentDriven => write!(f, "agent-driven"),
            Self::ModelDriven => write!(f, "model-driven"),
        }
    }
}

/// Options for the AI-powered agentic spec inference.
#[derive(Debug, Clone, Parser)]
pub struct AgentOptions {
    /// Enable AI-powered agentic spec inference (requires --inference).
    /// Optionally specify strategy: `--ai` (default: model-driven) or `--ai=agent-driven`.
    #[arg(
        long = "ai",
        value_enum,
        num_args = 0..=1,
        require_equals = true,
        default_missing_value = "model-driven"
    )]
    pub ai: Option<Strategy>,
    /// Maximum number of AI refinement iterations.
    #[arg(long = "ai-max-iterations", default_value_t = 10)]
    pub ai_max_iterations: usize,
    /// Claude model to use for AI inference.
    #[arg(long = "ai-model", default_value = "claude-opus-4-6")]
    pub ai_model: String,
    /// Maximum response tokens for AI inference.
    #[arg(long = "ai-max-tokens", default_value_t = 16384)]
    pub ai_max_tokens: usize,
    /// Optional custom system prompt file for AI inference.
    #[arg(long = "ai-system-prompt")]
    pub ai_system_prompt: Option<String>,
}

impl Default for AgentOptions {
    fn default() -> Self {
        Self {
            ai: None,
            ai_max_iterations: 10,
            ai_model: "claude-opus-4-6".to_string(),
            ai_max_tokens: 16384,
            ai_system_prompt: None,
        }
    }
}

/// Entry point for AI-powered agentic spec inference.
pub fn run_agent_inference(
    options: crate::cli::Options,
    experiments: Vec<String>,
) -> anyhow::Result<()> {
    match options.agent.ai {
        Some(Strategy::ModelDriven) => model_driven::run_model_driven_loop(options, experiments),
        _ => agent_driven::run_agent_loop(options, experiments),
    }
}
