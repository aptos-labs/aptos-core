// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Move package query tools.

use super::super::{
    common::{mcp_err, resolve_function, tool_error, try_call},
    session::{into_call_tool_result, FlowSession},
};
use move_model::{
    ast::{
        AccessSpecifier, AccessSpecifierKind, AddressSpecifier, Attribute, ExpData, Operation,
        ResourceSpecifier,
    },
    model::{
        FunId, FunctionEnv, GlobalEnv, Loc, ModuleEnv, NamedConstantEnv, QualifiedId, StructEnv,
        TypeParameter, Visibility,
    },
    ty::ReferenceKind,
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
    /// Returns per-module facts: functions, types, constants, friends,
    /// attributes, and source locations.
    Facts,
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
                let result = build_facts(data.env());
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

#[derive(Debug, serde::Serialize)]
#[serde(rename_all = "camelCase")]
struct ModuleFacts {
    #[serde(skip_serializing_if = "Option::is_none")]
    file: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    span: Option<[u32; 2]>,
    friends: Vec<FriendFacts>,
    attributes: Vec<AttributeFacts>,
    functions: Vec<FunctionFacts>,
    types: Vec<TypeFacts>,
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
struct DeclaredAccessFacts {
    /// One of `"reads"`, `"writes"`, `"legacy_acquires"`.
    kind: String,
    resource: DeclaredAccessResource,
    address: DeclaredAccessAddress,
    negated: bool,
}

#[derive(Debug, serde::Serialize)]
#[serde(rename_all = "camelCase")]
struct DeclaredAccessResource {
    /// One of `"any"`, `"at_address"`, `"in_module"`, `"resource"`.
    form: String,
    value: String,
}

#[derive(Debug, serde::Serialize)]
#[serde(rename_all = "camelCase")]
struct DeclaredAccessAddress {
    /// One of `"any"`, `"address"`, `"parameter"`, `"call"`.
    form: String,
    value: String,
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
    #[serde(skip_serializing_if = "Option::is_none")]
    file: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    span: Option<[u32; 2]>,
    /// One of `"public"`, `"friend"`, `"package"`, `"internal"`.
    visibility: String,
    is_entry: bool,
    is_inline: bool,
    is_native: bool,
    is_view: bool,
    attributes: Vec<AttributeFacts>,
    type_params: Vec<TypeParamFacts>,
    params: Vec<ParamFacts>,
    return_type: Option<String>,
    declared_access: Vec<DeclaredAccessFacts>,
    acquires_inferred: Vec<String>,
    resource_access: ResourceAccessFacts,
}

#[derive(Debug, serde::Serialize)]
#[serde(rename_all = "camelCase")]
struct TypeFacts {
    /// `"struct"` or `"enum"`.
    kind: String,
    name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    file: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    span: Option<[u32; 2]>,
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
    let (file, span) = loc_to_file_span(env, &module.get_loc());

    let friends: Vec<FriendFacts> = module
        .get_friend_modules()
        .iter()
        .map(|id| FriendFacts {
            module: env.get_module(*id).get_full_name_str(),
        })
        .collect();

    let attributes = attrs_to_facts(env, module.get_attributes());

    let functions: Vec<FunctionFacts> = module
        .get_functions()
        .map(|f| build_function_facts(env, &f))
        .collect();

    let types: Vec<TypeFacts> = module
        .get_structs()
        .map(|s| build_type_facts(env, &s))
        .collect();

    let constants: Vec<ConstantSummary> = module
        .get_named_constants()
        .map(|c| build_constant_summary(env, &c))
        .collect();

    ModuleFacts {
        file,
        span,
        friends,
        attributes,
        functions,
        types,
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

fn build_function_facts(env: &GlobalEnv, func: &FunctionEnv<'_>) -> FunctionFacts {
    let (file, span) = loc_to_file_span(env, &func.get_loc());
    let type_ctx = func.get_type_display_ctx();

    let symbol_pool = env.symbol_pool();
    let params: Vec<ParamFacts> = func
        .get_parameters_ref()
        .iter()
        .map(|p| ParamFacts {
            name: p.0.display(symbol_pool).to_string(),
            type_: p.1.display(&type_ctx).to_string(),
        })
        .collect();

    let return_type = format_return_type(func, &type_ctx);

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

    let declared_access: Vec<DeclaredAccessFacts> = func
        .get_access_specifiers()
        .map(|specs| specs.iter().map(|s| access_spec_to_facts(env, s)).collect())
        .unwrap_or_default();

    let resource_access = build_resource_access(env, func, &type_ctx);

    let attributes = attrs_to_facts(env, func.get_attributes());
    let is_view = func
        .get_attributes()
        .iter()
        .any(|a| symbol_pool.string(a.name()).as_str() == "view");

    FunctionFacts {
        name: func.get_name_str(),
        file,
        span,
        visibility: visibility_str(func),
        is_entry: func.is_entry(),
        is_inline: func.is_inline(),
        is_native: func.is_native(),
        is_view,
        attributes,
        type_params: type_params_to_facts(env, &func.get_type_parameters()),
        params,
        return_type,
        declared_access,
        acquires_inferred,
        resource_access,
    }
}

fn build_type_facts(env: &GlobalEnv, s: &StructEnv<'_>) -> TypeFacts {
    let (file, span) = loc_to_file_span(env, &s.get_loc());
    let type_ctx = s.get_type_display_ctx();
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

    TypeFacts {
        kind: kind.to_string(),
        name: s.get_name().display(symbol_pool).to_string(),
        file,
        span,
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
    let symbol_pool = env.symbol_pool();
    attrs
        .iter()
        .map(|a| AttributeFacts {
            name: symbol_pool.string(a.name()).to_string(),
        })
        .collect()
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

fn format_return_type(
    func: &FunctionEnv<'_>,
    type_ctx: &move_model::ty::TypeDisplayContext<'_>,
) -> Option<String> {
    use move_model::ty::Type;
    let result = func.get_result_type();
    if let Type::Tuple(ts) = &result
        && ts.is_empty()
    {
        return None;
    }
    Some(result.display(type_ctx).to_string())
}

fn access_spec_to_facts(env: &GlobalEnv, spec: &AccessSpecifier) -> DeclaredAccessFacts {
    let kind = match spec.kind {
        AccessSpecifierKind::Reads => "reads",
        AccessSpecifierKind::Writes => "writes",
        AccessSpecifierKind::LegacyAcquires => "legacy_acquires",
    };
    let (form, value) = match &spec.resource.1 {
        ResourceSpecifier::Any => ("any", "*".to_string()),
        ResourceSpecifier::DeclaredAtAddress(addr) => ("at_address", env.display(addr).to_string()),
        ResourceSpecifier::DeclaredInModule(mid) => {
            ("in_module", env.get_module(*mid).get_full_name_str())
        },
        ResourceSpecifier::Resource(qid) => {
            let struct_env = env.get_struct(qid.to_qualified_id());
            let mut s = struct_env.get_full_name_with_address();
            if !qid.inst.is_empty() {
                let ctx = struct_env.get_type_display_ctx();
                let args: Vec<String> = qid
                    .inst
                    .iter()
                    .map(|t| t.display(&ctx).to_string())
                    .collect();
                s.push('<');
                s.push_str(&args.join(", "));
                s.push('>');
            }
            ("resource", s)
        },
    };
    let address = address_spec_to_facts(env, &spec.address.1);
    DeclaredAccessFacts {
        kind: kind.to_string(),
        resource: DeclaredAccessResource {
            form: form.to_string(),
            value,
        },
        address,
        negated: spec.negated,
    }
}

fn address_spec_to_facts(env: &GlobalEnv, addr: &AddressSpecifier) -> DeclaredAccessAddress {
    let symbol_pool = env.symbol_pool();
    let (form, value) = match addr {
        AddressSpecifier::Any => ("any", "*".to_string()),
        AddressSpecifier::Address(a) => ("address", env.display(a).to_string()),
        AddressSpecifier::Parameter(sym) => ("parameter", sym.display(symbol_pool).to_string()),
        AddressSpecifier::Call(qid, sym) => {
            let fun = env.get_function(qid.to_qualified_id());
            let mut s = fun.get_full_name_with_address();
            if !qid.inst.is_empty() {
                let ctx = fun.get_type_display_ctx();
                let args: Vec<String> = qid
                    .inst
                    .iter()
                    .map(|t| t.display(&ctx).to_string())
                    .collect();
                s.push('<');
                s.push_str(&args.join(", "));
                s.push('>');
            }
            s.push('(');
            s.push_str(&sym.display(symbol_pool).to_string());
            s.push(')');
            ("call", s)
        },
    };
    DeclaredAccessAddress {
        form: form.to_string(),
        value,
    }
}

/// Walk the function body for `borrow_global`/`move_from`/`move_to` and collect
/// the resource types read and written. Empty when the function has no AST body.
fn build_resource_access(
    env: &GlobalEnv,
    func: &FunctionEnv<'_>,
    type_ctx: &move_model::ty::TypeDisplayContext<'_>,
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
            let bucket = match op {
                Operation::BorrowGlobal(ReferenceKind::Immutable) => Some(&mut reads),
                Operation::BorrowGlobal(ReferenceKind::Mutable)
                | Operation::MoveFrom
                | Operation::MoveTo => Some(&mut writes),
                _ => None,
            };
            if let Some(bucket) = bucket {
                let insts = env.get_node_instantiation(*node_id);
                if let Some(ty) = insts.first() {
                    bucket.insert(ty.display(type_ctx).to_string());
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
fn loc_to_file_span(env: &GlobalEnv, loc: &Loc) -> (Option<String>, Option<[u32; 2]>) {
    let file = Some(env.get_file(loc.file_id()).to_string_lossy().into_owned());
    let start = env.get_location(loc).map(|l| l.line.0 + 1);
    let end = env.get_location(&loc.at_end()).map(|l| l.line.0 + 1);
    let span = start.zip(end).map(|(s, e)| [s, e]);
    (file, span)
}
