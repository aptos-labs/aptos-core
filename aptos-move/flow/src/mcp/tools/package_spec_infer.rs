// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use super::{
    super::{package_data::DiagnosticSource, session::FlowSession},
    load_sanitized_prover_options, resolve_filter,
};
use crate::hooks::source_check;
use codespan_reporting::term::termcolor::NoColor;
use move_model::model::GlobalEnv;
use move_prover::inference::InferenceOutput;
use rmcp::{
    handler::server::wrapper::Parameters,
    model::{CallToolResult, Content},
    schemars, tool, tool_router,
};
use std::{
    fs,
    path::PathBuf,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
    time::Instant,
};

/// Controls where inferred specs are written.
#[derive(Debug, Default, Clone, Copy, serde::Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "snake_case")]
enum SpecOutput {
    /// Inject inferred specs inline into the original source files (default).
    #[default]
    Inline,
    /// Write inferred specs to separate `.spec.move` files alongside the sources,
    /// leaving the original source files untouched.
    File,
}

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
struct MovePackageSpecInferParams {
    /// Path to the Move package directory.
    package_path: String,
    /// Optional filter: `module_name` or `module_name::function_name`.
    /// When omitted, all target modules are inferred.
    filter: Option<String>,
    /// Where to write inferred specifications. Defaults to `inline` (inject into
    /// source files). Set to `file` to write separate `.spec.move` files instead,
    /// keeping original sources untouched.
    #[serde(default)]
    spec_output: SpecOutput,
}

