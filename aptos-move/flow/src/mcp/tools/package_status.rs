// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use super::super::{package_data::DiagnosticSource, session::FlowSession};
use rmcp::{
    handler::server::wrapper::Parameters,
    model::{CallToolResult, Content},
    schemars, tool, tool_router,
};

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
struct MovePackageStatusParams {
    /// Path to the Move package directory.
    package_path: String,
}

#[tool_router(router = package_status_router, vis = "pub(crate)")]
impl FlowSession {
    #[tool(
        description = "Check a Move package for compilation errors and warnings",
        annotations(read_only_hint = false, destructive_hint = false)
    )]
    async fn move_package_status(
        &self,
        Parameters(params): Parameters<MovePackageStatusParams>,
    ) -> Result<CallToolResult, rmcp::ErrorData> {
        log::info!("move_package_status({})", params.package_path);
        let (pkg, _) = self.resolve_package(&params.package_path).await?;
        let data = pkg
            .lock()
            .map_err(|_| rmcp::ErrorData::internal_error("package lock poisoned", None))?;
        let has_errors = data.has_compilation_errors();
        let messages = data.diagnostics(DiagnosticSource::Compiler);
        let content = if messages.is_empty() {
            if has_errors {
                "package has errors (run move_package_status again after editing)".to_string()
            } else {
                "no errors or warnings".to_string()
            }
        } else {
            messages.join("\n")
        };
        let num_messages = messages.len();
        drop(data);
        let result = if has_errors {
            CallToolResult::error(vec![Content::text(content)])
        } else {
            CallToolResult::success(vec![Content::text(content)])
        };
        log::info!(
            "move_package_status: has_errors={}, {} message(s)",
            has_errors,
            num_messages
        );
        Ok(result)
    }
}
