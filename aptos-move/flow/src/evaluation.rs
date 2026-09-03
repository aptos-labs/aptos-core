// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use anyhow::{bail, Context, Result};
use clap::ValueEnum;
use serde::Serialize;
use sha2::{Digest, Sha256};
use std::ffi::OsString;

pub const INFERENCE_TACTIC_ENV_VAR: &str = "MOVE_FLOW_INFERENCE_TACTIC";
pub const EVALUATION_MODE_ENV_VAR: &str = "MOVE_FLOW_EVALUATION_MODE";
pub const EXPECTED_INFERENCE_TACTIC_ENV_VAR: &str = "MOVE_FLOW_EXPECTED_INFERENCE_TACTIC";
pub const EXPECTED_EVALUATION_MODE_ENV_VAR: &str = "MOVE_FLOW_EXPECTED_EVALUATION_MODE";
pub const EXPECTED_TOOL_LIST_SHA256_ENV_VAR: &str = "MOVE_FLOW_EXPECTED_TOOL_LIST_SHA256";
pub const SOURCE_COMMIT_ENV_VAR: &str = "MOVE_FLOW_SOURCE_COMMIT";
pub const FEEDBACK_LEVEL_ENV_VAR: &str = "MOVE_FLOW_FEEDBACK_LEVEL";
pub const EXPECTED_FEEDBACK_LEVEL_ENV_VAR: &str = "MOVE_FLOW_EXPECTED_FEEDBACK_LEVEL";

/// Specification-inference workflow exposed by the generated plugin and MCP server.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, ValueEnum)]
#[serde(rename_all = "snake_case")]
pub enum InferenceTactic {
    /// Infer specifications directly, without access to the WP tool.
    AgentOnly,
    /// Follow the prescribed invariants -> WP -> simplify -> verify workflow.
    HybridGuided,
    /// Make WP available while leaving orchestration to the agent.
    HybridFlexible,
}

impl InferenceTactic {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::AgentOnly => "agent_only",
            Self::HybridGuided => "hybrid_guided",
            Self::HybridFlexible => "hybrid_flexible",
        }
    }

    pub fn wp_tool_enabled(self) -> bool {
        !matches!(self, Self::AgentOnly)
    }

    pub fn guided_workflow(self) -> bool {
        matches!(self, Self::HybridGuided)
    }

    fn parse_env(value: &str) -> Result<Self> {
        match value {
            "agent_only" | "agent-only" => Ok(Self::AgentOnly),
            "hybrid_guided" | "hybrid-guided" => Ok(Self::HybridGuided),
            "hybrid_flexible" | "hybrid-flexible" => Ok(Self::HybridFlexible),
            _ => bail!(
                "invalid {INFERENCE_TACTIC_ENV_VAR} value `{value}`; expected one of: \
                 agent_only, hybrid_guided, hybrid_flexible"
            ),
        }
    }
}

impl std::fmt::Display for InferenceTactic {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

/// How much deterministic feedback the session gives the agent after an edit.
///
/// The levels are cumulative and exist so that one mechanism at a time can be
/// added to an otherwise identical apparatus. `Baseline` reproduces the
/// apparatus as it stood before the feedback work.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, ValueEnum)]
#[serde(rename_all = "snake_case")]
pub enum FeedbackLevel {
    /// Compiler and prover answers only.
    Baseline,
    /// Adds the candidate acceptance check, timeout attribution, and the
    /// shared toolchain reference.
    Acceptance,
    /// Adds bounded loop-invariant evidence on the verification-failure path.
    Diagnostics,
    /// Adds per-condition progress deltas.
    Progress,
}

impl FeedbackLevel {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Baseline => "baseline",
            Self::Acceptance => "acceptance",
            Self::Diagnostics => "diagnostics",
            Self::Progress => "progress",
        }
    }

    /// Whether the agent-visible acceptance check and its supporting
    /// reference material are part of this apparatus.
    pub fn acceptance_check_enabled(self) -> bool {
        self >= Self::Acceptance
    }

    /// Whether the session reports per-condition progress between attempts.
    pub fn condition_progress_enabled(self) -> bool {
        self >= Self::Progress
    }

    fn parse_env(value: &str) -> Result<Self> {
        match value {
            "baseline" => Ok(Self::Baseline),
            "acceptance" => Ok(Self::Acceptance),
            "diagnostics" => Ok(Self::Diagnostics),
            "progress" => Ok(Self::Progress),
            _ => bail!(
                "invalid {FEEDBACK_LEVEL_ENV_VAR} value `{value}`; expected one of: \
                 baseline, acceptance, diagnostics, progress"
            ),
        }
    }
}

