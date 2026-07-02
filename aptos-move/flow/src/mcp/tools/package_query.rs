// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Move package query tools.

use super::super::{
    common::{mcp_err, resolve_function, tool_error, try_call},
    session::{into_call_tool_result, FlowSession},
};
use move_compiler_v2::env_pipeline::lambda_lifter::is_lambda_lifted_fun as is_lambda_lifted;
use move_model::{
    ast::{Attribute, AttributeValue, ExpData, Operation},
    model::{
        FunId, FunctionEnv, GlobalEnv, Loc, ModuleEnv, NamedConstantEnv, QualifiedId, StructEnv,
        TypeParameter, Visibility,
    },
    ty::{ReferenceKind, TypeDisplayContext},
};
use rmcp::{
    handler::server::wrapper::Parameters, model::CallToolResult, schemars, tool, tool_router,
};
use std::collections::{BTreeMap, BTreeSet};

// ========== MCP Tool types ==========

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
struct MovePackageQueryParams {
    /// Path to the Move package directory.
    package_path: String,
    /// Query type.
    query: QueryType,
    /// Function to query (required for "function_usage"), in the form "module_name::function_name".
    function: Option<String>,
}

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "snake_case")]
enum QueryType {
    /// Returns a map from each module to the modules it depends on.
    DepGraph,
    /// Returns a summary of each module's constants, structs, and functions.
    ModuleSummary,
    /// Returns a function-level call graph as a map from each function to the functions it calls.
    CallGraph,
    /// Returns direct and transitive calls/uses by a given function.
    /// "called" = direct calls; "used" = direct calls + closure captures.
    FunctionUsage,
    /// Returns per-module facts: functions, structs, constants, friends,
    /// attributes, and source locations.
    Facts,
}

// ========== MCP Tool ==========

