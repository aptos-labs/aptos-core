// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use super::{
    super::{
        package_data::{inspect_diagnostics, DiagnosticSource, VerifiedScope},
        session::FlowSession,
    },
    load_sanitized_prover_options, module_part_of, resolve_excludes, resolve_filter,
};
use crate::{
    evaluation::LOOP_INVARIANT_EVIDENCE_DEPTH,
    experiment::{attribute_prover_timeout, is_prover_timeout, scoped_function_names},
};
use codespan_reporting::term::termcolor::NoColor;
use move_model::{ast::ExpData, model::GlobalEnv};
use move_prover_bytecode_pipeline::spec_inference::LOOP_INVARIANT_EVIDENCE_NOTE;
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
    /// Solver timeout per verification condition, in seconds. Default: 10. Maximum: 60.
    timeout: Option<usize>,
    /// If set, generate an independent verification condition for each
    /// assertion in a function instead of a single combined condition. Can
    /// help when a function mixes provable-but-hard asserts with asserts
    /// that produce counterexamples; useful for diagnosing per-function
    /// timeouts.
    split_vcs_by_assert: Option<bool>,
    /// Maximum number of counterexamples reported per verification condition.
    /// The request also has a fixed aggregate diagnostic limit.
    error_limit: Option<usize>,
}

const DEFAULT_VC_TIMEOUT: usize = 10;
const MAX_VC_TIMEOUT: usize = 60;
const MCP_PROCESS_TIMEOUT_GRACE_SECS: u64 = 10;
/// Multiple of the solver budget the watchdog allows before killing Boogie.
/// Matches the prover backend's own proportional watchdog.
const PROCESS_TIMEOUT_FACTOR: u64 = 4;
const MAX_VC_ERROR_LIMIT: usize = 20;
const MAX_PACKAGE_ERROR_LIMIT: usize = 20;
/// Below this much remaining tool budget, attribution cannot finish a probe and
/// would only delay the diagnostic the caller already has.
const MIN_ATTRIBUTION_BUDGET_SECS: u64 = 20;

/// Attribute a prover timeout to individual functions, or explain why not.
///
/// Returns an empty string when no scope is known, so a filterless whole-package
/// call is reported unchanged rather than probing every function in the package.
fn attribute_timeout(
    env: &GlobalEnv,
    package: &std::path::Path,
    filter: &Option<String>,
    vc_timeout: usize,
    budget: std::time::Duration,
) -> String {
    let Some(filter) = filter else {
        return String::new();
    };
    // Naming the functions in scope needs a model, and the session holds one.
    let scoped = scoped_function_names(env, Some(filter));
    if budget < std::time::Duration::from_secs(MIN_ATTRIBUTION_BUDGET_SECS) {
        return String::new();
    }
    match attribute_prover_timeout(package, scoped, vc_timeout, budget) {
        Ok(attribution) => format!("\n\n{}", attribution.render()),
        Err(error) => format!("\n\ntimeout attribution unavailable: {error:#}"),
    }
}

/// How many errors the prover recorded on the model.
///
/// Read from the diagnostic records rather than counted out of rendered text:
/// a message that happens to contain the word "error" is not an error, and a
/// multi-line one is not several.
fn count_reported_errors(env: &GlobalEnv) -> usize {
    inspect_diagnostics(env)
        .iter()
        .filter(|record| record.is_error)
        .count()
}

/// Whether any function the filter selects contains a loop.
///
/// Answered from the model the session already holds, so a scope with no loop
/// costs nothing to rule out.
fn scope_contains_loop(env: &GlobalEnv, filter: &Option<String>) -> bool {
    let selected = match resolve_filter(env, filter.as_deref()) {
        Ok((scope, _)) => scope,
        // An unresolvable filter is reported elsewhere; do not suppress
        // evidence on account of it.
        Err(_) => return true,
    };
    env.get_modules()
        .filter(|module| module.is_target())
        .flat_map(|module| module.into_functions())
        .filter(|function| match &selected {
            VerifiedScope::Package => true,
            VerifiedScope::Module(id) => function.module_env.get_id() == *id,
            VerifiedScope::Function(id) => function.get_qualified_id() == *id,
        })
        .any(|function| {
            function.get_def().is_some_and(|def| {
                let mut found = false;
                def.visit_post_order(&mut |exp| {
                    if matches!(exp, ExpData::Loop(..)) {
                        found = true;
                    }
                    true
                });
                found
            })
        })
}

