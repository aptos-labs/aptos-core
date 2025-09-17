// Copyright (c) Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{
    deps::{PkgDefinition, PkgKind},
    prep::{
        canvas::{DriverCanvas, LambdaBinding, ScriptSignature},
        datatype::DatatypeRegistry,
        function::{FunctionDecl, FunctionRegistry},
        graph::GraphBuilder,
        ident::FunctionIdent,
        typing::{TypeBase, TypeItem, TypeRef, TypeSubstitution},
    },
};
use itertools::Itertools;
use legacy_move_compiler::compiled_unit::CompiledUnit;
use log::{debug, info, warn};
use move_core_types::ability::AbilitySet;
use move_package::compilation::compiled_package::CompiledUnitWithSource;
use serde_json::json;
use std::{
    cmp::Reverse,
    collections::{BTreeMap, BTreeSet},
    fs,
    path::{Path, PathBuf},
    time::{Duration, Instant},
};
use walkdir::WalkDir;

/// Hard caps to avoid exploding script counts with near-duplicate wrappers.
const MAX_SCRIPTS_PER_FUNCTION: usize = 24;

fn write_script_generation_progress(
    progress_path: &Path,
    total_primary_functions: usize,
    processed_primary_functions: usize,
    generated_scripts: usize,
    zero_script_functions: usize,
    limited_functions: usize,
    current_function: Option<&FunctionDecl>,
    elapsed_secs: f64,
) {
    let payload = json!({
        "stage": "script_generation",
        "total_primary_functions": total_primary_functions,
        "processed_primary_functions": processed_primary_functions,
        "generated_scripts": generated_scripts,
        "zero_script_functions": zero_script_functions,
        "limited_functions": limited_functions,
        "current_function": current_function.map(|decl| decl.ident.to_string()),
        "elapsed_secs": elapsed_secs,
    });
    if let Ok(encoded) = serde_json::to_vec_pretty(&payload) {
        let _ = fs::write(progress_path, encoded);
    }
}

/// A database that holds information we can statically get from the packages
pub struct Model {
    pub datatype_registry: DatatypeRegistry,
    pub function_registry: FunctionRegistry,
}

impl Model {
    /// Provision the model with a list of packages
    pub fn new(pkgs: &[PkgDefinition]) -> Self {
        // initialize the datatype registry
        let mut datatype_registry = DatatypeRegistry::new();
        for pkg in pkgs {
            let pkg_kind = pkg.kind;
            let pkg_details = pkg.package.compiled_package();
            for CompiledUnitWithSource {
                unit,
                source_path: _,
            } in &pkg_details.root_compiled_units
            {
                let module = match unit {
                    CompiledUnit::Script(_) => continue,
                    CompiledUnit::Module(m) => &m.module,
                };

                // go over all datatypes defined
                datatype_registry.analyze(module, pkg_kind);
            }
            debug!(
                "datatype registry populated for package {}",
                pkg_details.compiled_package_info.package_name
            );
        }

        // initialize the function registry
        let mut function_registry = FunctionRegistry::new();
        for pkg in pkgs {
            let pkg_kind = pkg.kind;
            let pkg_details = pkg.package.compiled_package();
            let package_root = pkg.manifest_path.as_path();
            let module_source_index = index_package_module_sources(package_root);
            for CompiledUnitWithSource { unit, source_path } in &pkg_details.root_compiled_units {
                let module = match unit {
                    CompiledUnit::Script(_) => continue,
                    CompiledUnit::Module(m) => &m.module,
                };
                let module_name = module.self_id().to_string();
                let source_text = read_module_source(
                    package_root,
                    source_path,
                    &module_name,
                    &module_source_index,
                );
                if source_text.is_none() && matches!(pkg_kind, PkgKind::Primary) {
                    debug!(
                        "missing source text for module {} (source_path: {}, package_root: {})",
                        module.self_id(),
                        source_path.display(),
                        package_root.display(),
                    );
                }

                // go over all datatypes defined
                function_registry.analyze(
                    &mut datatype_registry,
                    module,
                    pkg_kind,
                    source_text.as_deref(),
                );
            }
            debug!(
                "function registry populated for package {}",
                pkg_details.compiled_package_info.package_name
            );
        }

        // done
        Self {
            datatype_registry,
            function_registry,
        }
    }

