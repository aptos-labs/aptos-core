// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Module selection: the modules a verification of some modules reads, so
//! that an export for it carries those and no others.

use anyhow::{bail, Result};
use move_model::{
    ast::ExpData,
    model::{GlobalEnv, ModuleId},
};
use std::collections::BTreeSet;

/// The modules `selectors` name, each as `module`, `address::module`, or
/// `alias::module`. A selector naming no module is an error.
pub fn select_modules(env: &GlobalEnv, selectors: &[String]) -> Result<BTreeSet<ModuleId>> {
    let mut selected = BTreeSet::new();
    for selector in selectors {
        let matching: Vec<ModuleId> = env
            .get_modules()
            .filter(|module| module.matches_name(selector))
            .map(|module| module.get_id())
            .collect();
        if matching.is_empty() {
            bail!("no module matches `{}`", selector);
        }
        selected.extend(matching);
    }
    Ok(selected)
}

/// The modules verifying `selected` reads: the modules they use in code and
/// in specifications, transitively, and the modules whose global invariants
/// read memory declared in one of those, with what those use in turn.
pub fn module_closure(env: &GlobalEnv, selected: &BTreeSet<ModuleId>) -> BTreeSet<ModuleId> {
    let mut closure = BTreeSet::new();
    let mut worklist: Vec<ModuleId> = selected.iter().copied().collect();
    loop {
        while let Some(id) = worklist.pop() {
            if closure.insert(id) {
                worklist.extend(dependencies(env, id));
            }
        }
        worklist.extend(
            env.get_modules()
                .map(|module| module.get_id())
                .filter(|id| !closure.contains(id) && constrains(env, *id, &closure)),
        );
        if worklist.is_empty() {
            return closure;
        }
    }
}

/// The modules `module` uses: through the functions its code calls or
/// references, the types its declarations and expressions mention, and its
/// specifications. A model built without bytecode records only the
/// specifications' part, so the code is walked here.
fn dependencies(env: &GlobalEnv, module: ModuleId) -> BTreeSet<ModuleId> {
    let module_env = env.get_module(module);
    let mut usage = module_env.get_used_modules(true);
    let use_exp = |usage: &mut BTreeSet<ModuleId>, exp: &ExpData| {
        exp.module_usage(usage);
        usage.extend(
            exp.used_funs()
                .into_iter()
                .map(|function| function.module_id),
        );
        for node in exp.node_ids() {
            env.get_node_type(node).module_usage(usage);
            for ty in env.get_node_instantiation(node) {
                ty.module_usage(usage);
            }
        }
    };
    for function in module_env.get_functions() {
        for parameter in function.get_parameters() {
            parameter.1.module_usage(&mut usage);
        }
        function.get_result_type().module_usage(&mut usage);
        if let Some(body) = function.get_def() {
            use_exp(&mut usage, body);
        }
    }
    for struct_env in module_env.get_structs() {
        for field in struct_env.get_fields().chain(struct_env.get_ghost_fields()) {
            field.get_type().module_usage(&mut usage);
        }
    }
    for (_, decl) in module_env.get_spec_funs() {
        for parameter in &decl.params {
            parameter.1.module_usage(&mut usage);
        }
        decl.result_type.module_usage(&mut usage);
    }
    for (_, decl) in module_env.get_spec_vars() {
        decl.type_.module_usage(&mut usage);
    }
    usage.remove(&module);
    usage
}

/// Whether a global invariant of `module` reads memory declared in `modules`.
fn constrains(env: &GlobalEnv, module: ModuleId, modules: &BTreeSet<ModuleId>) -> bool {
    env.get_global_invariants_by_module(module)
        .into_iter()
        .filter_map(|id| env.get_global_invariant(id))
        .any(|invariant| {
            invariant
                .mem_usage
                .iter()
                .any(|memory| modules.contains(&memory.module_id))
        })
}
