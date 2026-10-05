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
pub const ABORTS_IF_IS_STRICT_ENV_VAR: &str = "MOVE_FLOW_ABORTS_IF_IS_STRICT";
pub const EXPECTED_ABORTS_IF_IS_STRICT_ENV_VAR: &str = "MOVE_FLOW_EXPECTED_ABORTS_IF_IS_STRICT";
pub const INFER_UNSPECIFIED_HELPERS_ENV_VAR: &str = "MOVE_FLOW_INFER_UNSPECIFIED_HELPERS";
pub const EXPECTED_INFER_UNSPECIFIED_HELPERS_ENV_VAR: &str =
    "MOVE_FLOW_EXPECTED_INFER_UNSPECIFIED_HELPERS";

/// Specification-inference workflow exposed by the generated plugin and MCP server.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, ValueEnum)]
#[serde(rename_all = "snake_case")]
pub enum InferenceTactic {
    /// Infer specifications directly, without access to the WP tool.
    AgentOnly,
    /// Follow the prescribed WP -> invariants -> WP -> simplify -> candidate-check workflow.
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
    /// Whether WP reports an abort characterization it cannot make exact as
    /// an error rather than emitting `aborts_if_is_partial`.
    pub aborts_if_is_strict: bool,
    /// Whether WP run on a single function also infers that function's
    /// callees which have no specification.
    pub infer_unspecified_helpers: bool,
}

/// Values of the evaluation settings as found in the environment, either the
/// settings themselves or the `EXPECTED_*` mirrors a generated plugin pins.
#[derive(Debug, Default)]
struct EnvironmentValues {
    inference_tactic: Option<OsString>,
    evaluation_mode: Option<OsString>,
    feedback_level: Option<OsString>,
    aborts_if_is_strict: Option<OsString>,
    infer_unspecified_helpers: Option<OsString>,
}

fn env_string(value: OsString, name: &str) -> Result<String> {
    value
        .into_string()
        .map_err(|_| anyhow::anyhow!("{name} is not valid UTF-8"))
}