    /// Populate the flow graphs for targeted functions which might be entrypoints
    pub fn populate(
        &self,
        max_trace_depth: usize,
        max_call_repetition: usize,
        max_script_gen_secs_per_function: u64,
        script_output_dir: &Path,
        progress_path: Option<&Path>,
    ) -> Vec<ScriptSignature> {
        // initialize the graph builder
        let max_script_gen_time_per_function = (max_script_gen_secs_per_function != 0)
            .then(|| Duration::from_secs(max_script_gen_secs_per_function));
        let mut builder = GraphBuilder::new(
            self,
            max_trace_depth,
            max_call_repetition,
            max_script_gen_time_per_function,
        );

        // count primary functions per module
        let mut primary_func_count = 0;
        let mut module_script_counts: BTreeMap<String, usize> = BTreeMap::new();

        // Dedup key set for generated scripts.
        let mut generated_keys = BTreeSet::new();

        // Sort primary declarations to process entry functions first.
        let mut primary_decls: Vec<_> = self
            .function_registry
            .iter_decls()
            .filter(|decl| matches!(decl.kind, PkgKind::Primary))
            .collect();
        primary_decls.sort_by_key(|decl| (Reverse(decl.is_entry), decl.ident.clone()));
        let total_primary_functions = primary_decls.len();
        let generation_started = Instant::now();
        let mut zero_script_functions = 0usize;
        let mut limited_functions = 0usize;
        if let Some(progress_path) = progress_path {
            write_script_generation_progress(
                progress_path,
                total_primary_functions,
                0,
                0,
                0,
                0,
                None,
                generation_started.elapsed().as_secs_f64(),
            );
        }

        // go over primary declarations (entry-first)
        let mut generated_scripts = vec![];
        for decl in primary_decls {
            primary_func_count += 1;
            let module_key = format!("{}::{}", decl.ident.address(), decl.ident.module_name());

            info!(
                "processing primary function: {} (entry: {}, generics: {}, params: {}, returns: {})",
                decl.ident,
                decl.is_entry,
                decl.generics.len(),
                decl.parameters.len(),
                decl.return_sig.len(),
            );

            if let Some(progress_path) = progress_path {
                write_script_generation_progress(
                    progress_path,
                    total_primary_functions,
                    primary_func_count - 1,
                    generated_scripts.len(),
                    zero_script_functions,
                    limited_functions,
                    Some(decl),
                    generation_started.elapsed().as_secs_f64(),
                );
            }

            // try to instantiate the function with identity type arguments with varying ability sets
            let num_combos: usize = decl
                .generics
                .iter()
                .map(|constraint| ability_set_candidates(*constraint).len())
                .product();
            info!("- {num_combos} ability set combinations to explore");

            let mut scripts_for_func = 0usize;
            let mut function_limited = false;
            let module_count = module_script_counts.entry(module_key.clone()).or_insert(0);

            'combo_loop: for combo in decl
                .generics
                .iter()
                .map(|constraint| ability_set_candidates(*constraint))
                .multi_cartesian_product()
            {
                assert_eq!(combo.len(), decl.generics.len());
                let type_args: Vec<_> = combo
                    .into_iter()
                    .enumerate()
                    .map(|(index, abilities)| TypeBase::Param { index, abilities })
                    .collect();

                // identify Function-typed params and find matching candidates
                let lambda_params = find_lambda_params(self, decl, &type_args);

                // build per-param candidate lists
                let mut any_param_infeasible = None;
                let mut per_param_candidates = vec![];
                for (idx, fn_params, fn_returns) in lambda_params {
                    let candidates = find_matching_functions(self, decl, &fn_params, &fn_returns);
                    if candidates.is_empty() {
                        any_param_infeasible = Some(idx);
                        break;
                    }
                    per_param_candidates.push((idx, fn_params, candidates));
                }
                if let Some(idx) = any_param_infeasible {
                    debug!("  -> skipping lambda instantiation: no candidates for param {idx}");
                    continue;
                }

                // cartesian product of lambda candidates
                // (if no `Function` params, yields a single empty combination)
                let lambda_combos: Vec<Vec<_>> = if per_param_candidates.is_empty() {
                    vec![vec![]]
                } else {
                    per_param_candidates
                        .iter()
                        .map(|(_, _, candidates)| candidates.iter().enumerate().collect::<Vec<_>>())
                        .multi_cartesian_product()
                        .collect()
                };

                for lambda_combo in lambda_combos {
                    // build the bindings map
                    let mut bindings = BTreeMap::new();
                    for (param_pos, (_, candidate)) in lambda_combo.into_iter().enumerate() {
                        let (param_idx, fn_params, _) = &per_param_candidates[param_pos];
                        let (fn_ident, fn_type_args) = candidate;
                        bindings.insert(*param_idx, LambdaBinding {
                            fn_params: fn_params.clone(),
                            fn_ident: fn_ident.clone(),
                            fn_type_args: fn_type_args.clone(),
                        });
                    }

                    // build flow graphs for this instantiation + lambda combination
                    let raw_graphs = builder.process(decl, &type_args);
                    if let Some(limit_reason) = builder.process_limit_hit() {
                        warn!(
                            "  -> graph exploration truncated by {limit_reason} for {}",
                            decl.ident
                        );
                        if !function_limited {
                            function_limited = true;
                            limited_functions += 1;
                        }
                    }

                    let raw_count = raw_graphs.len();
                    let mut feasible_count = 0;
                    for graph in raw_graphs {
                        if scripts_for_func >= MAX_SCRIPTS_PER_FUNCTION {
                            debug!(
                                "reached per-function cap ({MAX_SCRIPTS_PER_FUNCTION}) for {}",
                                decl.ident
                            );
                            break 'combo_loop;
                        }
                        if builder.is_feasible(&graph) {
                            let graph = graph.compact_generics();
                            let Some(canvas) = DriverCanvas::try_build(self, &graph, &bindings)
                            else {
                                debug!(
                                    "  -> skipping incomplete canvas after feasibility check for {}",
                                    decl.ident
                                );
                                continue;
                            };
                            let dedup_key = canvas.dedup_key(&decl.ident);
                            if !generated_keys.insert(dedup_key) {
                                continue;
                            }
                            let script = canvas.generate_script(
                                generated_scripts.len(),
                                &decl.ident,
                                script_output_dir,
                            );
                            generated_scripts.push(script);
                            feasible_count += 1;
                            scripts_for_func += 1;
                            *module_count += 1;
                        }
                    }
                    debug!(
                        "  -> instantiation produced {raw_count} graphs, {feasible_count} feasible"
                    );
                }
            }

            info!("  -> {scripts_for_func} script(s) for {}", decl.ident);
            if scripts_for_func == 0 {
                zero_script_functions += 1;
            }
            if let Some(progress_path) = progress_path {
                write_script_generation_progress(
                    progress_path,
                    total_primary_functions,
                    primary_func_count,
                    generated_scripts.len(),
                    zero_script_functions,
                    limited_functions,
                    Some(decl),
                    generation_started.elapsed().as_secs_f64(),
                );
            }
        }

        // print summary
        info!("========== Script Generation Summary ==========");
        info!("Total primary functions analyzed: {primary_func_count}");
        info!("Total scripts generated: {}", generated_scripts.len());
        info!("Per-module breakdown:");
        for (module, count) in &module_script_counts {
            info!("  {module}: {count} script(s)");
        }
        info!("================================================");
        if let Some(progress_path) = progress_path {
            write_script_generation_progress(
                progress_path,
                total_primary_functions,
                total_primary_functions,
                generated_scripts.len(),
                zero_script_functions,
                limited_functions,
                None,
                generation_started.elapsed().as_secs_f64(),
            );
        }

        // done
        generated_scripts
    }
}

