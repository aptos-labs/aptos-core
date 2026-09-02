// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use super::super::session::FlowSession;
use crate::{candidate::CandidateCheckConfig, experiment::evaluate_candidate};
use rmcp::{
    handler::server::wrapper::Parameters,
    model::{CallToolResult, Content},
    schemars, tool, tool_router,
};

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
struct MovePackageCheckCandidateParams {
    /// Path to the Move package directory holding the candidate specification.
    package_path: String,
    /// Optional filter: `module_name` or `module_name::function_name`.
    /// When omitted, all target modules are checked. An evaluation session
    /// takes its target from the task instead, so this is ignored there.
    filter: Option<String>,
    /// Solver timeout per verification condition, in seconds. Default: 10.
    timeout: Option<usize>,
    /// Optional path to a pristine copy of the package, from before the
    /// specification work started. Given one, the check also reports whether
    /// the runtime implementation still compiles to the same bytecode.
    /// An evaluation session takes its baseline from the task instead.
    baseline_path: Option<String>,
}

/// Matches the verification tool's default so the two are interchangeable.
const DEFAULT_CHECK_TIMEOUT_SECS: usize = 10;

#[tool_router(router = spec_check_router, vis = "pub(crate)")]
impl FlowSession {
    // The evaluation task's baseline, target, editable paths, and required
    // contract categories come from the controller-supplied configuration, not
    // from the workspace, so a candidate cannot relax its own criteria.
    #[tool(
        description = "Run every acceptance check for the current specification task: \
                       compilation, complete target verification, contract-category \
                       coverage and forbidden weakening. Given a baseline, also reports \
                       whether the runtime implementation and the edit scope changed. \
                       Returns CANDIDATE_ACCEPTED when the task is done.",
        annotations(read_only_hint = true, destructive_hint = false)
    )]
    async fn move_spec_check(
        &self,
        Parameters(params): Parameters<MovePackageCheckCandidateParams>,
    ) -> Result<CallToolResult, rmcp::ErrorData> {
        log::info!("move_spec_check({})", params.package_path);
        // Below the acceptance level the task's criteria are withheld, so the
        // check falls back to what the package itself declares.
        let configured = self
            .args()
            .candidate_check
            .clone()
            .filter(|_| self.evaluation().task_criteria_enabled());
        let package = self.resolve_package_path(&params.package_path);
        let filter = params.filter.clone();
        let baseline = params
            .baseline_path
            .as_ref()
            .map(|path| self.resolve_package_path(path));
        // A per-VC budget beyond the tool's own deadline never helps the
        // caller, and would let solver work the caller has already stopped
        // waiting for outlive the response by that much.
        let timeout = params
            .timeout
            .unwrap_or(DEFAULT_CHECK_TIMEOUT_SECS)
            .min(self.tool_timeout().as_secs() as usize)
            .max(1);
        let telemetry = self.telemetry().clone();
        // Per-condition progress is what the `progress` feedback level adds.
        let report_conditions = self
            .evaluation()
            .feedback_level
            .condition_progress_enabled();
        let package_key = package.clone();
        // The check builds its own model rather than resolving through the
        // session cache, so it guards the manifest itself.
        self.refuse_remote_dependencies(std::path::Path::new(&package))?;
        let tool_timeout = self.tool_timeout();
        let verdict = tokio::time::timeout(
            tool_timeout,
            tokio::task::spawn_blocking(move || {
                let config = match configured {
                    // An evaluation task fixes the target and the contract
                    // categories, so a candidate cannot relax its own criteria.
                    Some(config_path) => {
                        let config = CandidateCheckConfig::load(&config_path)?;
                        anyhow::ensure!(
                            config.package.canonicalize().ok()
                                == std::path::Path::new(&package).canonicalize().ok(),
                            "the candidate check is configured for a different package"
                        );
                        config
                    },
                    None => {
                        let mut config = CandidateCheckConfig::for_package(
                            std::path::Path::new(&package),
                            filter.as_deref(),
                            timeout,
                        )?;
                        config.baseline = baseline.map(std::path::PathBuf::from);
                        config.report_conditions = report_conditions;
                        config
                    },
                };
                evaluate_candidate(&with_deadline(config, tool_timeout))
            }),
        )
        .await
        .map_err(|_| {
            rmcp::ErrorData::internal_error(
                format!("tool timeout ({}s exceeded)", tool_timeout.as_secs()),
                None,
            )
        })?
        .map_err(|error| rmcp::ErrorData::internal_error(error.to_string(), None))?;
        match verdict {
            Ok(verdict) => {
                telemetry.emit(
                    "candidate_check",
                    serde_json::json!({
                        "state": verdict.state,
                        "accepted": verdict.accepted,
                    }),
                );
                let mut rendered = verdict.render();
                if !verdict.conditions.is_empty() {
                    if let Some(progress) =
                        self.condition_progress(&package_key, &verdict.conditions)
                    {
                        rendered.push('\n');
                        rendered.push_str(&progress);
                    }
                }
                let content = vec![Content::text(rendered)];
                Ok(if verdict.accepted {
                    CallToolResult::success(content)
                } else {
                    CallToolResult::error(content)
                })
            },
            Err(error) => Ok(CallToolResult::error(vec![Content::text(format!(
                "candidate check could not run: {error:#}"
            ))])),
        }
    }
}

/// The tool answers within its deadline; the prover process must too, or the
/// work outlives the answer.
fn with_deadline(
    mut config: CandidateCheckConfig,
    tool_timeout: std::time::Duration,
) -> CandidateCheckConfig {
    config.process_deadline_seconds = Some(tool_timeout.as_secs().saturating_sub(1).max(1));
    config
}
