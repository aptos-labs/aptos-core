// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use super::super::session::{into_call_tool_result, FlowSession};
use rmcp::{
    handler::server::wrapper::Parameters, model::CallToolResult, schemars, tool, tool_router,
};

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
struct MovePackageManifestParams {
    /// Path to the Move package directory.
    package_path: String,
}

#[derive(Debug, serde::Serialize, schemars::JsonSchema)]
struct MovePackageManifestResult {
    source_paths: Vec<String>,
    dep_paths: Vec<String>,
}

#[tool_router(router = package_manifest_router, vis = "pub(crate)")]
impl FlowSession {
    #[tool(description = "Get information about the current Move package")]
    async fn move_package_manifest(
        &self,
        Parameters(params): Parameters<MovePackageManifestParams>,
    ) -> Result<CallToolResult, rmcp::ErrorData> {
        log::info!("move_package_manifest({})", params.package_path);
        let pkg = self.resolve_package(&params.package_path).await?;
        let data = pkg.lock().unwrap();
        let env = data.env();
        let source_paths: Vec<String> = env
            .get_primary_target_modules()
            .iter()
            .map(|m| m.get_source_path().to_string_lossy().into_owned())
            .collect();
        let dep_paths: Vec<String> = env
            .get_modules()
            .filter(|m| !m.is_target())
            .map(|m| m.get_source_path().to_string_lossy().into_owned())
            .collect();
        drop(data);
        let result = MovePackageManifestResult {
            source_paths,
            dep_paths,
        };
        log::info!(
            "move_package_manifest: {} source(s), {} dep(s)",
            result.source_paths.len(),
            result.dep_paths.len()
        );
        Ok(into_call_tool_result(&result))
    }
}
