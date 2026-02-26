// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use super::super::{package_data::VerifiedScope, session::FlowSession};
use codespan_reporting::term::termcolor::NoColor;
use move_model::model::VerificationScope;
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
    /// Solver timeout per verification condition, in seconds. Default: 40.
    timeout: Option<usize>,
}

const DEFAULT_VC_TIMEOUT: usize = 40;

#[tool_router(router = package_verify_router, vis = "pub(crate)")]
impl FlowSession {
    #[tool(
        description = "Run the Move Prover on a package to formally verify specifications. \
                        An optional filter can restrict verification to a single module \
                        (`module_name`) or function (`module_name::function_name`)."
    )]
    async fn move_package_verify(
        &self,
        Parameters(params): Parameters<MovePackageVerifyParams>,
    ) -> Result<CallToolResult, rmcp::ErrorData> {
        log::info!(
            "move_package_verify({}, filter={:?})",
            params.package_path,
            params.filter
        );
        let pkg = self.resolve_package(&params.package_path).await?;
        let filter = params.filter.clone();
        let vc_timeout = params.timeout.unwrap_or(DEFAULT_VC_TIMEOUT);

        let result = tokio::task::spawn_blocking(move || {
            let mut data = pkg.lock().unwrap();

            // 1. Check for compilation errors.
            if data.env().has_errors() {
                return Ok(CallToolResult::error(vec![Content::text(
                    "package has compilation errors; run move_package_status for details",
                )]));
            }

            // 2. Ensure bytecode is available (prover requires it).
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
            let (scope, verification_scope) = resolve_filter(data.env(), filter.as_deref())?;

            // 4. Check cache.
            //    - Success propagates via entailment (wider success ⇒ narrower success).
            //    - Errors only reuse an exact scope match; a wider-scope failure does
            //      not imply a narrower scope also fails (the error may be elsewhere).
            if let Some((ref cached_scope, success)) = data.verified() {
                let hit = if success {
                    cached_scope.entails_success(&scope)
                } else {
                    cached_scope.entails_error(&scope)
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

            // 5. Build prover options.
            let temp_dir = tempfile::tempdir().map_err(|e| {
                rmcp::ErrorData::internal_error(format!("failed to create temp dir: {}", e), None)
            })?;
            let mut options = move_prover::cli::Options::default();
            options.prover.verify_scope = verification_scope;
            options.backend.vc_timeout = vc_timeout;
            options.output_path = temp_dir
                .path()
                .join("output.bpl")
                .to_string_lossy()
                .into_owned();

            // 6. Run the prover.
            let mut error_writer = NoColor::new(Vec::new());
            let prover_result = move_prover::run_move_prover_with_model_v2(
                data.env_mut(),
                &mut error_writer,
                options,
                Instant::now(),
            );

            match prover_result {
                Ok(()) => {
                    data.set_verified(scope, true);
                    log::info!("move_package_verify: succeeded");
                    Ok(CallToolResult::success(vec![Content::text(
                        "verification succeeded",
                    )]))
                },
                Err(_) => {
                    data.set_verified(scope, false);
                    let diag_text =
                        String::from_utf8(error_writer.into_inner()).unwrap_or_default();
                    let msg = if diag_text.is_empty() {
                        "verification failed".to_string()
                    } else {
                        format!("verification failed:\n{}", diag_text)
                    };
                    log::info!("move_package_verify: failed");
                    Ok(CallToolResult::error(vec![Content::text(msg)]))
                },
            }
        })
        .await
        .map_err(|e| {
            rmcp::ErrorData::internal_error(format!("verify task panicked: {}", e), None)
        })??;

        Ok(result)
    }
}

/// Resolve an optional filter string into `(VerifiedScope, VerificationScope)`.
fn resolve_filter(
    env: &move_model::model::GlobalEnv,
    filter: Option<&str>,
) -> Result<(VerifiedScope, VerificationScope), rmcp::ErrorData> {
    let filter = match filter {
        None => return Ok((VerifiedScope::Package, VerificationScope::All)),
        Some(f) => f,
    };

    if let Some(pos) = filter.rfind("::") {
        // Function filter: "module::function"
        let module_part = &filter[..pos];
        let func_part = &filter[pos + 2..];

        let module_sym = env.symbol_pool().make(module_part);
        let module = env
            .find_module_by_name(module_sym)
            .filter(|m| m.is_target())
            .ok_or_else(|| {
                rmcp::ErrorData::invalid_params(
                    format!(
                        "no module matching `{}` found in target modules",
                        module_part
                    ),
                    None,
                )
            })?;
        let func_sym = env.symbol_pool().make(func_part);
        let func = module.find_function(func_sym).ok_or_else(|| {
            rmcp::ErrorData::invalid_params(
                format!(
                    "no function `{}` found in module `{}`",
                    func_part, module_part
                ),
                None,
            )
        })?;
        Ok((
            VerifiedScope::Function(func.get_qualified_id()),
            VerificationScope::Only(filter.to_string()),
        ))
    } else {
        // Module filter: "module_name"
        let module_sym = env.symbol_pool().make(filter);
        let module = env
            .find_module_by_name(module_sym)
            .filter(|m| m.is_target())
            .ok_or_else(|| {
                rmcp::ErrorData::invalid_params(
                    format!("no module matching `{}` found in target modules", filter),
                    None,
                )
            })?;
        Ok((
            VerifiedScope::Module(module.get_id()),
            VerificationScope::OnlyModule(filter.to_string()),
        ))
    }
}