fn read_module_source(
    package_root: &Path,
    source_path: &Path,
    module_id: &str,
    module_source_index: &BTreeMap<String, PathBuf>,
) -> Option<String> {
    if let Some(indexed_path) = module_source_index.get(module_id) {
        if let Ok(src) = fs::read_to_string(indexed_path) {
            return Some(src);
        }
    }

    fs::read_to_string(source_path)
        .or_else(|_| {
            if source_path.is_absolute() {
                Err(std::io::Error::new(
                    std::io::ErrorKind::NotFound,
                    "absolute source path not readable",
                ))
            } else {
                fs::read_to_string(package_root.join(source_path))
            }
        })
        .ok()
}

fn index_package_module_sources(package_root: &Path) -> BTreeMap<String, PathBuf> {
    let mut index = BTreeMap::new();

    for relative_root in ["sources", "scripts"] {
        let dir = package_root.join(relative_root);
        if !dir.exists() {
            continue;
        }

        for entry in WalkDir::new(dir).into_iter().filter_map(Result::ok) {
            let path = entry.path();
            if !entry.file_type().is_file() || path.extension().is_none_or(|ext| ext != "move") {
                continue;
            }

            let source = match fs::read_to_string(path) {
                Ok(s) => s,
                Err(_) => continue,
            };

            for module_id in parse_declared_module_ids(&source) {
                index.entry(module_id).or_insert_with(|| path.to_path_buf());
            }
        }
    }

    index
}

