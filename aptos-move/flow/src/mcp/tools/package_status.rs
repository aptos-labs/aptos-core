// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use super::super::session::FlowSession;
use codespan_reporting::{
    diagnostic::Severity,
    term::{emit, termcolor::NoColor, Config},
};
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
        let env = data.env();
        let has_errors = env.has_errors();
        let mut messages = Vec::new();
        env.report_diag_with_filter(
            |files, diag| {
                let mut buf = NoColor::new(Vec::new());
                emit(&mut buf, &Config::default(), files, diag).expect("emit must not fail");
                let text = String::from_utf8(buf.into_inner()).unwrap_or_default();
                messages.push(text);
            },
            |d| d.severity >= Severity::Warning,
        );
        drop(data);
        let content = if messages.is_empty() {
            "no errors or warnings".to_string()
        } else {
            messages.join("\n")
        };
        let result = if has_errors {
            CallToolResult::error(vec![Content::text(content)])
        } else {
            CallToolResult::success(vec![Content::text(content)])
        };
        log::info!(
            "move_package_status: has_errors={}, {} message(s)",
            has_errors,
            messages.len()
        );
        Ok(result)
    }
}
