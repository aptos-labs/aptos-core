// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Module selection: the modules a verification of some modules reads, so
//! that an export for it carries those and no others.

use anyhow::{bail, Result};
use move_model::{
    ast::{collect_proof_exps, ExpData, Proof, Spec},
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
    // A proof reads its expressions and the lemmas it applies.
    let use_proof = |usage: &mut BTreeSet<ModuleId>, proof: &Proof| {
        let mut exps = vec![];
        collect_proof_exps(proof, &mut exps);
        for exp in exps {
            use_exp(usage, exp);
        }
        applied_lemma_modules(proof, usage);
    };
    // A specification also calls functions, which the module usage of
    // specifications does not record.
    let use_spec = |usage: &mut BTreeSet<ModuleId>, spec: &Spec| {
        for condition in &spec.conditions {
            use_exp(usage, &condition.exp);
            for exp in &condition.additional_exps {
                use_exp(usage, exp);
            }
        }
    };
    use_spec(&mut usage, &module_env.get_spec());
    for function in module_env.get_functions() {
        for parameter in function.get_parameters() {
            parameter.1.module_usage(&mut usage);
        }
        function.get_result_type().module_usage(&mut usage);
        if let Some(body) = function.get_def() {
            use_exp(&mut usage, body);
        }
        use_spec(&mut usage, &function.get_spec());
        if let Some(proof) = &function.get_spec().proof {
            use_proof(&mut usage, proof);
        }
    }
    for (_, decl) in module_env.get_lemmas() {
        for parameter in &decl.params {
            parameter.1.module_usage(&mut usage);
        }
        for condition in &decl.conditions {
            use_exp(&mut usage, &condition.exp);
        }
        if let Some(proof) = &decl.proof {
            use_proof(&mut usage, proof);
        }
    }
    for struct_env in module_env.get_structs() {
        for field in struct_env.get_fields().chain(struct_env.get_ghost_fields()) {
            field.get_type().module_usage(&mut usage);
        }
        use_spec(&mut usage, &struct_env.get_spec());
    }
    for (_, decl) in module_env.get_spec_funs() {
        for parameter in &decl.params {
            parameter.1.module_usage(&mut usage);
        }
        decl.result_type.module_usage(&mut usage);
        if let Some(body) = &decl.body {
            use_exp(&mut usage, body);
        }
    }
    for (_, decl) in module_env.get_spec_vars() {
        decl.type_.module_usage(&mut usage);
    }
    usage.remove(&module);
    usage
}

/// The modules of the lemmas a proof applies.
fn applied_lemma_modules(proof: &Proof, usage: &mut BTreeSet<ModuleId>) {
    match proof {
        Proof::Apply(_, lemma, _) | Proof::ForallApply(_, _, _, lemma, _, _) => {
            usage.insert(lemma.module_id);
        },
        Proof::IfElse(_, _, then_proof, else_proof) => {
            applied_lemma_modules(then_proof, usage);
            if let Some(proof) = else_proof {
                applied_lemma_modules(proof, usage);
            }
        },
        Proof::Block(_, proofs) => {
            for proof in proofs {
                applied_lemma_modules(proof, usage);
            }
        },
        Proof::Post(_, proof) => applied_lemma_modules(proof, usage),
        Proof::Let(..)
        | Proof::Assert(..)
        | Proof::Assume(..)
        | Proof::Calc(..)
        | Proof::Split(..) => {},
    }
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
