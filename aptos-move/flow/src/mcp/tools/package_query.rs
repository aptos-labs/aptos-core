// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Move package query tools.

use super::super::{
    common::{mcp_err, resolve_function, tool_error, try_call},
    session::{into_call_tool_result, FlowSession},
};
use move_model::model::{FunId, GlobalEnv, QualifiedId};
use rmcp::{
    handler::server::wrapper::Parameters, model::CallToolResult, schemars, tool, tool_router,
};
use std::collections::{BTreeMap, BTreeSet};

// ========== MCP Tool types ==========

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
struct MovePackageQueryParams {
    /// Path to the Move package directory.
    package_path: String,
    #[serde(flatten)]
    query: MovePackageQuery,
}

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
#[serde(tag = "query")]
enum MovePackageQuery {
    /// Returns a map from each module to the modules it depends on.
    #[serde(rename = "dep_graph")]
    DepGraph,
    /// Returns a summary of each module's constants, structs, and functions.
    #[serde(rename = "module_summary")]
    ModuleSummary,
    /// Returns a function-level call graph as a map from each function to the functions it calls.
    #[serde(rename = "call_graph")]
    CallGraph,
    /// Returns direct and transitive calls/uses by a given function.
    /// "called" = direct calls; "used" = direct calls + closure captures.
    #[serde(rename = "function_usage")]
    FunctionUsage {
        /// Function to query, in the form "module_name::function_name".
        function: String,
    },
}

// ========== MCP Tool ==========

#[tool_router(router = package_query_router, vis = "pub(crate)")]
impl FlowSession {
    #[tool(
        description = "Query structural information about a Move package.",
        annotations(read_only_hint = false, destructive_hint = false)
    )]
    async fn move_package_query(
        &self,
        Parameters(params): Parameters<MovePackageQueryParams>,
    ) -> Result<CallToolResult, rmcp::ErrorData> {
        Ok(self
            .move_package_query_impl(params)
            .await
            .unwrap_or_else(tool_error))
    }
}

impl FlowSession {
    async fn move_package_query_impl(
        &self,
        params: MovePackageQueryParams,
    ) -> Result<CallToolResult, rmcp::ErrorData> {
        log::info!("move_package_query({})", params.package_path);
        let (pkg, _) = self.resolve_package(&params.package_path).await?;
        let data = pkg.lock().map_err(|_| mcp_err("package lock poisoned"))?;

        match params.query {
            MovePackageQuery::DepGraph => {
                let result = build_dep_graph(data.env());
                log::info!("move_package_query dep_graph: {} module(s)", result.len());
                Ok(into_call_tool_result(&result))
            },
            MovePackageQuery::ModuleSummary => {
                let result = build_module_summary(data.env());
                log::info!(
                    "move_package_query module_summary: {} module(s)",
                    result.len()
                );
                Ok(into_call_tool_result(&result))
            },
            MovePackageQuery::CallGraph => {
                let result = build_call_graph(data.env());
                log::info!(
                    "move_package_query call_graph: {} function(s)",
                    result.len()
                );
                Ok(into_call_tool_result(&result))
            },
            MovePackageQuery::FunctionUsage { function } => {
                let result = build_function_usage(data.env(), &function)?;
                log::info!("move_package_query function_usage({})", function);
                Ok(into_call_tool_result(&result))
            },
        }
    }
}

// ========== Query: dep_graph ==========

/// Returns an adjacency map: module name → set of modules it depends on.
fn build_dep_graph(env: &GlobalEnv) -> BTreeMap<String, BTreeSet<String>> {
    env.get_primary_target_modules()
        .iter()
        .map(|module| {
            let name = module.get_full_name_str();
            let deps: BTreeSet<String> = module
                .get_used_modules(false)
                .iter()
                .map(|dep_id| env.get_module(*dep_id).get_full_name_str())
                .collect();
            (name, deps)
        })
        .collect()
}

// ========== Query: module_summary ==========

// Response types for module_summary. Using typed structs rather than plain
// strings ensures the MCP client receives structured JSON it can access
// programmatically.

#[derive(Debug, serde::Serialize)]
struct ModuleSummary {
    constants: Vec<ConstantSummary>,
    structs: Vec<StructSummary>,
    functions: Vec<FunctionSummary>,
}