fn parse_declared_module_ids(source: &str) -> Vec<String> {
    fn strip_comments(source: &str) -> String {
        let mut output = String::with_capacity(source.len());
        let bytes = source.as_bytes();
        let mut idx = 0;
        let mut block_comment_depth = 0usize;
        while idx < bytes.len() {
            let ch = bytes[idx] as char;
            if block_comment_depth > 0 {
                if ch == '/' && idx + 1 < bytes.len() && bytes[idx + 1] as char == '*' {
                    block_comment_depth += 1;
                    idx += 2;
                    continue;
                }
                if ch == '*' && idx + 1 < bytes.len() && bytes[idx + 1] as char == '/' {
                    block_comment_depth -= 1;
                    idx += 2;
                    continue;
                }
                if ch == '\n' {
                    output.push('\n');
                }
                idx += 1;
                continue;
            }
            if ch == '/' && idx + 1 < bytes.len() {
                let next = bytes[idx + 1] as char;
                if next == '/' {
                    idx += 2;
                    while idx < bytes.len() && bytes[idx] as char != '\n' {
                        idx += 1;
                    }
                    continue;
                }
                if next == '*' {
                    block_comment_depth = 1;
                    idx += 2;
                    continue;
                }
            }

            output.push(ch);
            idx += 1;
        }
        output
    }

    let source = strip_comments(source);
    let mut ids = vec![];
    for line in source.lines() {
        let s = line.trim_start();
        let Some(rest) = s.strip_prefix("module ") else {
            continue;
        };
        let module_spec: String = rest
            .chars()
            .take_while(|c| !c.is_whitespace() && *c != '{')
            .collect();
        if module_spec.contains("::") {
            ids.push(module_spec);
        }
    }
    ids
}

/// Identify `Function`-typed parameters in a function instantiation (decl + type args).
///
/// Returns a list of (param_index, fn_params, fn_returns) for each `Function`-typed parameter.
fn find_lambda_params(
    model: &Model,
    decl: &FunctionDecl,
    type_args: &[TypeBase],
) -> Vec<(usize, Vec<TypeItem>, Vec<TypeItem>)> {
    let mut result = vec![];
    for (idx, ty) in decl.parameters.iter().enumerate() {
        let ty_inst = model.datatype_registry.instantiate_type_ref(ty, type_args);

        // extract the base type (unwrapping references)
        let ty_base = match &ty_inst {
            TypeItem::Base(b) | TypeItem::ImmRef(b) | TypeItem::MutRef(b) => b,
        };

        // collect the function type
        match ty_base {
            TypeBase::Function {
                params,
                returns,
                abilities: _,
            } => {
                result.push((idx, params.clone(), returns.clone()));
            },
            _ => continue,
        }
    }
    result
}

