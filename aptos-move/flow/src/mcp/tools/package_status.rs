// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use super::super::session::FlowSession;
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
    #[tool(description = "Check a Move package for compilation errors and warnings")]
    async fn move_package_status(
        &self,
        Parameters(params): Parameters<MovePackageStatusParams>,
    ) -> Result<CallToolResult, rmcp::ErrorData> {
        log::info!("move_package_status({})", params.package_path);
        let pkg = self.resolve_package(&params.package_path).await?;
        let data = pkg.lock().unwrap();
        let has_errors = data.env().has_errors();
        let (messages, source) = data.diagnostics();
        let content = if messages.is_empty() {
            "no errors or warnings".to_string()
        } else {
            format!("{}\n(from {})", messages.join("\n"), source)
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