#[tool_router(router = package_spec_infer_router, vis = "pub(crate)")]
impl FlowSession {
    // Low-level WP inference tool. Requires multi-phase workflow context
    // (loop-invariant synthesis, simplification, verification) that is only
    // available through subagent delegation. See skill docs for spec_output param.
    #[tool(
        description = "Raw WP engine — output requires loop-invariant synthesis and simplification \
                       that only the /move-inf skill workflow provides. Do not call directly.",
        annotations(read_only_hint = false, destructive_hint = true)
    )]
    async fn move_package_wp(
        &self,
        Parameters(params): Parameters<MovePackageSpecInferParams>,
    ) -> Result<CallToolResult, rmcp::ErrorData> {
        log::info!(
            "move_package_wp({}, filter={:?})",
            params.package_path,
            params.filter
        );
        let (pkg, _) = self.resolve_package(&params.package_path).await?;
        let filter = params.filter.clone();
        let spec_output = params.spec_output;

        let tool_timeout = self.tool_timeout();
        let wrote_files = Arc::new(AtomicBool::new(false));
        let wrote_files_in = Arc::clone(&wrote_files);
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
                let (_scope, verification_scope) = resolve_filter(data.env(), filter.as_deref())?;

                // 4. Build prover options for inference.
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
                aptos_framework::prover::configure_aptos_custom_natives(&mut options);
                options.inference.inference = true;
                options.inference.inference_output = match spec_output {
                    SpecOutput::Inline => InferenceOutput::Unified,
                    SpecOutput::File => InferenceOutput::File,
                };
                options.inference.inference_output_dir = None;
                options.output_path = temp_dir
                    .path()
                    .join("output.bpl")
                    .to_string_lossy()
                    .into_owned();

                // 5. Run inference.
                //
                // When `filter` is set, build a *fresh* env in which only files
                // matching the filter are primary targets — same rationale as in
                // `move_package_verify`. Without this, the cached env (all modules
                // `is_target == true`) forces the prover's bytecode pipeline over
                // the whole package and can panic on unverified modules whose
                // intent is to be skipped (e.g. `storage_slot`).
                let mut error_writer = NoColor::new(Vec::new());
                let mut filtered_env_holder: Option<GlobalEnv> = None;
                let inference_result = if let Some(filter_str) = filter.as_deref() {
                    let mut fresh = match data.build_filtered_env(Some(filter_str), &[]) {
                        Ok(env) => env,
                        Err(e) => {
                            return Ok(CallToolResult::error(vec![Content::text(format!(
                                "failed to rebuild env for filter `{}`: {}",
                                filter_str, e
                            ))]))
                        },
                    };
                    let result = move_prover::inference::run_spec_inference_with_model(
                        &mut fresh,
                        &mut error_writer,
                        options,
                        Instant::now(),
                    );
                    filtered_env_holder = Some(fresh);
                    result
                } else {
                    data.env().clear_diag();
                    move_prover::inference::run_spec_inference_with_model(
                        data.env_mut(),
                        &mut error_writer,
                        options,
                        Instant::now(),
                    )
                };

                match inference_result {
                    Ok(()) => {
                        // 6. Collect output files depending on the output mode.
                        let mut modified_files: Vec<String> = Vec::new();

                        // Use the env that actually ran inference (filtered when
                        // a filter was set, cached otherwise) so module iteration
                        // matches what was processed.
                        let modules_env: &GlobalEnv =
                            filtered_env_holder.as_ref().unwrap_or_else(|| data.env());
                        for module in modules_env.get_modules() {
                            if !module.is_target() {
                                continue;
                            }
                            let source_path = PathBuf::from(module.get_source_path());
                            let stem = source_path
                                .file_stem()
                                .expect("source file should have a stem");
                            let source_dir = source_path
                                .parent()
                                .expect("source file should have a parent directory");

                            match spec_output {
                                SpecOutput::Inline => {
                                    // Read each .enriched.move file, overwrite the
                                    // original, and delete the enriched file.
                                    let enriched_path = source_dir
                                        .join(format!("{}.enriched.move", stem.to_string_lossy()));

                                    if enriched_path.exists() {
                                        let content =
                                            fs::read_to_string(&enriched_path).map_err(|e| {
                                                rmcp::ErrorData::internal_error(
                                                    format!(
                                                        "failed to read enriched file {}: {}",
                                                        enriched_path.display(),
                                                        e
                                                    ),
                                                    None,
                                                )
                                            })?;
                                        fs::write(&source_path, &content).map_err(|e| {
                                            rmcp::ErrorData::internal_error(
                                                format!(
                                                    "failed to write source file {}: {}",
                                                    source_path.display(),
                                                    e
                                                ),
                                                None,
                                            )
                                        })?;
                                        fs::remove_file(&enriched_path).map_err(|e| {
                                            rmcp::ErrorData::internal_error(
                                                format!(
                                                    "failed to remove enriched file {}: {}",
                                                    enriched_path.display(),
                                                    e
                                                ),
                                                None,
                                            )
                                        })?;
                                        modified_files
                                            .push(source_path.to_string_lossy().into_owned());
                                    }
                                },
                                SpecOutput::File => {
                                    // The prover wrote a .spec.move file; collect
                                    // its path without touching the original source.
                                    let spec_path = source_dir
                                        .join(format!("{}.spec.move", stem.to_string_lossy()));

                                    if spec_path.exists() {
                                        modified_files
                                            .push(spec_path.to_string_lossy().into_owned());
                                    }
                                },
                            }
                        }

                        if modified_files.is_empty() {
                            log::info!("move_package_wp: no specs inferred");
                            Ok(CallToolResult::success(vec![Content::text(
                                "inference completed but no specifications were inferred",
                            )]))
                        } else {
                            wrote_files_in.store(true, Ordering::Relaxed);
                            log::info!(
                                "move_package_wp: wrote specs to {} file(s)",
                                modified_files.len()
                            );

                            // Run the same checks the edit hook would apply
                            // (format + AST validation) on each modified file.
                            let mut check_diags = String::new();
                            for path in &modified_files {
                                let source = fs::read_to_string(path).unwrap_or_default();
                                let result = source_check::check(path, &source);
                                if !result.has_parse_errors {
                                    source_check::format_file(path);
                                }
                                if !result.output.is_empty() {
                                    check_diags.push_str(&result.output);
                                }
                            }

                            let action = match spec_output {
                                SpecOutput::Inline => "injected specs into",
                                SpecOutput::File => "wrote spec files for",
                            };
                            let mut msg = format!(
                                "inference succeeded, {} {} file(s) \
                             (read the files to see the changes):\n",
                                action,
                                modified_files.len()
                            );
                            for path in &modified_files {
                                msg.push_str(&format!("- {}\n", path));
                            }
                            if !check_diags.is_empty() {
                                msg.push_str(&format!(
                                    "\ndiagnostics in inferred output:\n{}",
                                    check_diags
                                ));
                            }
                            Ok(CallToolResult::success(vec![Content::text(msg)]))
                        }
                    },
                    Err(_) => {
                        // Inference may have written .enriched.move (Inline) or
                        // .spec.move (File) artifacts before erroring on a later
                        // module. Detect any such artifacts so the session-level
                        // cache is invalidated even on partial-failure paths.
                        let modules_env: &GlobalEnv =
                            filtered_env_holder.as_ref().unwrap_or_else(|| data.env());
                        for module in modules_env.get_modules() {
                            if !module.is_target() {
                                continue;
                            }
                            let source_path = PathBuf::from(module.get_source_path());
                            let stem = source_path
                                .file_stem()
                                .expect("source file should have a stem");
                            let source_dir = source_path
                                .parent()
                                .expect("source file should have a parent directory");
                            let artifact = match spec_output {
                                SpecOutput::Inline => source_dir
                                    .join(format!("{}.enriched.move", stem.to_string_lossy())),
                                SpecOutput::File => {
                                    source_dir.join(format!("{}.spec.move", stem.to_string_lossy()))
                                },
                            };
                            if artifact.exists() {
                                wrote_files_in.store(true, Ordering::Relaxed);
                                break;
                            }
                        }
                        let writer_text =
                            String::from_utf8(error_writer.into_inner()).unwrap_or_default();
                        let env_diags = super::super::package_data::render_diagnostics(modules_env);
                        let combined = if writer_text.is_empty() {
                            env_diags.join("\n")
                        } else if env_diags.is_empty() {
                            writer_text.clone()
                        } else {
                            format!("{}\n{}", writer_text, env_diags.join("\n"))
                        };
                        let record = if combined.is_empty() {
                            "spec inference failed".to_string()
                        } else {
                            combined.clone()
                        };
                        data.set_diagnostics(DiagnosticSource::Inference, vec![record]);
                        let msg = if combined.is_empty() {
                            "spec inference failed".to_string()
                        } else {
                            format!("spec inference failed:\n{}", combined)
                        };
                        log::info!("move_package_wp: failed");
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
            rmcp::ErrorData::internal_error(format!("spec infer task panicked: {}", e), None)
        })??;

        if wrote_files.load(Ordering::Relaxed) {
            self.invalidate_package(&params.package_path);
        }

        Ok(result)
    }
}