impl std::fmt::Display for FeedbackLevel {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

/// Back-edge traversals reported when loop-invariant evidence is enabled.
///
/// Not gated by feedback level: the evidence explains *why* a loop needs an
/// invariant rather than supplying one, so withholding it makes a diagnostic
/// worse without making the task harder in any way worth measuring. Three is
/// enough to expose a linear or geometric accumulator; deeper output grows
/// combinatorially once a loop's update is data-dependent.
pub const LOOP_INVARIANT_EVIDENCE_DEPTH: usize = 3;

/// Fully resolved evaluation settings. CLI values take precedence over the
/// environment; the ordinary plugin remains guided and non-evaluation by default.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct EvaluationConfig {
    pub inference_tactic: InferenceTactic,
    pub evaluation_mode: bool,
    pub feedback_level: FeedbackLevel,
}

impl EvaluationConfig {
    /// Whether this session exposes the agent-visible acceptance check.
    pub fn acceptance_check_enabled(self) -> bool {
        self.evaluation_mode && self.feedback_level.acceptance_check_enabled()
    }

    /// Whether the hybrid tactic may be chosen per skill invocation.
    ///
    /// The two hybrid tactics share one tool inventory, so outside an
    /// evaluation a hybrid plugin carries both and an invocation may name
    /// either; the rendered one is the default. The direct tactic is its own
    /// plugin, because it must not serve the WP tool at all, and an
    /// evaluation pins the tactic, because the arm is the treatment.
    pub fn tactic_selectable(self) -> bool {
        !self.evaluation_mode && self.inference_tactic.wp_tool_enabled()
    }

    /// Whether the WP tool is served: never by a direct-tactic plugin.
    pub fn wp_tool_enabled(self) -> bool {
        self.inference_tactic.wp_tool_enabled()
    }

    /// Whether a configured candidate check supplies the task's criteria.
    ///
    /// The task's fixed target, edit scope and required contract categories
    /// are the acceptance intervention. An evaluation below the acceptance
    /// level must not receive them, or the control would get the treatment's
    /// feedback; the check tool stays, with the package's own defaults.
    /// Outside an evaluation a caller who passes the configuration means it.
    pub fn task_criteria_enabled(self) -> bool {
        !self.evaluation_mode || self.feedback_level.acceptance_check_enabled()
    }

    /// Whether an uninvariant loop is an error rather than a warning.
    ///
    /// WP drops the conditions a loop havoc left unconstrained and emits an
    /// empty `aborts_if_is_partial` contract, with the reason in a warning.
    /// That is the right default: a person reading the warning can still use
    /// what WP derived. A measured round cannot, because the empty contract
    /// compiles and verifies, so anything that only asks whether the prover
    /// succeeded cannot tell it from a complete specification -- which is the
    /// exact failure the study exists to detect.
    pub fn uninvariant_loop_is_error(self) -> bool {
        self.evaluation_mode
    }

    /// Whether the transaction-replay tool is served.
    ///
    /// Not in an evaluation session. Replay reaches an arbitrary REST endpoint
    /// and sends a caller-supplied key as a bearer token, so it is a network
    /// egress channel; a measured session denies `Bash`, `WebFetch` and
    /// `WebSearch` precisely to have none, and it has no use for replay while
    /// specifying a package. See `evaluation/spec-inference/sandbox/README.md`.
    pub fn replay_tool_enabled(self) -> bool {
        !self.evaluation_mode
    }
}

