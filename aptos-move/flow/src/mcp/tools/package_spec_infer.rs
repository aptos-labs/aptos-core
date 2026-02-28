// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use super::{super::session::FlowSession, resolve_filter};
use crate::hooks::source_check;
use codespan_reporting::term::termcolor::NoColor;
use move_prover::inference::InferenceOutput;
use rmcp::{
    handler::server::wrapper::Parameters,
    model::{CallToolResult, Content},
    schemars, tool, tool_router,
};
use std::{fs, path::PathBuf, time::Instant};

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
struct MovePackageSpecInferParams {
    /// Path to the Move package directory.
    package_path: String,
    /// Optional filter: `module_name` or `module_name::function_name`.
    /// When omitted, all target modules are inferred.
    filter: Option<String>,
}

#[tool_router(router = package_spec_infer_router, vis = "pub(crate)")]
impl FlowSession {
    #[tool(description = "Low-level WP inference tool — not for direct use. \
                       Requires multi-phase workflow context (loop-invariant synthesis, \
                       simplification, verification) that is only available through \
                       subagent delegation.")]
    async fn move_package_spec_infer(
        &self,
        Parameters(params): Parameters<MovePackageSpecInferParams>,
    ) -> Result<CallToolResult, rmcp::ErrorData> {
        log::info!(
            "move_package_spec_infer({}, filter={:?})",
            params.package_path,
            params.filter
        );
        let pkg = self.resolve_package(&params.package_path).await?;
        let filter = params.filter.clone();

        let result = tokio::task::spawn_blocking(move || {
            let mut data = pkg.lock().unwrap();

            // 1. Check for compilation errors.
            if data.env().has_errors() {
                return Ok(CallToolResult::error(vec![Content::text(
                    "package has compilation errors; run move_package_status for details",
                )]));
            }

            // 2. Ensure bytecode is available (inference requires it).
            if !data.has_bytecode() {
                data.rebuild_with_bytecode().map_err(|e| {
                    rmcp::ErrorData::internal_error(
                        format!("failed to rebuild with bytecode: {}", e),
                        None,
                    )
                })?;
                if data.env().has_errors() {
                    return Ok(CallToolResult::error(vec![Content::text(
                        "package has compilation errors after bytecode build; \
                         run move_package_status for details",
                    )]));
                }
            }

            // 3. Resolve filter into (VerifiedScope, VerificationScope).
            let (_scope, verification_scope) = resolve_filter(data.env(), filter.as_deref())?;

            // 4. Build prover options for inference.
            let temp_dir = tempfile::tempdir().map_err(|e| {
                rmcp::ErrorData::internal_error(format!("failed to create temp dir: {}", e), None)
            })?;
            let mut options = move_prover::cli::Options::default();
            options.prover.verify_scope = verification_scope;
            options.inference.inference = true;
            options.inference.inference_output = InferenceOutput::Unified;
            options.inference.inference_output_dir = None;
            options.output_path = temp_dir
                .path()
                .join("output.bpl")
                .to_string_lossy()
                .into_owned();

            // 5. Run spec inference.
            let mut error_writer = NoColor::new(Vec::new());
            let inference_result = move_prover::inference::run_spec_inference_with_model(
                data.env_mut(),
                &mut error_writer,
                options,
                Instant::now(),
            );

            match inference_result {
                Ok(()) => {
                    // 6. Read each .enriched.move file, overwrite the original, and delete it.
                    let mut modified_files: Vec<String> = Vec::new();

                    for module in data.env().get_modules() {
                        if !module.is_target() {
                            continue;
                        }
                        let source_path = PathBuf::from(module.get_source_path());
                        let stem = source_path
                            .file_stem()
                            .expect("source file should have a stem");
                        let enriched_path = source_path
                            .parent()
                            .expect("source file should have a parent directory")
                            .join(format!("{}.enriched.move", stem.to_string_lossy()));

                        if enriched_path.exists() {
                            let content = fs::read_to_string(&enriched_path).map_err(|e| {
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
                            modified_files.push(source_path.to_string_lossy().into_owned());
                        }
                    }

                    if modified_files.is_empty() {
                        log::info!("move_package_spec_infer: no specs inferred");
                        Ok(CallToolResult::success(vec![Content::text(
                            "inference completed but no specifications were inferred",
                        )]))
                    } else {
                        log::info!(
                            "move_package_spec_infer: injected specs into {} file(s)",
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

                        let mut msg = format!(
                            "inference succeeded, injected specs into {} file(s) \
                             (read the files to see the changes):\n",
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
                    let diag_text =
                        String::from_utf8(error_writer.into_inner()).unwrap_or_default();
                    if diag_text.is_empty() {
                        data.set_diagnostics(
                            vec!["spec inference failed".to_string()],
                            "inferring",
                        );
                    } else {
                        data.set_diagnostics(vec![diag_text.clone()], "inferring");
                    }
                    let msg = if diag_text.is_empty() {
                        "spec inference failed".to_string()
                    } else {
                        format!("spec inference failed:\n{}", diag_text)
                    };
                    log::info!("move_package_spec_infer: failed");
                    Ok(CallToolResult::error(vec![Content::text(msg)]))
                },
            }
        })
        .await
        .map_err(|e| {
            rmcp::ErrorData::internal_error(format!("spec infer task panicked: {}", e), None)
        })??;

        Ok(result)
    }
}