#[tool_router(router = package_query_router, vis = "pub(crate)")]
impl FlowSession {
    #[tool(
        description = "Query structural information about a Move package.",
        annotations(read_only_hint = true, destructive_hint = false)
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
            QueryType::DepGraph => {
                let result = build_dep_graph(data.env());
                log::info!("move_package_query dep_graph: {} module(s)", result.len());
                Ok(into_call_tool_result(&result))
            },
            QueryType::ModuleSummary => {
                let result = build_module_summary(data.env());
                log::info!(
                    "move_package_query module_summary: {} module(s)",
                    result.len()
                );
                Ok(into_call_tool_result(&result))
            },
            QueryType::CallGraph => {
                let result = build_call_graph(data.env());
                log::info!(
                    "move_package_query call_graph: {} function(s)",
                    result.len()
                );
                Ok(into_call_tool_result(&result))
            },
            QueryType::FunctionUsage => {
                let function = params.function.ok_or_else(|| {
                    mcp_err("\"function\" parameter is required for function_usage query")
                })?;
                let result = build_function_usage(data.env(), &function)?;
                log::info!("move_package_query function_usage({})", function);
                Ok(into_call_tool_result(&result))
            },
            QueryType::Facts => {
                let result = try_call("failed to build facts", || build_facts(data.env()))?;
                log::info!("move_package_query facts: {} module(s)", result.len());
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
#[serde(rename_all = "camelCase")]
struct FunctionSummary {
    name: String,
    signature: String,
    /// True for compiler-synthesized lambda-lifted functions.
    is_lambda_lifted: bool,
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
                .map(|c| build_constant_summary(env, &c))
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
                    is_lambda_lifted: is_lambda_lifted(&f),
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
    // These methods panic if any function in the BFS lacks call info.
    // Guard with try_call to return a clean error instead of crashing.
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

// ========== Query: facts ==========
//
// Per-module facts read from the compiler's GlobalEnv. Wire keys use camelCase.

/// Source file and 1-indexed `[start_line, end_line]` span for an item.
#[derive(Debug, serde::Serialize)]
#[serde(rename_all = "camelCase")]
struct SourceLocation {
    #[serde(skip_serializing_if = "Option::is_none")]
    file: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    span: Option<[u32; 2]>,
}

#[derive(Debug, serde::Serialize)]
#[serde(rename_all = "camelCase")]
struct ModuleFacts {
    #[serde(flatten)]
    location: SourceLocation,
    friends: Vec<FriendFacts>,
    attributes: Vec<AttributeFacts>,
    functions: Vec<FunctionFacts>,
    structs: Vec<StructFacts>,
    constants: Vec<ConstantSummary>,
}

#[derive(Debug, serde::Serialize)]
#[serde(rename_all = "camelCase")]
struct FriendFacts {
    module: String,
}

#[derive(Debug, serde::Serialize)]
#[serde(rename_all = "camelCase")]
struct AttributeFacts {
    name: String,
    /// Present for an assignment attribute `#[name = value]`.
    #[serde(skip_serializing_if = "Option::is_none")]
    value: Option<String>,
    /// Present for a nested attribute `#[name(inner, ...)]`.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    args: Vec<AttributeFacts>,
}

#[derive(Debug, serde::Serialize)]
#[serde(rename_all = "camelCase")]
struct TypeParamFacts {
    name: String,
    abilities: Vec<String>,
    is_phantom: bool,
}

#[derive(Debug, serde::Serialize)]
#[serde(rename_all = "camelCase")]
struct ParamFacts {
    name: String,
    #[serde(rename = "type")]
    type_: String,
}

#[derive(Debug, serde::Serialize)]
#[serde(rename_all = "camelCase")]
struct FieldFacts {
    name: String,
    #[serde(rename = "type")]
    type_: String,
    positional: bool,
}

#[derive(Debug, serde::Serialize)]
#[serde(rename_all = "camelCase")]
struct VariantFacts {
    name: String,
    /// One of `"unit"`, `"positional"`, `"named"`.
    kind: String,
    fields: Vec<FieldFacts>,
    attributes: Vec<AttributeFacts>,
}

#[derive(Debug, serde::Serialize)]
#[serde(rename_all = "camelCase")]
struct ResourceAccessFacts {
    reads: Vec<String>,
    writes: Vec<String>,
}

#[derive(Debug, serde::Serialize)]
#[serde(rename_all = "camelCase")]
struct FunctionFacts {
    name: String,
    #[serde(flatten)]
    location: SourceLocation,
    /// One of `"public"`, `"friend"`, `"package"`, `"internal"`.
    visibility: String,
    is_entry: bool,
    is_inline: bool,
    is_native: bool,
    is_view: bool,
    attributes: Vec<AttributeFacts>,
    type_params: Vec<TypeParamFacts>,
    params: Vec<ParamFacts>,
    return_types: Vec<String>,
    acquires_inferred: Vec<String>,
    resource_access: ResourceAccessFacts,
    /// True for lambda-lifted functions.
    is_lambda_lifted: bool,
    /// For a lambda-lifted function, the user function whose source defines the lambda.
    #[serde(skip_serializing_if = "Option::is_none")]
    defined_in: Option<String>,
}

#[derive(Debug, serde::Serialize)]
#[serde(rename_all = "camelCase")]
struct StructFacts {
    /// `"struct"` or `"enum"`.
    kind: String,
    name: String,
    #[serde(flatten)]
    location: SourceLocation,
    abilities: Vec<String>,
    type_params: Vec<TypeParamFacts>,
    #[serde(skip_serializing_if = "Option::is_none")]
    fields: Option<Vec<FieldFacts>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    variants: Option<Vec<VariantFacts>>,
    attributes: Vec<AttributeFacts>,
}

/// Build per-module facts for every primary target module in `env`.
fn build_facts(env: &GlobalEnv) -> BTreeMap<String, ModuleFacts> {
    env.get_primary_target_modules()
        .iter()
        .map(|module| {
            let name = module.get_full_name_str();
            let facts = build_module_facts(env, module);
            (name, facts)
        })
        .collect()
}

fn build_module_facts(env: &GlobalEnv, module: &ModuleEnv<'_>) -> ModuleFacts {
    let location = loc_to_file_span(env, &module.get_loc());

    let friends: Vec<FriendFacts> = module
        .get_friend_modules()
        .iter()
        .map(|id| FriendFacts {
            module: env.get_module(*id).get_full_name_str(),
        })
        .collect();

    let attributes = attrs_to_facts(env, module.get_attributes());

    let defined_in = defining_functions(env, module);
    let functions: Vec<FunctionFacts> = module
        .get_functions()
        .map(|f| build_function_facts(env, &f, &defined_in))
        .collect();

    let structs: Vec<StructFacts> = module
        .get_structs()
        .map(|s| build_struct_facts(env, &s))
        .collect();

    let constants: Vec<ConstantSummary> = module
        .get_named_constants()
        .map(|c| build_constant_summary(env, &c))
        .collect();

    ModuleFacts {
        location,
        friends,
        attributes,
        functions,
        structs,
        constants,
    }
}

fn build_constant_summary(env: &GlobalEnv, c: &NamedConstantEnv<'_>) -> ConstantSummary {
    let ctx = c.get_type_display_ctx();
    ConstantSummary {
        name: c.get_name().display(env.symbol_pool()).to_string(),
        type_: c.get_type().display(&ctx).to_string(),
        value: env.display(&c.get_value()).to_string(),
    }
}

/// Maps each lambda-lifted function to the user function whose source
/// contains it, chaining through nested closures.
fn defining_functions(
    env: &GlobalEnv,
    module: &ModuleEnv<'_>,
) -> BTreeMap<QualifiedId<FunId>, String> {
    let mut result = BTreeMap::new();
    for f in module.get_functions() {
        if !is_lambda_lifted(&f) {
            continue;
        }
        let lifted = f.get_qualified_id();
        let mut current = lifted;
        let mut visited = BTreeSet::new();
        let name = loop {
            let Some(host) = env
                .get_function(current)
                .get_using_functions()
                .and_then(|using| using.into_iter().next())
            else {
                break None;
            };
            if !visited.insert(host) {
                break None;
            }
            let host_env = env.get_function(host);
            if !is_lambda_lifted(&host_env) {
                break Some(host_env.get_name_str());
            }
            current = host;
        };
        if let Some(name) = name {
            result.insert(lifted, name);
        }
    }
    result
}

fn build_function_facts(
    env: &GlobalEnv,
    func: &FunctionEnv<'_>,
    defined_in: &BTreeMap<QualifiedId<FunId>, String>,
) -> FunctionFacts {
    let location = loc_to_file_span(env, &func.get_loc());
    let type_ctx = qualified_type_ctx(func.get_type_display_ctx());

    let symbol_pool = env.symbol_pool();
    let params: Vec<ParamFacts> = func
        .get_parameters_ref()
        .iter()
        .map(|p| ParamFacts {
            name: p.0.display(symbol_pool).to_string(),
            type_: p.1.display(&type_ctx).to_string(),
        })
        .collect();

    let return_types = format_return_types(func, &type_ctx);

    let acquires_inferred: Vec<String> = func
        .get_acquired_structs()
        .map(|set| {
            set.iter()
                .map(|sid| {
                    func.module_env
                        .get_struct(*sid)
                        .get_full_name_with_address()
                })
                .collect()
        })
        .unwrap_or_default();

    let resource_access = build_resource_access(env, func, &type_ctx);

    let attributes = attrs_to_facts(env, func.get_attributes());

    let is_view = func
        .get_attributes()
        .iter()
        .any(|a| symbol_pool.string(a.name()).as_str() == "view");

    FunctionFacts {
        name: func.get_name_str(),
        location,
        visibility: visibility_str(func),
        is_entry: func.is_entry(),
        is_inline: func.is_inline(),
        is_native: func.is_native(),
        is_view,
        attributes,
        type_params: type_params_to_facts(env, &func.get_type_parameters()),
        params,
        return_types,
        acquires_inferred,
        resource_access,
        is_lambda_lifted: is_lambda_lifted(func),
        defined_in: defined_in.get(&func.get_qualified_id()).cloned(),
    }
}

fn build_struct_facts(env: &GlobalEnv, s: &StructEnv<'_>) -> StructFacts {
    let location = loc_to_file_span(env, &s.get_loc());
    let type_ctx = qualified_type_ctx(s.get_type_display_ctx());
    let symbol_pool = env.symbol_pool();

    let abilities: Vec<String> = s
        .get_abilities()
        .into_iter()
        .map(|a| a.to_string())
        .collect();

    let type_params = type_params_to_facts(env, s.get_type_parameters());

    let (kind, fields, variants) = if s.has_variants() {
        let variants: Vec<VariantFacts> = s
            .get_variants()
            .map(|v| {
                let v_fields: Vec<FieldFacts> = s
                    .get_fields_of_variant(v)
                    .map(|f| FieldFacts {
                        name: f.get_name().display(symbol_pool).to_string(),
                        type_: f.get_type().display(&type_ctx).to_string(),
                        positional: f.is_positional(),
                    })
                    .collect();
                let v_attrs = attrs_to_facts(env, s.get_variant_attributes(v));
                let kind = if v_fields.is_empty() {
                    "unit"
                } else if v_fields.iter().all(|f| f.positional) {
                    "positional"
                } else {
                    "named"
                };
                VariantFacts {
                    name: v.display(symbol_pool).to_string(),
                    kind: kind.to_string(),
                    fields: v_fields,
                    attributes: v_attrs,
                }
            })
            .collect();
        ("enum", None, Some(variants))
    } else {
        let fields: Vec<FieldFacts> = s
            .get_fields()
            .map(|f| FieldFacts {
                name: f.get_name().display(symbol_pool).to_string(),
                type_: f.get_type().display(&type_ctx).to_string(),
                positional: f.is_positional(),
            })
            .collect();
        ("struct", Some(fields), None)
    };

    let attributes = attrs_to_facts(env, s.get_attributes());

    StructFacts {
        kind: kind.to_string(),
        name: s.get_name().display(symbol_pool).to_string(),
        location,
        abilities,
        type_params,
        fields,
        variants,
        attributes,
    }
}

fn type_params_to_facts(env: &GlobalEnv, params: &[TypeParameter]) -> Vec<TypeParamFacts> {
    let symbol_pool = env.symbol_pool();
    params
        .iter()
        .map(|tp| TypeParamFacts {
            name: tp.0.display(symbol_pool).to_string(),
            abilities: tp.1.abilities.into_iter().map(|a| a.to_string()).collect(),
            is_phantom: tp.1.is_phantom,
        })
        .collect()
}

fn attrs_to_facts(env: &GlobalEnv, attrs: &[Attribute]) -> Vec<AttributeFacts> {
    attrs.iter().map(|a| attr_to_facts(env, a)).collect()
}

/// Serialize a single attribute, preserving its arguments. `#[name(a, b = c)]`
/// becomes `{ name, args: [...] }`; `#[name = value]` becomes `{ name, value }`.
fn attr_to_facts(env: &GlobalEnv, attr: &Attribute) -> AttributeFacts {
    let symbol_pool = env.symbol_pool();
    match attr {
        Attribute::Apply(_, name, sub) => AttributeFacts {
            name: symbol_pool.string(*name).to_string(),
            value: None,
            args: sub.iter().map(|a| attr_to_facts(env, a)).collect(),
        },
        Attribute::Assign(_, name, val) => AttributeFacts {
            name: symbol_pool.string(*name).to_string(),
            value: Some(attr_value_to_string(env, val)),
            args: vec![],
        },
    }
}

/// Render an attribute value: literals via the model's value display, named
/// paths as fully-qualified `address::module::name` (or a bare name).
fn attr_value_to_string(env: &GlobalEnv, val: &AttributeValue) -> String {
    match val {
        AttributeValue::Value(_, v) => env.display(v).to_string(),
        AttributeValue::Name(_, module_opt, sym) => {
            let name = sym.display(env.symbol_pool()).to_string();
            match module_opt {
                Some(module) => format!("{}::{}", module.display_full(env), name),
                None => name,
            }
        },
    }
}

fn visibility_str(func: &FunctionEnv<'_>) -> String {
    if func.has_package_visibility() {
        return "package".to_string();
    }
    match func.visibility() {
        Visibility::Public => "public",
        Visibility::Friend => "friend",
        Visibility::Private => "internal",
    }
    .to_string()
}

/// A type-display context that renders every struct fully-qualified as `address::module::Name<...>`
fn qualified_type_ctx(base: TypeDisplayContext<'_>) -> TypeDisplayContext<'_> {
    TypeDisplayContext {
        display_module_addr: true,
        use_module_qualification: true,
        ..base
    }
}

/// Render a function's return as a list of types: `[]` for no return, one entry
/// for a scalar, and one entry per element for a tuple (multiple) return.
fn format_return_types(func: &FunctionEnv<'_>, type_ctx: &TypeDisplayContext<'_>) -> Vec<String> {
    func.get_result_type()
        .flatten()
        .iter()
        .map(|t| t.display(type_ctx).to_string())
        .collect()
}
/// Empty when the function has no AST body.
fn build_resource_access(
    env: &GlobalEnv,
    func: &FunctionEnv<'_>,
    type_ctx: &TypeDisplayContext<'_>,
) -> ResourceAccessFacts {
    let Some(body) = func.get_def() else {
        return ResourceAccessFacts {
            reads: vec![],
            writes: vec![],
        };
    };
    let mut reads = BTreeSet::new();
    let mut writes = BTreeSet::new();
    body.visit_pre_order(&mut |e| {
        if let ExpData::Call(node_id, op, _) = e {
            let (does_read, does_write) = match op {
                Operation::Exists(_) | Operation::BorrowGlobal(ReferenceKind::Immutable) => {
                    (true, false)
                },
                Operation::BorrowGlobal(ReferenceKind::Mutable) | Operation::MoveFrom => {
                    (true, true)
                },
                Operation::MoveTo => (false, true),
                _ => (false, false),
            };
            if does_read || does_write {
                let insts = env.get_node_instantiation(*node_id);
                if let Some(ty) = insts.first() {
                    let ty = ty.display(type_ctx).to_string();
                    if does_read {
                        reads.insert(ty.clone());
                    }
                    if does_write {
                        writes.insert(ty);
                    }
                }
            }
        }
        true
    });
    ResourceAccessFacts {
        reads: reads.into_iter().collect(),
        writes: writes.into_iter().collect(),
    }
}

/// Resolve a `Loc` to its source file and 1-indexed `[start_line, end_line]`.
/// The span is `None` when either endpoint cannot be resolved.
fn loc_to_file_span(env: &GlobalEnv, loc: &Loc) -> SourceLocation {
    let file = Some(env.get_file(loc.file_id()).to_string_lossy().into_owned());
    let start = env.get_location(loc).map(|l| l.line.0 + 1);
    let end = env.get_location(&loc.at_end()).map(|l| l.line.0 + 1);
    let span = start.zip(end).map(|(s, e)| [s, e]);
    SourceLocation { file, span }
}