#[derive(Debug, serde::Serialize)]
struct ConstantSummary {
    name: String,
    #[serde(rename = "type")]
    type_: String,
    value: String,
}

#[derive(Debug, serde::Serialize)]
struct StructSummary {
    name: String,
    abilities: Vec<String>,
    fields: Vec<String>,
}

#[derive(Debug, serde::Serialize)]
struct FunctionSummary {
    name: String,
    signature: String,
}

/// Build a summary of each target module's constants, structs, and functions.
fn build_module_summary(env: &GlobalEnv) -> BTreeMap<String, ModuleSummary> {
    env.get_primary_target_modules()
        .iter()
        .map(|module| {
            let name = module.get_full_name_str();
            let type_ctx = module.get_type_display_ctx();

            let constants: Vec<ConstantSummary> = module
                .get_named_constants()
                .map(|c| {
                    let ctx = c.get_type_display_ctx();
                    ConstantSummary {
                        name: c.get_name().display(env.symbol_pool()).to_string(),
                        type_: c.get_type().display(&ctx).to_string(),
                        value: env.display(&c.get_value()).to_string(),
                    }
                })
                .collect();

            let structs: Vec<StructSummary> = module
                .get_structs()
                .map(|s| {
                    let abilities: Vec<String> = s
                        .get_abilities()
                        .into_iter()
                        .map(|a| a.to_string())
                        .collect();
                    let fields: Vec<String> = s
                        .get_fields()
                        .map(|f| {
                            format!(
                                "{}: {}",
                                f.get_name().display(env.symbol_pool()),
                                f.get_type().display(&type_ctx)
                            )
                        })
                        .collect();
                    StructSummary {
                        name: s.get_name().display(env.symbol_pool()).to_string(),
                        abilities,
                        fields,
                    }
                })
                .collect();

            let functions: Vec<FunctionSummary> = module
                .get_functions()
                .map(|f| FunctionSummary {
                    name: f.get_name_str(),
                    signature: f.get_header_string(),
                })
                .collect();

            (name, ModuleSummary {
                constants,
                structs,
                functions,
            })
        })
        .collect()
}

// ========== Query: call_graph ==========

/// Returns an adjacency map: function name → set of called function names.
fn build_call_graph(env: &GlobalEnv) -> BTreeMap<String, BTreeSet<String>> {
    env.get_primary_target_modules()
        .iter()
        .flat_map(|module| module.get_functions())
        .map(|func| {
            let name = func.get_full_name_with_address();
            let callees: BTreeSet<String> = func
                .get_called_functions()
                .map(|funs| {
                    funs.iter()
                        .map(|qid| env.get_function(*qid).get_full_name_with_address())
                        .collect()
                })
                .unwrap_or_default();
            (name, callees)
        })
        .collect()
}

// ========== Query: function_usage ==========

#[derive(Debug, serde::Serialize)]
struct FunctionUsage {
    called: BTreeSet<String>,
    called_transitive: BTreeSet<String>,
    used: BTreeSet<String>,
    used_transitive: BTreeSet<String>,
}

/// Build function usage for a given function.
fn build_function_usage(env: &GlobalEnv, function: &str) -> Result<FunctionUsage, rmcp::ErrorData> {
    let func = resolve_function(env, function)?;

    let called = func.get_called_functions().cloned().unwrap_or_default();
    let used = func.get_used_functions().cloned().unwrap_or_default();
    // These methods document that they panic if any function in the closure
    // lacks call info. Guard against that to avoid crashing the server.
    let called_transitive = try_call("failed to compute transitive called functions", || {
        func.get_transitive_closure_of_called_functions()
    })?;
    let used_transitive = try_call("failed to compute transitive used functions", || {
        func.get_transitive_closure_of_used_functions()
    })?;

    Ok(FunctionUsage {
        called: qids_to_names(env, &called),
        called_transitive: qids_to_names(env, &called_transitive),
        used: qids_to_names(env, &used),
        used_transitive: qids_to_names(env, &used_transitive),
    })
}

fn qids_to_names(env: &GlobalEnv, qids: &BTreeSet<QualifiedId<FunId>>) -> BTreeSet<String> {
    qids.iter()
        .map(|qid| env.get_function(*qid).get_full_name_with_address())
        .collect()
}