/// Find functions whose signature matches the given function type.
///
/// A function matches if:
/// 1. Its parameter count equals fn_params length
/// 2. Its return count equals fn_returns length
/// 3. TypeSubstitution successfully unifies all params and returns
/// 4. All generic type parameters are fully resolved
fn find_matching_functions(
    model: &Model,
    caller_decl: &FunctionDecl,
    fn_params: &[TypeItem],
    fn_returns: &[TypeItem],
) -> Vec<(FunctionIdent, Vec<TypeBase>)> {
    let mut matches = vec![];

    for decl in model
        .function_registry
        .iter_decls()
        .filter(|decl| decl.kind.is_external_provider_candidate())
        .sorted_by_key(|decl| (decl.kind.external_provider_rank(), decl.ident.clone()))
    {
        // Avoid immediately recursing into the same function as callback target.
        if decl.ident == caller_decl.ident {
            continue;
        }

        // check arity
        if decl.parameters.len() != fn_params.len() {
            continue;
        }
        if decl.return_sig.len() != fn_returns.len() {
            continue;
        }

        // try to unify params and returns
        let mut unifier = TypeSubstitution::new(&decl.generics);

        let mut ok = true;
        for (decl_param, fn_param) in decl.parameters.iter().zip(fn_params.iter()) {
            if !unify_ref(&mut unifier, decl_param, fn_param) {
                ok = false;
                break;
            }
        }
        if !ok {
            continue;
        }
        for (decl_ret, fn_ret) in decl.return_sig.iter().zip(fn_returns.iter()) {
            if !unify_ref(&mut unifier, decl_ret, fn_ret) {
                ok = false;
                break;
            }
        }
        if !ok {
            continue;
        }

        // check all generics are resolved
        let unified = unifier.finish();
        if unified.iter().any(|u| u.is_none()) {
            continue;
        }

        let type_args: Vec<_> = unified.into_iter().map(|u| u.unwrap()).collect();
        matches.push((decl.ident.clone(), type_args));
    }

    matches
}

/// Unify a TypeRef (from function decl) against a TypeItem (from function type)
fn unify_ref(unifier: &mut TypeSubstitution, decl_ty: &TypeRef, fn_ty: &TypeItem) -> bool {
    match (decl_ty, fn_ty) {
        (TypeRef::Base(tag), TypeItem::Base(base))
        | (TypeRef::ImmRef(tag), TypeItem::ImmRef(base))
        | (TypeRef::MutRef(tag), TypeItem::MutRef(base)) => unifier.unify(tag, base),
        _ => false,
    }
}

// Utility: enumerate ability sets that satisfy the constraints
pub fn ability_set_candidates(constraint: AbilitySet) -> Vec<AbilitySet> {
    (AbilitySet::ALL.setminus(constraint))
        .into_iter()
        .powerset()
        .map(|set| {
            let mut combined = constraint;
            for ability in set {
                combined = combined.add(ability);
            }
            combined
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::{ability_set_candidates, parse_declared_module_ids};
    use move_core_types::ability::{Ability, AbilitySet};

    #[test]
    fn test_parse_declared_module_ids_ignores_comments() {
        let source = r#"
            /*
                module 0x1::commented_out {}
            */
            module 0x1::real {}
            // module 0x1::also_commented {}
        "#;
        assert_eq!(parse_declared_module_ids(source), vec![
            "0x1::real".to_string()
        ]);
    }

    #[test]
    fn test_parse_declared_module_ids_supports_multiple_modules() {
        let source = r#"
            module 0x1::first {}
            module 0x2::second{
            }
        "#;
        assert_eq!(parse_declared_module_ids(source), vec![
            "0x1::first".to_string(),
            "0x2::second".to_string(),
        ]);
    }

    #[test]
    fn test_ability_set_candidates_are_supersets_of_constraint() {
        let constraint = AbilitySet::EMPTY.add(Ability::Copy).add(Ability::Drop);
        let candidates = ability_set_candidates(constraint);
        assert!(!candidates.is_empty());
        assert!(candidates
            .iter()
            .all(|candidate| constraint.is_subset(*candidate)));
        assert!(candidates.contains(&constraint));
    }
}