impl EvaluationConfig {
    pub fn resolve(
        explicit_tactic: Option<InferenceTactic>,
        evaluation_mode: bool,
        explicit_feedback_level: Option<FeedbackLevel>,
    ) -> Result<Self> {
        Self::resolve_from_values(
            explicit_tactic,
            std::env::var_os(INFERENCE_TACTIC_ENV_VAR),
            evaluation_mode,
            std::env::var_os(EVALUATION_MODE_ENV_VAR),
            explicit_feedback_level,
            std::env::var_os(FEEDBACK_LEVEL_ENV_VAR),
        )
    }

    fn resolve_from_values(
        explicit_tactic: Option<InferenceTactic>,
        environment_tactic: Option<OsString>,
        evaluation_mode: bool,
        environment_evaluation_mode: Option<OsString>,
        explicit_feedback_level: Option<FeedbackLevel>,
        environment_feedback_level: Option<OsString>,
    ) -> Result<Self> {
        let inference_tactic = match explicit_tactic {
            Some(tactic) => tactic,
            None => match environment_tactic {
                Some(value) => {
                    let value = value.into_string().map_err(|_| {
                        anyhow::anyhow!("{INFERENCE_TACTIC_ENV_VAR} is not valid UTF-8")
                    })?;
                    InferenceTactic::parse_env(&value)?
                },
                None => InferenceTactic::HybridGuided,
            },
        };

        let evaluation_mode = if evaluation_mode {
            true
        } else {
            match environment_evaluation_mode {
                Some(value) => {
                    let value = value.into_string().map_err(|_| {
                        anyhow::anyhow!("{EVALUATION_MODE_ENV_VAR} is not valid UTF-8")
                    })?;
                    parse_bool_env(&value).with_context(|| {
                        format!("invalid {EVALUATION_MODE_ENV_VAR} value `{value}`")
                    })?
                },
                None => false,
            }
        };

        let feedback_level = match explicit_feedback_level {
            Some(level) => level,
            None => match environment_feedback_level {
                Some(value) => {
                    let value = value.into_string().map_err(|_| {
                        anyhow::anyhow!("{FEEDBACK_LEVEL_ENV_VAR} is not valid UTF-8")
                    })?;
                    FeedbackLevel::parse_env(&value)?
                },
                None => FeedbackLevel::Acceptance,
            },
        };

        Ok(Self {
            inference_tactic,
            evaluation_mode,
            feedback_level,
        })
    }

    /// Fail if a generated plugin pinned a different configuration than the
    /// one resolved at MCP startup (for example through MOVE_FLOW_ARGS).
    pub fn validate_expected(self) -> Result<()> {
        self.validate_expected_values(
            std::env::var_os(EXPECTED_INFERENCE_TACTIC_ENV_VAR),
            std::env::var_os(EXPECTED_EVALUATION_MODE_ENV_VAR),
            std::env::var_os(EXPECTED_FEEDBACK_LEVEL_ENV_VAR),
        )
    }