/// Bounded loop-invariant evidence for a failed verification.
///
/// Inference runs on a throwaway model and writes nothing: only its
/// diagnostics are kept.
fn loop_invariant_evidence(
    package: &std::path::Path,
    filter: &Option<String>,
    depth: usize,
) -> String {
    let mut options = match load_sanitized_prover_options(package) {
        Ok(options) => options,
        Err(_) => return String::new(),
    };
    options.inference.inference = true;
    // Inference derives `prover.loop_invariant_evidence_depth` from this field
    // on every run, so setting the prover field directly would be overwritten.
    options.inference.loop_invariant_evidence = Some(depth);
    // Inference writes its result somewhere by design, and the default is
    // stdout -- the MCP transport. Send it to a directory discarded with this
    // function; only the diagnostics are wanted.
    let Ok(temporary) = tempfile::tempdir() else {
        return String::new();
    };
    options.inference.inference_output = move_prover::inference::InferenceOutput::File;
    options.inference.inference_output_dir = Some(temporary.path().to_string_lossy().into_owned());
    options.output_path = temporary
        .path()
        .join("output.bpl")
        .to_string_lossy()
        .into_owned();
    let mut env = match crate::experiment::build_model(package) {
        Ok(env) => env,
        Err(_) => return String::new(),
    };
    if let Some(filter) = filter {
        match resolve_filter(&env, Some(filter.as_str())) {
            Ok((_, scope)) => options.prover.verify_scope = scope,
            Err(_) => return String::new(),
        }
    }
    let mut writer = NoColor::new(Vec::new());
    let started = std::time::Instant::now();
    // Inference fails when a loop left a condition `vacuous` and warns when it
    // only made one `sathard`; the diagnostic is recorded on the model either
    // way, so the writer is only there to satisfy the signature.
    let _ = move_prover::inference::run_spec_inference_with_model(
        &mut env,
        &mut writer,
        options,
        started,
    );
    // Keep only the evidence, and take it as the notes the inference pass
    // attached rather than by cutting up its rendering. The inferred
    // conditions themselves are not a contract and must not be shown as one.
    let evidence: Vec<String> = inspect_diagnostics(&env)
        .into_iter()
        .filter(|record| {
            record
                .notes
                .first()
                .is_some_and(|note| note.starts_with(LOOP_INVARIANT_EVIDENCE_NOTE))
        })
        .map(|record| {
            // Say which loop. The notes describe one loop head and the record
            // knows where it is, so the caller does not have to guess.
            let location = match (&record.file, record.line) {
                (Some(file), Some(line)) => format!("{file}:{line}: "),
                _ => String::new(),
            };
            format!("{location}{}", record.notes.join("\n"))
        })
        .collect();
    if evidence.is_empty() {
        String::new()
    } else {
        format!("\n\n{}", evidence.join("\n\n"))
    }
}

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
            "move_package_verify({}, filter={:?}, exclude={:?}, timeout={:?}, \
             split_vcs_by_assert={:?}, error_limit={:?})",
            params.package_path,
            params.filter,
            params.exclude,
            params.timeout,
            params.split_vcs_by_assert,
            params.error_limit
        );
        // Loop-invariant evidence rebuilds from disk on the failure path, so
        // the manifest is guarded here and not only on a cache miss.
        self.refuse_remote_dependencies(std::path::Path::new(
            &self.resolve_package_path(&params.package_path),
        ))?;
        let (pkg, _) = self.resolve_package(&params.package_path).await?;
        let filter = params.filter.clone();
        let verification_filter = params.filter.clone();
        let attribute_timeouts = self.evaluation().feedback_level.acceptance_check_enabled();
        let evidence_depth = Some(LOOP_INVARIANT_EVIDENCE_DEPTH);
        let exclude = params.exclude.clone();
        let vc_timeout = params.timeout.unwrap_or(DEFAULT_VC_TIMEOUT);
        let split_vcs_by_assert = params.split_vcs_by_assert.unwrap_or(false);
        let error_limit = params.error_limit;
        let telemetry = self.telemetry().clone();
        let telemetry_package = self.resolve_package_path(&params.package_path);
        let telemetry_filter = filter.clone();

        if vc_timeout == 0 || vc_timeout > MAX_VC_TIMEOUT {
            return Ok(CallToolResult::error(vec![Content::text(
                "timeout must be between 1 and 60 seconds; read the instructions about timeout management \
                 in the verification agent guide",
            )]));
        }
        if error_limit.is_some_and(|n| !(1..=MAX_VC_ERROR_LIMIT).contains(&n)) {
            return Ok(CallToolResult::error(vec![Content::text(format!(
                "error_limit must be between 1 and {}",
                MAX_VC_ERROR_LIMIT
            ))]));
        }

        let tool_timeout = self.tool_timeout();
        // The watchdog has to leave Z3 room to overshoot its own soft timeout,
        // which it does on nonlinear or heavily quantified queries. A killed
        // process carries no solver output, so a margin that is too small
        // silently loses the quantifier-instantiation analysis -- the evidence
        // a caller needs most when a proof times out. Scale the grace with the
        // solver budget instead of adding a constant to the maximum, and keep
        // the tool's own deadline as the outer bound.
        let hard_timeout_secs = tool_timeout
            .as_secs()
            .saturating_sub(1)
            .min(
                (vc_timeout as u64).saturating_mul(PROCESS_TIMEOUT_FACTOR)
                    + MCP_PROCESS_TIMEOUT_GRACE_SECS,
            )
            .max(1);
        let result = tokio::time::timeout(
            tool_timeout,
            tokio::task::spawn_blocking(move || {
                let (verification_filter, attribute_timeouts, evidence_depth) =
                    (verification_filter, attribute_timeouts, evidence_depth);
                let mut data = pkg.lock().unwrap();

                // 1. Check for compilation errors.
                if data.has_compilation_errors() {
                    return Ok(CallToolResult::error(vec![Content::text(
                        data.compilation_errors_report(),
                    )]));
                }

                // 2. Resolve filter into (VerifiedScope, VerificationScope).
                let (scope, verification_scope) = resolve_filter(data.env(), filter.as_deref())?;
                let verify_exclude = resolve_excludes(exclude.as_deref());
                let has_excludes = !verify_exclude.is_empty();

                // Reject `exclude` entries that don't match any target module
                // (or function, for `module::function` entries). Both
                // `build_filtered_env` and the anti-vacuity check below silently
                // no-op on unmatched names, which would otherwise hide typos.
                for entry in exclude.as_deref().unwrap_or(&[]) {
                    let module_name = module_part_of(entry);
                    let func_name = entry.rfind("::").map(|pos| &entry[pos + 2..]);
                    let exists = data.env().get_modules().any(|m| {
                        m.is_target()
                            && m.matches_name(module_name)
                            && func_name
                                .is_none_or(|f| m.get_functions().any(|fn_env| fn_env.matches_name(f)))
                    });
                    if !exists {
                        let kind = if func_name.is_some() { "function" } else { "module" };
                        return Err(rmcp::ErrorData::invalid_params(
                            format!("exclude entry `{}` does not match any target {}", entry, kind),
                            None,
                        ));
                    }
                }

                let exclude_entries: Vec<&str> = exclude
                    .as_deref()
                    .unwrap_or(&[])
                    .iter()
                    .map(String::as_str)
                    .collect();
                let func_excluded = |m: &move_model::model::ModuleEnv,
                                     func: &move_model::model::FunctionEnv|
                 -> bool {
                    exclude_entries.iter().any(|e| match e.rfind("::") {
                        Some(pos) => m.matches_name(&e[..pos]) && func.matches_name(&e[pos + 2..]),
                        None => m.matches_name(e),
                    })
                };
                let any_verifiable = data.env().get_modules().any(|m| {
                    m.is_target()
                        && m.get_functions().any(|func| {
                            func.should_verify(&verification_scope) && !func_excluded(&m, &func)
                        })
                });
                if !any_verifiable {
                    return Ok(CallToolResult::success(vec![Content::text(
                        "nothing to verify: filter and exclude leave no verifiable functions in scope",
                    )]));
                }

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
                            telemetry.emit(
                                "prover",
                                serde_json::json!({
                                    "package_id": telemetry_package,
                                    "filter": telemetry_filter,
                                    "vc_timeout_secs": vc_timeout,
                                    "cache_hit": true,
                                    "duration_us": 0,
                                    "outcome": if success { "success" } else { "failure" },
                                    "diagnostic_count": if success { 0 } else { 1 },
                                }),
                            );
                            return Ok(if success {
                                CallToolResult::success(vec![Content::text(msg)])
                            } else {
                                CallToolResult::error(vec![Content::text(msg)])
                            });
                        }
                    }
                }

                // 5. Build the env to run on (filtered or cached) up front so
                //    verify_exclude can be aligned with the actual target set.
                let excluded_modules: Vec<String> = exclude
                    .as_deref()
                    .map(|v| v.iter().filter(|e| !e.contains("::")).cloned().collect())
                    .unwrap_or_default();
                let needs_filtered_env = filter.is_some() || !excluded_modules.is_empty();
                let mut maybe_filtered_env = if needs_filtered_env {
                    match data.build_filtered_env(filter.as_deref(), &excluded_modules) {
                        Ok(env) => Some(env),
                        Err(e) => {
                            return Ok(CallToolResult::error(vec![Content::text(format!(
                                "failed to rebuild env for filter `{:?}` exclude `{:?}`: {}",
                                filter, excluded_modules, e
                            ))]))
                        },
                    }
                } else {
                    data.env().clear_diag();
                    None
                };

                // Keep verify_exclude entries whose module is still a target in
                // the env that will actually run. Modules demoted out of targets
                // are silently dropped (prover would otherwise reject them);
                // modules that survived (e.g. via soft fallback on file-share)
                // remain so the prover still applies them.
                let env_for_check = maybe_filtered_env.as_ref().unwrap_or_else(|| data.env());
                let prover_verify_exclude: Vec<_> = verify_exclude
                    .iter()
                    .filter(|s| {
                        use move_model::model::VerificationScope::*;
                        let module_name = match s {
                            OnlyModule(name) => name.as_str(),
                            Only(qname) => module_part_of(qname),
                            _ => return true,
                        };
                        env_for_check
                            .get_modules()
                            .any(|m| m.is_target() && m.matches_name(module_name))
                    })
                    .cloned()
                    .collect();

                // 6. Build prover options.
                let temp_dir = tempfile::tempdir().map_err(|e| {
                    rmcp::ErrorData::internal_error(
                        format!("failed to create temp dir: {}", e),
                        None,
                    )
                })?;
                let mut options = match load_sanitized_prover_options(data.path()) {
                    Ok(o) => o,
                    Err(msg) => {
                        return Ok(CallToolResult::error(vec![Content::text(msg)]));
                    },
                };
                options.prover.verify_scope = verification_scope;
                options.prover.verify_exclude = prover_verify_exclude;
                options.backend.vc_timeout = vc_timeout;
                options.backend.hard_timeout_secs = hard_timeout_secs;
                options.backend.package_timeout_secs = hard_timeout_secs;
                options.backend.package_error_limit = MAX_PACKAGE_ERROR_LIMIT;
                options.backend.split_vcs_by_assert = split_vcs_by_assert;
                if split_vcs_by_assert {
                    // Keep diagnostic assertion splits sequential. A large
                    // batch of simultaneous solver instances can contend for
                    // resources and turn otherwise-provable splits into hard
                    // process timeouts.
                    options.backend.proc_cores = 1;
                    // Explicit QI thresholds can make quantified assertion
                    // shards substantially slower than Z3's own defaults.
                    options.backend.use_solver_default_qi_thresholds = true;
                }
                if let Some(n) = error_limit {
                    options.backend.error_limit = n;
                }
                aptos_framework::prover::configure_aptos_custom_natives(&mut options);
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

                // 7. Run the prover.
                let mut error_writer = NoColor::new(Vec::new());
                let prover_start = Instant::now();
                // `inspect_diagnostics` reads without draining, so the error
                // count is taken from the records themselves rather than from
                // the rendered text, which `render_diagnostics` then consumes.
                let (prover_result, fallback_env_diags, reported_errors) =
                    if let Some(env) = maybe_filtered_env.as_mut() {
                        let result = move_prover::run_move_prover_with_model_v2(
                            env,
                            &mut error_writer,
                            options,
                            Instant::now(),
                        );
                        let errors = count_reported_errors(env);
                        let fallback = if result.is_err() {
                            super::super::package_data::render_diagnostics(env)
                        } else {
                            Vec::new()
                        };
                        (result, fallback, errors)
                    } else {
                        let result = move_prover::run_move_prover_with_model_v2(
                            data.env_mut(),
                            &mut error_writer,
                            options,
                            Instant::now(),
                        );
                        let errors = count_reported_errors(data.env());
                        let fallback = if result.is_err() {
                            super::super::package_data::render_diagnostics(data.env())
                        } else {
                            Vec::new()
                        };
                        (result, fallback, errors)
                    };
                let prover_duration_us = prover_start.elapsed().as_micros() as u64;

                match prover_result {
                    Ok(()) => {
                        // Only cache when no exclusions — an excluded-scope result
                        // doesn't represent the full scope and must not be reused.
                        if !has_excludes {
                            data.set_verified(scope, true, vc_timeout);
                        }
                        log::info!("move_package_verify: succeeded");
                        telemetry.emit(
                            "prover",
                            serde_json::json!({
                                "package_id": telemetry_package,
                                "filter": telemetry_filter,
                                "vc_timeout_secs": vc_timeout,
                                "cache_hit": false,
                                "duration_us": prover_duration_us,
                                "outcome": "success",
                                "diagnostic_count": 0,
                            }),
                        );
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
                        // Nothing reported on the model and no solver budget
                        // exhausted means no obligation was checked: the
                        // prover itself could not run.
                        let infrastructure =
                            reported_errors == 0 && !is_prover_timeout(&diag_text);
                        let diagnostic_count = if reported_errors > 0 {
                            reported_errors
                        } else {
                            fallback_env_diags.len().max(1)
                        };
                        let msg = if !diag_text.is_empty() {
                            data.set_diagnostics(DiagnosticSource::Verifier, vec![
                                diag_text.clone()
                            ]);
                            let attribution = if attribute_timeouts
                                && is_prover_timeout(&diag_text)
                            {
                                // Spend local computation to name the timed-out
                                // obligation instead of leaving the caller to
                                // narrow the filter over several turns.
                                attribute_timeout(
                                    data.env(),
                                    data.path(),
                                    &verification_filter,
                                    vc_timeout,
                                    tool_timeout.saturating_sub(prover_start.elapsed()),
                                )
                            } else {
                                String::new()
                            };
                            // Evidence explains why a loop is hard to verify,
                            // so a scope with no loop has none to give. The
                            // pass costs a full rebuild plus an inference run,
                            // and most failures here are not about loops.
                            let evidence = match evidence_depth {
                                Some(depth)
                                    if scope_contains_loop(data.env(), &verification_filter) =>
                                {
                                    loop_invariant_evidence(data.path(), &verification_filter, depth)
                                },
                                _ => String::new(),
                            };
                            format!(
                                "verification failed:\n{}{}{}",
                                diag_text, attribution, evidence
                            )
                        } else if !fallback_env_diags.is_empty() {
                            // The prover may return errors (e.g. tool version
                            // mismatch, boogie crash) that bypass the diagnostic
                            // writer. Use the env diags captured above from
                            // whichever env actually ran.
                            let joined = fallback_env_diags.join("\n");
                            data.set_diagnostics(DiagnosticSource::Verifier, fallback_env_diags);
                            format!("verification failed:\n{}", joined)
                        } else {
                            let err_msg = format!("{:#}", e);
                            data.set_diagnostics(DiagnosticSource::Verifier, vec![err_msg.clone()]);
                            format!("verification failed: {}", err_msg)
                        };
                        let msg = if infrastructure {
                            format!(
                                "PROVER UNAVAILABLE: the prover could not run, so nothing was \
                                 verified and this is not a verdict on the specification.\n{}",
                                msg
                            )
                        } else {
                            msg
                        };
                        log::info!("move_package_verify: failed\n{}", msg);
                        telemetry.emit(
                            "prover",
                            serde_json::json!({
                                "package_id": telemetry_package,
                                "filter": telemetry_filter,
                                "vc_timeout_secs": vc_timeout,
                                "cache_hit": false,
                                "duration_us": prover_duration_us,
                                "outcome": if infrastructure { "infrastructure" } else { "failure" },
                                "diagnostic_count": diagnostic_count,
                            }),
                        );
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

#[cfg(test)]
mod watchdog_tests {
    use super::{MAX_VC_TIMEOUT, MCP_PROCESS_TIMEOUT_GRACE_SECS, PROCESS_TIMEOUT_FACTOR};

    /// A killed Boogie process carries no solver output, so the watchdog has to
    /// leave Z3 room to overshoot its own soft timeout. Too small a margin does
    /// not just lose the quantifier-instantiation analysis: it discards every
    /// error Boogie had already found. Guard the margin against shrinking back
    /// to a constant.
    #[test]
    fn the_watchdog_scales_with_the_solver_budget() {
        let watchdog =
            |vc: u64| vc.saturating_mul(PROCESS_TIMEOUT_FACTOR) + MCP_PROCESS_TIMEOUT_GRACE_SECS;
        for vc in [1_u64, 10, 40, MAX_VC_TIMEOUT as u64] {
            assert!(
                watchdog(vc) >= vc.saturating_mul(2),
                "a {vc}s solver budget must leave at least the budget again as grace"
            );
        }
        assert!(
            watchdog(MAX_VC_TIMEOUT as u64)
                > MAX_VC_TIMEOUT as u64 + MCP_PROCESS_TIMEOUT_GRACE_SECS,
            "the margin must scale with the budget, not be a constant added to the maximum"
        );
    }
}
