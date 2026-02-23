// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

//! Tera-based prompt builder for the agentic spec inference loop.

use super::loop_driver::Agent;
use anyhow::Context;
use tera::{self, Tera};

const SYSTEM_PROMPT_TEMPLATE: &str = include_str!("system_prompt.md");
const VERIFY_FAILURE_PROMPT_TEMPLATE: &str = include_str!("verify_failure_prompt.md");
const MOVE_FAILURE_PROMPT_TEMPLATE: &str = include_str!("move_failure_prompt.md");
const INVARIANT_PROMPT_TEMPLATE: &str = include_str!("invariant_prompt.md");
const TIMEOUT_PROMPT_TEMPLATE: &str = include_str!("timeout_prompt.md");
const SIMPLIFY_PROMPT_TEMPLATE: &str = include_str!("simplify_prompt.md");

/// Identifies which prompt template to render.
pub(super) enum PromptTemplate {
    /// Verification failure fix prompt.
    VerifyFailure,
    /// Move compilation failure fix prompt.
    MoveFailure,
    /// Loop-invariant probing prompt.
    Invariant,
    /// Timeout reformulation prompt.
    Timeout,
    /// Post-verification simplification prompt.
    Simplify,
}

impl PromptTemplate {
    /// Tera template name used for registration and lookup.
    fn name(&self) -> &'static str {
        match self {
            Self::VerifyFailure => "verify_failure_prompt",
            Self::MoveFailure => "move_failure_prompt",
            Self::Invariant => "invariant_prompt",
            Self::Timeout => "timeout_prompt",
            Self::Simplify => "simplify_prompt",
        }
    }
}

/// Builds prompts for the Claude API using Tera templates.
pub struct PromptBuilder {
    system_prompt: String,
    tera: Tera,
}

impl PromptBuilder {
    /// Create a new prompt builder.
    ///
    /// If `custom_system_prompt_file` is provided, its contents are used as the
    /// system prompt instead of the built-in template.
    pub fn new(custom_system_prompt_file: Option<&str>) -> anyhow::Result<Self> {
        let system_prompt = if let Some(path) = custom_system_prompt_file {
            std::fs::read_to_string(path)
                .with_context(|| format!("failed to read custom system prompt from {}", path))?
        } else {
            SYSTEM_PROMPT_TEMPLATE.to_string()
        };

        let mut tera = Tera::default();
        tera.add_raw_templates(vec![
            ("verify_failure_prompt", VERIFY_FAILURE_PROMPT_TEMPLATE),
            ("move_failure_prompt", MOVE_FAILURE_PROMPT_TEMPLATE),
            ("invariant_prompt", INVARIANT_PROMPT_TEMPLATE),
            ("timeout_prompt", TIMEOUT_PROMPT_TEMPLATE),
            ("simplify_prompt", SIMPLIFY_PROMPT_TEMPLATE),
        ])
        .context("failed to compile prompt templates")?;

        Ok(Self {
            system_prompt,
            tera,
        })
    }

    /// Return the system prompt string.
    pub fn system_prompt(&self) -> &str {
        &self.system_prompt
    }

    /// Render a prompt template with `agent` as the context variable.
    ///
    /// Templates access agent fields via `{{ agent.field_name }}`.
    pub(super) fn render(&self, template: PromptTemplate, agent: &Agent) -> anyhow::Result<String> {
        let name = template.name();
        let mut context = tera::Context::new();
        context.insert("agent", agent);
        self.tera
            .render(name, &context)
            .with_context(|| format!("failed to render {} template", name))
    }
}