    fn validate_expected_values(
        self,
        expected_tactic: Option<OsString>,
        expected_evaluation_mode: Option<OsString>,
        expected_feedback_level: Option<OsString>,
    ) -> Result<()> {
        if let Some(value) = expected_tactic {
            let value = value.into_string().map_err(|_| {
                anyhow::anyhow!("{EXPECTED_INFERENCE_TACTIC_ENV_VAR} is not valid UTF-8")
            })?;
            let expected = InferenceTactic::parse_env(&value)?;
            if expected != self.inference_tactic {
                bail!(
                    "inference tactic mismatch: generated plugin expects `{expected}`, \
                     MCP resolved `{}`",
                    self.inference_tactic
                );
            }
        }
        if let Some(value) = expected_evaluation_mode {
            let value = value.into_string().map_err(|_| {
                anyhow::anyhow!("{EXPECTED_EVALUATION_MODE_ENV_VAR} is not valid UTF-8")
            })?;
            let expected = parse_bool_env(&value).with_context(|| {
                format!("invalid {EXPECTED_EVALUATION_MODE_ENV_VAR} value `{value}`")
            })?;
            if expected != self.evaluation_mode {
                bail!(
                    "evaluation mode mismatch: generated plugin expects `{expected}`, \
                     MCP resolved `{}`",
                    self.evaluation_mode
                );
            }
        }
        if let Some(value) = expected_feedback_level {
            let value = value.into_string().map_err(|_| {
                anyhow::anyhow!("{EXPECTED_FEEDBACK_LEVEL_ENV_VAR} is not valid UTF-8")
            })?;
            let expected = FeedbackLevel::parse_env(&value)?;
            if expected != self.feedback_level {
                bail!(
                    "feedback level mismatch: generated plugin expects `{expected}`, \
                     MCP resolved `{}`",
                    self.feedback_level
                );
            }
        }
        Ok(())
    }
}

pub(crate) fn sha256_hex(bytes: &[u8]) -> String {
    let digest = Sha256::digest(bytes);
    digest.iter().map(|byte| format!("{byte:02x}")).collect()
}

fn parse_bool_env(value: &str) -> Result<bool> {
    match value {
        "1" | "true" | "yes" | "on" => Ok(true),
        "0" | "false" | "no" | "off" => Ok(false),
        _ => bail!("expected one of: 1, true, yes, on, 0, false, no, off"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults_to_guided_non_evaluation() {
        let config =
            EvaluationConfig::resolve_from_values(None, None, false, None, None, None).unwrap();
        assert_eq!(config.inference_tactic, InferenceTactic::HybridGuided);
        assert!(!config.evaluation_mode);
        assert_eq!(config.feedback_level, FeedbackLevel::Acceptance);
    }

    #[test]
    fn environment_overrides_default() {
        let config = EvaluationConfig::resolve_from_values(
            None,
            Some("agent_only".into()),
            false,
            Some("true".into()),
            None,
            Some("baseline".into()),
        )
        .unwrap();
        assert_eq!(config.inference_tactic, InferenceTactic::AgentOnly);
        assert!(config.evaluation_mode);
        assert_eq!(config.feedback_level, FeedbackLevel::Baseline);
        assert!(!config.acceptance_check_enabled());
    }

    #[test]
    fn explicit_values_override_environment() {
        let config = EvaluationConfig::resolve_from_values(
            Some(InferenceTactic::HybridFlexible),
            Some("not-a-tactic".into()),
            true,
            Some("not-a-bool".into()),
            Some(FeedbackLevel::Progress),
            Some("not-a-level".into()),
        )
        .unwrap();
        assert_eq!(config.inference_tactic, InferenceTactic::HybridFlexible);
        assert!(config.evaluation_mode);
    }

    #[test]
    fn invalid_environment_tactic_fails_clearly() {
        let error = EvaluationConfig::resolve_from_values(
            None,
            Some("not-a-tactic".into()),
            false,
            None,
            None,
            None,
        )
        .unwrap_err();
        assert!(error.to_string().contains(INFERENCE_TACTIC_ENV_VAR));
        assert!(error.to_string().contains("hybrid_flexible"));
    }

    #[test]
    fn expected_configuration_mismatch_fails() {
        let config = EvaluationConfig {
            inference_tactic: InferenceTactic::AgentOnly,
            evaluation_mode: true,
            feedback_level: FeedbackLevel::Acceptance,
        };
        let error = config
            .validate_expected_values(Some("hybrid_guided".into()), Some("true".into()), None)
            .unwrap_err();
        assert!(error.to_string().contains("tactic mismatch"));
        let error = config
            .validate_expected_values(None, None, Some("baseline".into()))
            .unwrap_err();
        assert!(error.to_string().contains("feedback level mismatch"));
    }
}