fn env_bool(value: OsString, name: &str) -> Result<bool> {
    let value = env_string(value, name)?;
    parse_bool_env(&value).with_context(|| format!("invalid {name} value `{value}`"))
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
        aborts_if_is_strict: bool,
        infer_unspecified_helpers: bool,
    ) -> Result<Self> {
        Self::resolve_from_values(
            explicit_tactic,
            evaluation_mode,
            explicit_feedback_level,
            aborts_if_is_strict,
            infer_unspecified_helpers,
            EnvironmentValues {
                inference_tactic: std::env::var_os(INFERENCE_TACTIC_ENV_VAR),
                evaluation_mode: std::env::var_os(EVALUATION_MODE_ENV_VAR),
                feedback_level: std::env::var_os(FEEDBACK_LEVEL_ENV_VAR),
                aborts_if_is_strict: std::env::var_os(ABORTS_IF_IS_STRICT_ENV_VAR),
                infer_unspecified_helpers: std::env::var_os(INFER_UNSPECIFIED_HELPERS_ENV_VAR),
            },
        )
    }

    fn resolve_from_values(
        explicit_tactic: Option<InferenceTactic>,
        evaluation_mode: bool,
        explicit_feedback_level: Option<FeedbackLevel>,
        aborts_if_is_strict: bool,
        infer_unspecified_helpers: bool,
        environment: EnvironmentValues,
    ) -> Result<Self> {
        let inference_tactic = match (explicit_tactic, environment.inference_tactic) {
            (Some(tactic), _) => tactic,
            (None, Some(value)) => {
                InferenceTactic::parse_env(&env_string(value, INFERENCE_TACTIC_ENV_VAR)?)?
            },
            (None, None) => InferenceTactic::HybridGuided,
        };
        let evaluation_mode = match (evaluation_mode, environment.evaluation_mode) {
            (true, _) => true,
            (false, Some(value)) => env_bool(value, EVALUATION_MODE_ENV_VAR)?,
            (false, None) => false,
        };
        let feedback_level = match (explicit_feedback_level, environment.feedback_level) {
            (Some(level), _) => level,
            (None, Some(value)) => {
                FeedbackLevel::parse_env(&env_string(value, FEEDBACK_LEVEL_ENV_VAR)?)?
            },
            (None, None) => FeedbackLevel::Acceptance,
        };
        let aborts_if_is_strict = match (aborts_if_is_strict, environment.aborts_if_is_strict) {
            (true, _) => true,
            (false, Some(value)) => env_bool(value, ABORTS_IF_IS_STRICT_ENV_VAR)?,
            (false, None) => false,
        };
        let infer_unspecified_helpers = match (
            infer_unspecified_helpers,
            environment.infer_unspecified_helpers,
        ) {
            (true, _) => true,
            (false, Some(value)) => env_bool(value, INFER_UNSPECIFIED_HELPERS_ENV_VAR)?,
            (false, None) => false,
        };
        Ok(Self {
            inference_tactic,
            evaluation_mode,
            feedback_level,
            aborts_if_is_strict,
            infer_unspecified_helpers,
        })
    }

    /// Fail if a generated plugin pinned a different configuration than the
    /// one resolved at MCP startup (for example through MOVE_FLOW_ARGS).
    pub fn validate_expected(self) -> Result<()> {
        self.validate_expected_values(EnvironmentValues {
            inference_tactic: std::env::var_os(EXPECTED_INFERENCE_TACTIC_ENV_VAR),
            evaluation_mode: std::env::var_os(EXPECTED_EVALUATION_MODE_ENV_VAR),
            feedback_level: std::env::var_os(EXPECTED_FEEDBACK_LEVEL_ENV_VAR),
            aborts_if_is_strict: std::env::var_os(EXPECTED_ABORTS_IF_IS_STRICT_ENV_VAR),
            infer_unspecified_helpers: std::env::var_os(EXPECTED_INFER_UNSPECIFIED_HELPERS_ENV_VAR),
        })
    }

    fn validate_expected_values(self, expected: EnvironmentValues) -> Result<()> {
        if let Some(value) = expected.inference_tactic {
            let expected =
                InferenceTactic::parse_env(&env_string(value, EXPECTED_INFERENCE_TACTIC_ENV_VAR)?)?;
            if expected != self.inference_tactic {
                bail!(
                    "inference tactic mismatch: generated plugin expects `{expected}`, \
                     MCP resolved `{}`",
                    self.inference_tactic
                );
            }
        }
        if let Some(value) = expected.evaluation_mode {
            let expected = env_bool(value, EXPECTED_EVALUATION_MODE_ENV_VAR)?;
            if expected != self.evaluation_mode {
                bail!(
                    "evaluation mode mismatch: generated plugin expects `{expected}`, \
                     MCP resolved `{}`",
                    self.evaluation_mode
                );
            }
        }
        if let Some(value) = expected.feedback_level {
            let expected =
                FeedbackLevel::parse_env(&env_string(value, EXPECTED_FEEDBACK_LEVEL_ENV_VAR)?)?;
            if expected != self.feedback_level {
                bail!(
                    "feedback level mismatch: generated plugin expects `{expected}`, \
                     MCP resolved `{}`",
                    self.feedback_level
                );
            }
        }
        if let Some(value) = expected.aborts_if_is_strict {
            let expected = env_bool(value, EXPECTED_ABORTS_IF_IS_STRICT_ENV_VAR)?;
            if expected != self.aborts_if_is_strict {
                bail!(
                    "strict aborts mismatch: generated plugin expects `{expected}`, \
                     MCP resolved `{}`",
                    self.aborts_if_is_strict
                );
            }
        }
        if let Some(value) = expected.infer_unspecified_helpers {
            let expected = env_bool(value, EXPECTED_INFER_UNSPECIFIED_HELPERS_ENV_VAR)?;
            if expected != self.infer_unspecified_helpers {
                bail!(
                    "helper inference mismatch: generated plugin expects `{expected}`, \
                     MCP resolved `{}`",
                    self.infer_unspecified_helpers
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
        let config = EvaluationConfig::resolve_from_values(
            None,
            false,
            None,
            false,
            false,
            Default::default(),
        )
        .unwrap();
        assert_eq!(config.inference_tactic, InferenceTactic::HybridGuided);
        assert!(!config.evaluation_mode);
        assert_eq!(config.feedback_level, FeedbackLevel::Acceptance);
        assert!(!config.aborts_if_is_strict);
    }

    #[test]
    fn environment_overrides_default() {
        let config = EvaluationConfig::resolve_from_values(
            None,
            false,
            None,
            false,
            false,
            EnvironmentValues {
                inference_tactic: Some("agent_only".into()),
                evaluation_mode: Some("true".into()),
                feedback_level: Some("baseline".into()),
                aborts_if_is_strict: Some("1".into()),
                infer_unspecified_helpers: Some("1".into()),
            },
        )
        .unwrap();
        assert!(config.infer_unspecified_helpers);
        assert_eq!(config.inference_tactic, InferenceTactic::AgentOnly);
        assert!(config.evaluation_mode);
        assert_eq!(config.feedback_level, FeedbackLevel::Baseline);
        assert!(config.aborts_if_is_strict);
        assert!(!config.acceptance_check_enabled());
    }

    #[test]
    fn explicit_values_override_environment() {
        let config = EvaluationConfig::resolve_from_values(
            Some(InferenceTactic::HybridFlexible),
            true,
            Some(FeedbackLevel::Progress),
            true,
            true,
            EnvironmentValues {
                inference_tactic: Some("not-a-tactic".into()),
                evaluation_mode: Some("not-a-bool".into()),
                feedback_level: Some("not-a-level".into()),
                aborts_if_is_strict: Some("not-a-bool".into()),
                infer_unspecified_helpers: Some("not-a-bool".into()),
            },
        )
        .unwrap();
        assert_eq!(config.inference_tactic, InferenceTactic::HybridFlexible);
        assert!(config.evaluation_mode);
        assert!(config.aborts_if_is_strict);
    }

    #[test]
    fn invalid_environment_tactic_fails_clearly() {
        let error = EvaluationConfig::resolve_from_values(
            None,
            false,
            None,
            false,
            false,
            EnvironmentValues {
                inference_tactic: Some("not-a-tactic".into()),
                ..Default::default()
            },
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
            aborts_if_is_strict: false,
            infer_unspecified_helpers: false,
        };
        let error = config
            .validate_expected_values(EnvironmentValues {
                inference_tactic: Some("hybrid_guided".into()),
                evaluation_mode: Some("true".into()),
                ..Default::default()
            })
            .unwrap_err();
        assert!(error.to_string().contains("tactic mismatch"));
        let error = config
            .validate_expected_values(EnvironmentValues {
                feedback_level: Some("baseline".into()),
                ..Default::default()
            })
            .unwrap_err();
        assert!(error.to_string().contains("feedback level mismatch"));
        let error = config
            .validate_expected_values(EnvironmentValues {
                aborts_if_is_strict: Some("1".into()),
                ..Default::default()
            })
            .unwrap_err();
        assert!(error.to_string().contains("strict aborts mismatch"));
    }
}
