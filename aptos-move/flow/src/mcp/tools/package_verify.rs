// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use super::{
    super::{package_data::DiagnosticSource, session::FlowSession},
    resolve_excludes, resolve_filter,
};
use codespan_reporting::term::termcolor::NoColor;
use rmcp::{
    handler::server::wrapper::Parameters,
    model::{CallToolResult, Content},
    schemars, tool, tool_router,
};
use std::time::Instant;

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
struct MovePackageVerifyParams {
    /// Path to the Move package directory.
    package_path: String,
    /// Optional filter: `module_name` or `module_name::function_name`.
    /// When omitted, all target modules are verified.
    filter: Option<String>,
    /// Optional list of targets to exclude from verification.
    /// Each entry follows the same format as `filter`: `module_name` or
    /// `module_name::function_name`. Exclusions take precedence over the filter scope.
    exclude: Option<Vec<String>>,
    /// Solver timeout per verification condition, in seconds. Default: 10. Maximum: 10.
    timeout: Option<usize>,
}

const DEFAULT_VC_TIMEOUT: usize = 10;
const MAX_VC_TIMEOUT: usize = 60;

#[tool_router(router = package_verify_router, vis = "pub(crate)")]
impl FlowSession {
    // Low-level prover tool. Requires phased verification workflow context
    // (timeout handling, diagnostic interpretation) that is only available
    // through subagent delegation.
    #[tool(
        description = "Verify Move specifications using the Move Prover",
        annotations(read_only_hint = false, destructive_hint = false)
    )]
    async fn move_package_verify(
        &self,
        Parameters(params): Parameters<MovePackageVerifyParams>,
    ) -> Result<CallToolResult, rmcp::ErrorData> {
        log::info!(
            "move_package_verify({}, filter={:?}, exclude={:?}, timeout={:?})",
            params.package_path,
            params.filter,
            params.exclude,
            params.timeout
        );
        let (pkg, _) = self.resolve_package(&params.package_path).await?;
        let filter = params.filter.clone();
        let exclude = params.exclude.clone();
        let vc_timeout = params.timeout.unwrap_or(DEFAULT_VC_TIMEOUT);

        if vc_timeout > MAX_VC_TIMEOUT {
            return Ok(CallToolResult::error(vec![Content::text(
                "timeout is too high; read the instructions about timeout management \
                 in the verification agent guide",
            )]));
        }

        let tool_timeout = self.tool_timeout();
        let result = tokio::time::timeout(
            tool_timeout,
            tokio::task::spawn_blocking(move || {
                let mut data = pkg.lock().unwrap();

                // 1. Check for compilation errors.
                if data.has_compilation_errors() {
                    return Ok(CallToolResult::error(vec![Content::text(
                        "package has compilation errors; run move_package_status for details",
                    )]));
                }

                // 2. Resolve filter into (VerifiedScope, VerificationScope).
                let (scope, verification_scope) = resolve_filter(data.env(), filter.as_deref())?;
                let verify_exclude = resolve_excludes(exclude.as_deref());
                let has_excludes = !verify_exclude.is_empty();

                // 4. Check cache (skip when exclusions are active — cached results
                //    don't account for the exclusion set).
                //    - Success propagates via entailment (wider success ⇒ narrower success).
                //    - Errors only reuse an exact scope match; a wider-scope failure does
                //      not imply a narrower scope also fails (the error may be elsewhere).
                if !has_excludes {
                    if let Some((ref cached_scope, success, cached_timeout)) = data.verified() {
                        let hit = if success {
                            cached_scope.entails_success(&scope) && vc_timeout >= cached_timeout
                        } else {
                            cached_scope.entails_error(&scope) && vc_timeout <= cached_timeout
                        };
                        if hit {
                            let msg = if success {
                                "verification succeeded (cached result)"
                            } else {
                                "verification failed (cached result)"
                            };
                            log::info!("move_package_verify: cache hit, success={}", success);
                            return Ok(if success {
                                CallToolResult::success(vec![Content::text(msg)])
                            } else {
                                CallToolResult::error(vec![Content::text(msg)])
                            });
                        }
                    }
                }

                // 5. Build prover options.
                let temp_dir = tempfile::tempdir().map_err(|e| {
                    rmcp::ErrorData::internal_error(
                        format!("failed to create temp dir: {}", e),
                        None,
                    )
                })?;
                let mut options = move_prover::cli::Options::default();
                options.prover.verify_scope = verification_scope;
                options.prover.verify_exclude = verify_exclude;
                options.backend.vc_timeout = vc_timeout;
                #[cfg(test)]
                {
                    options.prover.stable_test_output = true;
                    options.backend.stable_test_output = true;
                }
                options.output_path = temp_dir
                    .path()
                    .join("output.bpl")
                    .to_string_lossy()
                    .into_owned();

                // 6. Clear leftover diagnostics from previous runs, then run the prover.
                data.env().clear_diag();
                let mut error_writer = NoColor::new(Vec::new());
                let prover_result = move_prover::run_move_prover_with_model_v2(
                    data.env_mut(),
                    &mut error_writer,
                    options,
                    Instant::now(),
                );

                match prover_result {
                    Ok(()) => {
                        // Only cache when no exclusions — an excluded-scope result
                        // doesn't represent the full scope and must not be reused.
                        if !has_excludes {
                            data.set_verified(scope, true, vc_timeout);
                        }
                        log::info!("move_package_verify: succeeded");
                        Ok(CallToolResult::success(vec![Content::text(
                            "verification succeeded",
                        )]))
                    },
                    Err(e) => {
                        if !has_excludes {
                            data.set_verified(scope, false, vc_timeout);
                        }
                        let diag_text =
                            String::from_utf8(error_writer.into_inner()).unwrap_or_default();
                        let msg = if !diag_text.is_empty() {
                            data.set_diagnostics(DiagnosticSource::Verifier, vec![
                                diag_text.clone()
                            ]);
                            format!("verification failed:\n{}", diag_text)
                        } else {
                            // The prover may return errors (e.g. tool version
                            // mismatch, boogie crash) that bypass the diagnostic
                            // writer. Try unreported env diagnostics first, then
                            // fall back to the anyhow error.
                            let env_diags =
                                super::super::package_data::render_diagnostics(data.env());
                            if !env_diags.is_empty() {
                                let joined = env_diags.join("\n");
                                data.set_diagnostics(DiagnosticSource::Verifier, env_diags);
                                format!("verification failed:\n{}", joined)
                            } else {
                                let err_msg = format!("{:#}", e);
                                data.set_diagnostics(DiagnosticSource::Verifier, vec![
                                    err_msg.clone()
                                ]);
                                format!("verification failed: {}", err_msg)
                            }
                        };
                        log::info!("move_package_verify: failed\n{}", msg);
                        Ok(CallToolResult::error(vec![Content::text(msg)]))
                    },
                }
            }),
        )
        .await
        .map_err(|_| {
            self.invalidate_package(&params.package_path);
            rmcp::ErrorData::internal_error(
                format!("tool timeout ({}s exceeded)", tool_timeout.as_secs()),
                None,
            )
        })?
        .map_err(|e| {
            rmcp::ErrorData::internal_error(format!("verify task panicked: {}", e), None)
        })??;

        Ok(result)
    }
}
