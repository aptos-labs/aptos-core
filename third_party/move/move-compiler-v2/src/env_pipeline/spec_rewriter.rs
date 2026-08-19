// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! The spec rewriter runs on the whole model after inlining and after check for pureness
//! and does the following:
//!
//! - For every transitively used Move function in specs, it derives
//!   a spec function version of it.
//! - It rewrites all specification expressions to call the derived spec
//!   function instead of the Move function.
//! - It also rewrites expression to replace Move constructs with spec
//!   constructs where possible. This includes replacing references
//!   with values. This transformation assumes that expressions
//!   are already checked for pureness.
//! - For all spec functions (including the derived ones) it computes
//!   transitive memory usage and callee functions.
//! - It checks that data invariants do not depend on memory, and flags
//!   errors if not. This can only be done after transitive memory
//!   usage is known.
//! - It collects all global invariants and attaches them, together
//!   with their memory usage, to the model.

use crate::env_pipeline::rewrite_target::{
    RewriteState, RewriteTarget, RewriteTargets, RewritingScope,
};
use itertools::Itertools;
use log::debug;
use move_model::{
    ast::{
        ConditionKind, Exp, ExpData, FrameSpec, FunParamAccessOf, GlobalInvariant, MemoryRange,
        Operation, SpecBlockTarget, SpecFunDecl, VisitorPosition,
    },
    exp_rewriter::ExpRewriterFunctions,
    model::{
        FunId, FunctionData, FunctionEnv, GlobalEnv, Loc, ModuleId, NodeId, Parameter, QualifiedId,
        QualifiedInstId, SpecFunId, StructEnv, StructId,
    },
    pureness_checker::{FunctionPurenessChecker, FunctionPurenessCheckerMode},
    symbol::Symbol,
    ty::{ReferenceKind, Type},
    well_known,
};
use petgraph::prelude::DiGraphMap;
use std::{
    cell::RefCell,
    collections::{BTreeMap, BTreeSet},
};

pub fn run_spec_rewriter(env: &mut GlobalEnv) {
    debug!("rewriting specifications");

    // Collect all spec blocks and spec functions in the whole program, plus
    // functions in compilation scope. For the later we need to process
    // inline spec blocks.
    // TODO: we may want to optimize this to only rewrite specs involved in
    //   a verification problem, but we need to have a precise definition
    //   what this entails. For example, pre/post conditions need to be present
    //   only if the function spec is marked as opaque.
    let mut targets = RewriteTargets::create(env, RewritingScope::Everything);
    targets.filter(|target, _| match target {
        RewriteTarget::MoveFun(fid) => {
            let fun = env.get_function(*fid);
            fun.module_env.is_target() && !fun.is_inline() && !fun.is_native()
        },
        RewriteTarget::SpecFun(fid) => {
            let fun = env.get_spec_fun(*fid);
            !fun.is_native
        },
        RewriteTarget::SpecBlock(_) => true,
    });

    // Identify the Move functions transitively called by those targets. They need to be
    // converted to spec functions.
    let mut called_funs = BTreeSet::new();
    for target in targets.keys() {
        let callees: BTreeSet<_> = match target {
            RewriteTarget::MoveFun(_) => {
                if let RewriteState::Def(def) = target.get_env_state(env) {
                    let mut spec_callees = BTreeSet::new();
                    def.visit_inline_specs(&mut |spec| {
                        spec_callees.extend(spec.used_funs_with_uses().into_keys());
                        true // keep going
                    });
                    spec_callees
                } else {
                    BTreeSet::new()
                }
            },
            RewriteTarget::SpecFun(_) | RewriteTarget::SpecBlock(_) => {
                target.used_funs_with_uses(env).into_keys().collect()
            },
        };
        for callee in callees {
            called_funs.insert(callee);
            let mut transitive = env
                .get_function(callee)
                .get_transitive_closure_of_called_functions();
            called_funs.append(&mut transitive);
        }
    }

    // When running from the prover, also derive spec functions for all pure
    // Move functions (no &mut params, no acquired resources). This allows the
    // spec inference engine to reference them directly (via SpecFunction)
    // instead of using result_of behavioral predicates.
    if env.get_extension::<crate::Options>().is_some_and(|opts| {
        opts.experiment_on(crate::experiments::Experiment::SPEC_REWRITE_PURE_FUNS)
    }) {
        for module in env.get_modules() {
            for fun in module.get_functions() {
                let qid = fun.get_qualified_id();
                if called_funs.contains(&qid)
                    || fun.is_inline()
                    || fun.is_native()
                    || fun.is_lemma()
                    || fun.get_def().is_none()
                {
                    continue;
                }
                if fun
                    .get_parameters()
                    .iter()
                    .any(|p| p.1.is_mutable_reference())
                {
                    continue;
                }
                if !matches!(fun.get_acquired_structs(), Some(s) if s.is_empty()) {
                    continue;
                }
                called_funs.insert(qid);
                // Include transitive used functions (not just called) so that
                // lambda-lifted closure functions are also mapped.
                let mut transitive = fun.get_transitive_closure_of_used_functions();
                called_funs.append(&mut transitive);
            }
        }
    }

    // For compatibility reasons with the v1 way how to compile spec
    // blocks of inline functions, we also need to add all 'lambda'
    // lifted functions.

    // Derive spec functions for all called Move functions,
    // building a mapping between function ids. Also add
    // those new spec functions to `targets` for subsequent
    // processing. A companion derived by the pre-inlining stage
    // (`run_pure_fun_companion_derivation`) is reused — its body is already
    // converted, so it is not added to the conversion targets.
    let mut function_mapping = BTreeMap::new();
    for fun_id in called_funs {
        let existing = env
            .get_function(fun_id)
            .find_spec_fun()
            .map(|(id, _)| fun_id.module_id.qualified(id));
        let spec_fun_id = match existing {
            Some(id) => id,
            None => {
                let id = derive_spec_fun(env, fun_id, false);
                // Add new spec fun to targets for later processing
                targets.entry(RewriteTarget::SpecFun(id));
                id
            },
        };
        function_mapping.insert(fun_id, spec_fun_id);
        // Mark spec fun to be used in environment
        env.add_used_spec_fun(spec_fun_id)
    }

    // Based on the mapping above, now visit all targets and convert them.
    for target in targets.keys().collect_vec() {
        use RewriteState::*;
        use RewriteTarget::*;
        let get_param_names =
            |params: &[Parameter]| params.iter().map(|Parameter(name, ..)| *name).collect_vec();
        match (&target, target.get_env_state(env)) {
            (MoveFun(_fun_id), Def(exp)) => {
                let mut converter = SpecConverter::new(env, &function_mapping, false);
                let new_exp = converter.rewrite_exp(exp.clone());
                if !ExpData::ptr_eq(&new_exp, &exp) {
                    *targets.state_mut(&target) = Def(new_exp)
                }
            },
            (SpecFun(id), Def(exp)) => {
                let paras = get_param_names(&env.get_spec_fun(*id).params);
                let mut converter =
                    SpecConverter::new(env, &function_mapping, true).symbolized_parameters(paras);
                let new_exp = converter.rewrite_exp(exp.clone());
                if !ExpData::ptr_eq(&new_exp, &exp) {
                    *targets.state_mut(&target) = Def(new_exp)
                }
            },
            (SpecBlock(sb_target), Spec(spec)) => {
                let mut converter = match sb_target {
                    SpecBlockTarget::SpecFunction(mid, spec_fun_id) => {
                        // When the spec block is associated with a spec function,
                        // replace temps with parameter names
                        let paras =
                            get_param_names(&env.get_spec_fun(mid.qualified(*spec_fun_id)).params);
                        SpecConverter::new(env, &function_mapping, true)
                            .symbolized_parameters(paras)
                    },
                    _ => SpecConverter::new(env, &function_mapping, true),
                };
                let (changed, new_spec) = converter.rewrite_spec_descent(sb_target, &spec);
                if changed {
                    *targets.state_mut(&target) = Spec(new_spec)
                }
            },
            _ => {},
        }
    }
    targets.write_to_env(env);

    // Now that all functions are defined, compute transitive callee and used memory,
    // as well as `uses_old` and `old_memory` for dual-state spec funs.
    // Since specification functions can be recursive we compute the strongly-connected
    // components first and then process each in bottom-up order.
    let mut graph: DiGraphMap<QualifiedId<SpecFunId>, ()> = DiGraphMap::new();
    let spec_funs = env
        .get_modules()
        .flat_map(|m| {
            m.get_spec_funs()
                .map(|(id, _)| m.get_id().qualified(*id))
                .collect_vec()
        })
        .collect_vec();
    for qid in spec_funs {
        graph.add_node(qid);
        let decl = env.get_spec_fun(qid);
        let has_mut_params = decl
            .params
            .iter()
            .any(|Parameter(_, ty, _)| ty.is_mutable_reference());
        let (initial_callees, initial_usage, direct_uses_old, direct_old_memory) =
            if let Some(def) = &decl.body {
                let callees = def.called_spec_funs(env);
                for callee in &callees {
                    graph.add_edge(qid, callee.to_qualified_id(), ());
                }
                let usage = def.directly_used_memory(env);
                // Detect direct old() usage and collect old_memory
                let (direct_uses_old, direct_old_memory) = compute_direct_old_usage(def, env);
                (callees, usage, direct_uses_old, direct_old_memory)
            } else {
                Default::default()
            };

        // If user-declared modifies/reads exist, derive memory from them
        let has_user_decl = decl
            .frame_spec
            .as_ref()
            .is_some_and(|fs| !fs.modifies_targets.is_empty() || !fs.reads_targets.is_empty());
        let (final_usage, final_uses_old, final_old_memory) = if has_user_decl {
            let frame = decl.frame_spec.as_ref().unwrap();
            let mut spec_usage = BTreeSet::new();
            let mut spec_old_memory = BTreeSet::new();
            // modifies targets → used_memory + old_memory
            for target in &frame.modifies_targets {
                if let ExpData::Call(id, Operation::Global(_), _) = target.as_ref() {
                    let ty = env.get_node_type(*id);
                    let ty = ty.skip_reference();
                    if let Type::Struct(mid, sid, inst) = ty {
                        let qid = mid.qualified_inst(*sid, inst.clone());
                        spec_usage.insert(qid.clone());
                        spec_old_memory.insert(qid);
                    }
                }
            }
            // reads targets → used_memory only (already resolved to struct IDs)
            for qid in &frame.reads_targets {
                spec_usage.insert(qid.clone());
            }
            // Also include old() usage detected from the body itself (e.g., a spec
            // fun with `reads R` that uses `old(R[a])` in its body).
            spec_old_memory.extend(direct_old_memory.iter().cloned());
            let spec_uses_old = !spec_old_memory.is_empty() || direct_uses_old || has_mut_params;

            // For funs with body, validate that body-derived memory is covered
            if decl.body.is_some() {
                for mem in &initial_usage {
                    if !spec_usage.contains(mem) {
                        env.error(
                            &decl.loc,
                            &format!(
                                "spec fun body accesses `{}` which is not covered by \
                                 its modifies/reads declaration",
                                env.display(mem)
                            ),
                        );
                    }
                }
            }
            (spec_usage, spec_uses_old, spec_old_memory)
        } else {
            (
                initial_usage,
                direct_uses_old || has_mut_params,
                direct_old_memory,
            )
        };

        let decl_mut = env.get_spec_fun_mut(qid);
        decl_mut.callees = initial_callees;
        decl_mut.used_memory = final_usage;
        decl_mut.uses_old = final_uses_old;
        decl_mut.old_memory = final_old_memory;
    }
    for scc in petgraph::algo::kosaraju_scc(&graph) {
        // Within each cycle, the transitive usage is the union of the transitive
        // usage of each member.
        let mut transitive_callees = BTreeSet::new();
        let mut transitive_usage = BTreeSet::new();
        let mut transitive_uses_old = false;
        let mut transitive_old_memory = BTreeSet::new();
        for qid in &scc {
            let decl = env.get_spec_fun(*qid);
            // Add direct usage.
            transitive_callees.extend(decl.callees.iter().cloned());
            transitive_usage.extend(decl.used_memory.iter().cloned());
            if decl.uses_old {
                transitive_uses_old = true;
            }
            transitive_old_memory.extend(decl.old_memory.iter().cloned());
            // Add indirect usage
            for callee in &decl.callees {
                let callee_decl = env.get_spec_fun(callee.to_qualified_id());
                transitive_callees.extend(
                    callee_decl
                        .callees
                        .iter()
                        .map(|id| id.clone().instantiate(&callee.inst)),
                );
                transitive_usage.extend(
                    callee_decl
                        .used_memory
                        .iter()
                        .map(|mem| mem.clone().instantiate(&callee.inst)),
                );
                if callee_decl.uses_old {
                    transitive_uses_old = true;
                }
                transitive_old_memory.extend(
                    callee_decl
                        .old_memory
                        .iter()
                        .map(|mem| mem.clone().instantiate(&callee.inst)),
                );
            }
        }
        // Store result back
        for qid in scc {
            let decl_mut = env.get_spec_fun_mut(qid);
            decl_mut.callees.clone_from(&transitive_callees);
            decl_mut.used_memory.clone_from(&transitive_usage);
            decl_mut.uses_old = decl_mut.uses_old || transitive_uses_old;
            decl_mut.old_memory.clone_from(&transitive_old_memory);
        }
    }

    // Compute spec memory for behavioral predicates.
    // For each function, derive (used_memory, old_memory) from its spec conditions.
    // Also populate access_of entries with derived memory.
    compute_behavioral_predicate_memory(env);

    // Validate that closures passed to functions with access_of respect the limits.
    validate_closure_access_of_compliance(env);

    // Last, process invariants
    for module in env.get_modules() {
        if module.is_target() {
            for str in module.get_structs() {
                check_data_invariants(&str)
            }
        }
    }
    collect_global_invariants_to_env(env)
}

/// Entry point to generate spec functions from lambda expressions to be expanded during inlining
pub fn run_spec_rewriter_inline(
    env: &mut GlobalEnv,
    module_id: ModuleId,
    lambda_fun_map: BTreeMap<usize, FunctionData>,
) -> BTreeMap<usize, (QualifiedId<SpecFunId>, QualifiedId<FunId>)> {
    debug!("rewriting specifications in inline functions");

    let mut targets = RewriteTargets::create_fun_targets(env, vec![]);

    // Traverse lifted lambda functions,
    // generate corresponding spec fun and ad them as rewrite target
    let mut function_data_mapping = BTreeMap::new();
    for (i, data) in lambda_fun_map {
        // Add lifted lambda expressions into env
        let fun_id = env.add_function_def_from_data(module_id, data);
        let qualified_fun_id = module_id.qualified(fun_id);
        let spec_fun_id = derive_spec_fun(env, qualified_fun_id, true);
        function_data_mapping.insert(i, (spec_fun_id, qualified_fun_id));
        // Add new spec fun to targets for later processing
        targets.entry(RewriteTarget::SpecFun(spec_fun_id));
        // For simplicity, we assume that all lambda parameters will be called
        env.add_used_spec_fun(spec_fun_id);
    }
    let function_mapping = BTreeMap::new();

    for target in targets.keys().collect_vec() {
        use RewriteState::*;
        use RewriteTarget::*;
        let get_param_names =
            |params: &[Parameter]| params.iter().map(|Parameter(name, ..)| *name).collect_vec();
        if let (SpecFun(id), Def(exp)) = (&target, target.get_env_state(env)) {
            let paras: Vec<Symbol> = get_param_names(&env.get_spec_fun(*id).params);
            let mut converter = SpecConverter::new_for_inline(env, &function_mapping, true)
                .symbolized_parameters(paras);
            let new_exp = converter.rewrite_exp(exp.clone());
            // If the spec function contains imperative expressions, set it to uninterpreted
            if converter.contains_imperative_expression {
                let spec_fun = converter.env.get_spec_fun_mut(*id);
                spec_fun.uninterpreted = true;
                spec_fun.body = None;
            } else if !ExpData::ptr_eq(&new_exp, &exp) {
                *targets.state_mut(&target) = Def(new_exp)
            }
        }
    }
    targets.write_to_env(env);
    function_data_mapping
}

// -------------------------------------------------------------------------------------------
// Pre-Inlining Companion Derivation for Pure Functions

/// Entry point for the pre-inlining companion-derivation stage (verify mode
/// only, registered before the inliner): derives `$fun` companion spec
/// functions for *hereditarily pure* Move functions, so the source-level
/// specification derivation for lambda bodies
/// (`move_model::spec_derivation`) can resolve calls to such functions as
/// direct spec-function calls (`try_as_pure_spec_call`) while inline
/// functions are expanded — instead of routing them through generic
/// behavioral summaries, which clear the exact modifies footprint and
/// introduce state labels. Without this stage, companions exist only after
/// [`run_spec_rewriter`], which runs after inlining.
///
/// A function qualifies when it mirrors the criteria of
/// `try_as_pure_spec_call` and the pure-function derivation in
/// [`run_spec_rewriter`]: not inline/native/lemma, has a body, takes no
/// `&mut` and no function-typed parameters, is pure in specification mode
/// (`FunctionPurenessChecker`), and its body — outside of spec blocks,
/// which the conversion drops — is free of global-memory operations and of
/// function values; and every function it (transitively) calls qualifies
/// likewise. Callees must qualify so the companion bodies can be fully
/// converted here (in particular, no call may reach an inline function,
/// whose expansion would change the body after this stage). Native callees
/// are permitted only for the `std::vector` bytecode-instruction set and
/// `std::signer::borrow_address`, whose `$`-spec versions the Boogie
/// prelude defines.
///
/// The derived companions are *not* marked used here: the consumers mark a
/// companion transitively used when they create a reference to it
/// (`GlobalEnv::add_used_spec_fun_transitive`), so unused companions are
/// not emitted by the backend. [`run_spec_rewriter`] later reuses these
/// companions instead of deriving duplicates.
pub fn run_pure_fun_companion_derivation(env: &mut GlobalEnv) {
    if !env.is_verify_mode() {
        return;
    }
    debug!("deriving companion spec functions for pure functions");

    // Phase 1: per-function local qualification and call edges.
    let all_funs = env
        .get_modules()
        .flat_map(|module| {
            module
                .get_functions()
                .map(|fun| fun.get_qualified_id())
                .collect_vec()
        })
        .collect_vec();
    let mut locals: BTreeMap<QualifiedId<FunId>, CompanionLocalInfo> = BTreeMap::new();
    for qid in all_funs {
        let mut info = companion_local_qualification(env, &env.get_function(qid));
        if info.ok {
            // The definitive convertibility test: dry-run the spec
            // conversion and reject bodies it judges imperative (e.g.
            // loops, or statement sequences it cannot reduce) — such a
            // companion could not serve as a pure spec call. The inline
            // variant leaves Move function calls in place, so no callee
            // mapping is needed.
            let def = env
                .get_function(qid)
                .get_def()
                .expect("definition exists for locally qualified function")
                .clone();
            let empty_mapping = BTreeMap::new();
            let mut converter = SpecConverter::new_for_inline(env, &empty_mapping, true);
            converter.rewrite_exp(def);
            if converter.contains_imperative_expression {
                info.ok = false;
            }
        }
        locals.insert(qid, info);
    }

    // Phase 2: greatest fixpoint over the call edges — a function stays
    // qualified only while all its callees are qualified or companion-safe
    // natives (recursion cycles among qualified functions stay in).
    let mut qualified: BTreeSet<QualifiedId<FunId>> = locals
        .iter()
        .filter(|(_, info)| info.ok)
        .map(|(qid, _)| *qid)
        .collect();
    loop {
        let to_remove = qualified
            .iter()
            .filter(|qid| {
                !locals[*qid].callees.iter().all(|callee| {
                    qualified.contains(callee)
                        || is_companion_safe_native(&env.get_function(*callee))
                })
            })
            .copied()
            .collect_vec();
        if to_remove.is_empty() {
            break;
        }
        for qid in to_remove {
            qualified.remove(&qid);
        }
    }

    // The members needing companions: the qualified functions plus the
    // companion-safe natives they reference.
    let mut members = qualified.clone();
    for qid in &qualified {
        for callee in &locals[qid].callees {
            if !qualified.contains(callee) {
                members.insert(*callee);
            }
        }
    }

    // Phase 3: derive the companions and convert their bodies. All callees
    // of a member are members, so the mapping is closed and the conversion
    // cannot encounter an unmapped call.
    let mut function_mapping = BTreeMap::new();
    for qid in &members {
        debug_assert!(
            env.get_function(*qid).find_spec_fun().is_none(),
            "companion derived twice"
        );
        let spec_fun_id = derive_spec_fun(env, *qid, false);
        function_mapping.insert(*qid, spec_fun_id);
    }
    for spec_fun_id in function_mapping.values() {
        let decl = env.get_spec_fun(*spec_fun_id);
        if decl.is_native {
            continue;
        }
        let body = decl.body.clone().expect("companion body");
        let params = decl
            .params
            .iter()
            .map(|Parameter(name, ..)| *name)
            .collect_vec();
        let mut converter =
            SpecConverter::new(env, &function_mapping, true).symbolized_parameters(params);
        let new_body = converter.rewrite_exp(body);
        if converter.contains_imperative_expression {
            // Defensive: qualification excludes imperative bodies; should
            // this trigger nevertheless, the companion is unusable as a
            // spec call (`try_as_pure_spec_call` rejects bodiless
            // non-native companions).
            let decl = env.get_spec_fun_mut(*spec_fun_id);
            decl.uninterpreted = true;
            decl.body = None;
        } else {
            env.get_spec_fun_mut(*spec_fun_id).body = Some(new_body);
        }
    }
}

/// The local (per-function) companion qualification result; see
/// [`companion_local_qualification`].
struct CompanionLocalInfo {
    ok: bool,
    callees: BTreeSet<QualifiedId<FunId>>,
}

/// The per-function part of the companion qualification: signature
/// criteria, specification-mode pureness, and the body scan (memory
/// operations, function values), collecting the called functions for the
/// closure fixpoint. Content of spec blocks is included: although the
/// conversion reduces the blocks themselves to unit, it still rewrites
/// Move function calls within them, so they participate in the mapping
/// closure (and memory operations there disqualify conservatively).
fn companion_local_qualification(env: &GlobalEnv, fun: &FunctionEnv) -> CompanionLocalInfo {
    let not_ok = || CompanionLocalInfo {
        ok: false,
        callees: BTreeSet::new(),
    };
    if fun.is_inline() || fun.is_native() || fun.is_lemma() {
        return not_ok();
    }
    let Some(def) = fun.get_def() else {
        return not_ok();
    };
    if fun.get_parameters().iter().any(|Parameter(_, ty, _)| {
        ty.is_mutable_reference() || matches!(ty.skip_reference(), Type::Fun(..))
    }) {
        return not_ok();
    }
    // Specification-mode pureness, mirroring `try_as_pure_spec_call`. The
    // checker recurses into callees itself.
    let mut is_pure = true;
    let mut checker =
        FunctionPurenessChecker::new(FunctionPurenessCheckerMode::Specification, |_, _, _| {
            is_pure = false;
        });
    checker.check_exp(env, def);
    if !is_pure {
        return not_ok();
    }
    // Scan the body (including spec blocks) for memory operations and
    // function values, collecting call edges.
    let mut ok = true;
    let mut callees = BTreeSet::new();
    def.visit_pre_order(&mut |e| {
        match e {
            ExpData::Lambda(..) | ExpData::Invoke(..) => ok = false,
            ExpData::Call(_, oper, _) => match oper {
                Operation::Global(_)
                | Operation::Exists(_)
                | Operation::BorrowGlobal(_)
                | Operation::MoveTo
                | Operation::MoveFrom
                | Operation::SpecPublish(_)
                | Operation::SpecRemove(_)
                | Operation::SpecUpdate(_)
                | Operation::Closure(..) => ok = false,
                Operation::MoveFunction(mid, fid) => {
                    callees.insert(mid.qualified(*fid));
                },
                _ => {},
            },
            _ => {},
        }
        ok
    });
    CompanionLocalInfo { ok, callees }
}

/// Whether the given native function may appear as a callee of a
/// companion-derived pure function: its `$`-spec version is defined in the
/// Boogie prelude (`native.bpl` for the `std::vector` bytecode-instruction
/// natives, `prelude.bpl` for `$1_signer_$borrow_address`).
fn is_companion_safe_native(fun: &FunctionEnv) -> bool {
    if !fun.is_native() {
        return false;
    }
    let name = fun.get_name_str();
    (fun.module_env.is_std_vector()
        && well_known::VECTOR_FUNCS_WITH_BYTECODE_INSTRS.contains(&name.as_str()))
        || (fun.module_env.is_module_in_std("signer") && name == "borrow_address")
}

// -------------------------------------------------------------------------------------------
// Deriving Specification Functions

// Derive a specification function from a Move function. Initially the body is the
// original one, not yet converted to the specification representation.
fn derive_spec_fun(
    env: &mut GlobalEnv,
    fun_id: QualifiedId<FunId>,
    for_inline: bool,
) -> QualifiedId<SpecFunId> {
    let fun = env.get_function(fun_id);
    let (is_native, body) = if fun.is_native() {
        (true, None)
    } else {
        let exp = fun.get_def().expect("function body").clone();
        (false, Some(exp))
    };

    // For historical reasons, those names are prefixed with `$` even though there
    // is no name clash allowed.
    let inline_prefix = if for_inline { "inline_" } else { "" };
    let name = env.symbol_pool().make(&format!(
        "${}{}",
        inline_prefix,
        fun.get_name().display(env.symbol_pool())
    ));
    // Eliminate references in parameters and result type
    let params = fun
        .get_parameters()
        .into_iter()
        .map(|Parameter(sym, ty, loc)| Parameter(sym, ty.skip_reference().clone(), loc))
        .collect();
    let result_type = fun.get_result_type().skip_reference().clone();

    // Attach the spec block when generated during inlining (lambda with imperative body).
    // This allows axiom generation for uninterpreted spec functions.
    let spec = if for_inline {
        fun.get_spec().clone()
    } else {
        Default::default()
    };
    let decl = SpecFunDecl {
        loc: fun.get_loc(),
        name,
        type_params: fun.get_type_parameters(),
        params,
        result_type,
        used_memory: BTreeSet::new(),
        old_memory: BTreeSet::new(),
        uninterpreted: false,
        is_move_fun: true,
        is_native,
        body,
        callees: BTreeSet::new(),
        is_recursive: RefCell::new(None),
        uses_old: false,
        frame_spec: None,
        insts_using_generic_type_reflection: Default::default(),
        spec: RefCell::new(spec),
    };
    env.add_spec_function_def(fun_id.module_id, decl)
}

/// Computes direct `old()` usage for a spec fun body. Returns (uses_old, old_memory)
/// where `uses_old` is true if the body contains `Operation::Old`, and `old_memory`
/// is the set of resources accessed inside `old()` contexts.
pub fn compute_direct_old_usage(
    body: &Exp,
    env: &GlobalEnv,
) -> (bool, BTreeSet<QualifiedInstId<StructId>>) {
    let mut uses_old = false;
    let mut old_memory = BTreeSet::new();
    let mut in_old_depth: usize = 0;
    body.visit_positions(&mut |pos, exp| {
        match exp {
            ExpData::Call(_, Operation::Old, _) => match pos {
                VisitorPosition::Pre => {
                    uses_old = true;
                    in_old_depth += 1;
                },
                VisitorPosition::Post => {
                    in_old_depth -= 1;
                },
                _ => {},
            },
            ExpData::Call(id, Operation::Global(_), _)
            | ExpData::Call(id, Operation::Exists(_), _)
                if in_old_depth > 0 && matches!(pos, VisitorPosition::Pre) =>
            {
                let inst = &env.get_node_instantiation(*id);
                let (mid, sid, sinst) = inst[0].require_struct();
                old_memory.insert(mid.qualified_inst(sid, sinst.to_owned()));
            },
            // Mutation builtins are inherently two-state: they transition from
            // pre-state to post-state. Their resource type is in old_memory.
            ExpData::Call(
                id,
                Operation::SpecPublish(_) | Operation::SpecRemove(_) | Operation::SpecUpdate(_),
                _,
            ) if matches!(pos, VisitorPosition::Pre) => {
                uses_old = true;
                let inst = &env.get_node_instantiation(*id);
                let (mid, sid, sinst) = inst[0].require_struct();
                old_memory.insert(mid.qualified_inst(sid, sinst.to_owned()));
            },
            _ => {},
        }
        true
    });
    (uses_old, old_memory)
}

/// Derive used_memory and old_memory from a `FunParamAccessOf` entry.
/// modifies_targets → used_memory + old_memory (writes implies dual-state).
/// reads_types → used_memory only.
pub fn derive_memory_from_access_of(
    env: &GlobalEnv,
    access: &FunParamAccessOf,
) -> (
    BTreeSet<QualifiedInstId<StructId>>,
    BTreeSet<QualifiedInstId<StructId>>,
) {
    let mut used_memory = BTreeSet::new();
    let mut old_memory = BTreeSet::new();
    // From modifies targets: extract resource struct ID from Operation::Global expressions
    for target in &access.frame_spec.modifies_targets {
        if let ExpData::Call(id, Operation::Global(_), _) = target.as_ref() {
            let ty = env.get_node_type(*id);
            // The node type may be &T (reference) from borrow_global→Global conversion;
            // strip the reference to get the struct type.
            let ty = ty.skip_reference();
            if let Type::Struct(mid, sid, inst) = ty {
                let qid = mid.qualified_inst(*sid, inst.clone());
                used_memory.insert(qid.clone());
                old_memory.insert(qid);
            }
        }
    }
    // From reads types
    for qid in &access.frame_spec.reads_targets {
        used_memory.insert(qid.clone());
    }
    (used_memory, old_memory)
}

/// Computes and stores memory footprints for behavioral predicates.
/// - For each `access_of` entry on functions: derives `used_memory`/`old_memory` from specifiers
/// - For each function with spec conditions: derives `spec_used_memory`/`spec_old_memory` from conditions
fn compute_behavioral_predicate_memory(env: &mut GlobalEnv) {
    // Collect all function IDs first to avoid borrow issues
    let fun_ids: Vec<QualifiedId<FunId>> = env
        .get_modules()
        .flat_map(|m| {
            m.get_functions()
                .map(|f| f.get_qualified_id())
                .collect_vec()
        })
        .collect_vec();

    for fun_id in fun_ids {
        // Collect memory info while borrowing env immutably via fun_env
        let (param_updates, spec_used, spec_old, spec_uses_old) = {
            let fun_env = env.get_function(fun_id);
            let mut param_updates: Vec<(
                usize,
                BTreeSet<QualifiedInstId<StructId>>,
                BTreeSet<QualifiedInstId<StructId>>,
            )> = vec![];
            for (i, access) in fun_env.get_fun_param_access_of().iter().enumerate() {
                let (used, old) = derive_memory_from_access_of(env, access);
                param_updates.push((i, used, old));
            }

            // Compute spec condition memory
            let spec = fun_env.get_spec();
            let mut spec_used = BTreeSet::new();
            let mut spec_old = BTreeSet::new();
            for cond in &spec.conditions {
                spec_used.extend(cond.exp.directly_used_memory(env));
                for e in &cond.additional_exps {
                    spec_used.extend(e.directly_used_memory(env));
                }
                let (uses_old, old_mem) = compute_direct_old_usage(&cond.exp, env);
                if uses_old {
                    spec_old.extend(old_mem);
                }
                for e in &cond.additional_exps {
                    let (uses_old, old_mem) = compute_direct_old_usage(e, env);
                    if uses_old {
                        spec_old.extend(old_mem);
                    }
                }
            }
            // Include old_memory from spec functions called in conditions.
            // If `ensures helper(a)` and `helper` uses `old()`, the function's
            // spec_old must include helper's old_memory for correct pre-state snapshots.
            for cond in &spec.conditions {
                for exp in std::iter::once(&cond.exp).chain(&cond.additional_exps) {
                    exp.visit_post_order(&mut |e: &ExpData| {
                        if let ExpData::Call(id, Operation::SpecFunction(mid, fid, _), _) = e {
                            let inst = &env.get_node_instantiation(*id);
                            let module = env.get_module(*mid);
                            let sfun = module.get_spec_fun(*fid);
                            for mem in &sfun.old_memory {
                                spec_old.insert(mem.clone().instantiate(inst));
                            }
                        }
                        true
                    });
                }
            }
            // Include modifies_of/reads_of memory in spec_used/spec_old so that SaveMem
            // generates Boogie declarations for behavioral predicate memory.
            for (_, used, old) in &param_updates {
                spec_used.extend(used.iter().cloned());
                spec_old.extend(old.iter().cloned());
            }

            let has_mut_params = fun_env
                .get_parameters()
                .iter()
                .any(|Parameter(_, ty, _)| ty.is_mutable_reference());
            let spec_uses_old = !spec_old.is_empty() || has_mut_params;
            (param_updates, spec_used, spec_old, spec_uses_old)
        };

        // Store results (env is no longer borrowed by fun_env)
        for (i, used, old) in param_updates {
            env.set_fun_param_access_of_memory(fun_id, i, used, old);
        }
        env.set_function_spec_memory(fun_id, spec_used, spec_old, spec_uses_old);
    }

    // Process struct field access declarations
    let struct_ids: Vec<QualifiedId<StructId>> = env
        .get_modules()
        .flat_map(|m| m.get_structs().map(|s| s.get_qualified_id()).collect_vec())
        .collect_vec();
    for struct_id in struct_ids {
        let field_updates: Vec<_> = {
            let struct_env = env.get_struct(struct_id);
            struct_env
                .get_field_access_of()
                .iter()
                .enumerate()
                .map(|(i, access)| {
                    let (used, old) = derive_memory_from_access_of(env, access);
                    (i, used, old)
                })
                .collect()
        };
        for (i, used, old) in field_updates {
            env.set_struct_field_access_of_memory(struct_id, i, used, old);
        }
    }
}

/// Validates that a single function-typed expression complies with a field's access declaration.
fn validate_field_access(
    env: &GlobalEnv,
    loc: &Loc,
    field_name: Symbol,
    field_access: &[FunParamAccessOf],
    arg: &Exp,
    caller_spec_used: &BTreeSet<QualifiedInstId<StructId>>,
    caller_spec_old: &BTreeSet<QualifiedInstId<StructId>>,
    caller_param_access: &[FunParamAccessOf],
    caller_params: &[Parameter],
) {
    let empty_access;
    let access = match field_access.iter().find(|a| a.fun_param == field_name) {
        Some(a) => a,
        None => {
            // No declaration = pure: field function can't access memory
            empty_access = FunParamAccessOf {
                loc: Loc::default(),
                fun_param: field_name,
                modifies_params: vec![],
                frame_spec: FrameSpec::default(),
                used_memory: BTreeSet::new(),
                old_memory: BTreeSet::new(),
            };
            &empty_access
        },
    };
    // modifies_all subsumes everything (read + write)
    if access.frame_spec.modifies_all {
        return;
    }
    let (arg_used, arg_old) = compute_arg_memory(
        env,
        arg,
        caller_spec_used,
        caller_spec_old,
        caller_param_access,
        caller_params,
    );
    // reads_all subsumes only read checks
    if !access.frame_spec.reads_all {
        for mem in &arg_used {
            if !access.used_memory.contains(mem) {
                env.error(
                    loc,
                    &format!(
                        "stored function accesses resource `{}` \
                         which is not declared in `modifies_of`/`reads_of` \
                         for field `{}`",
                        env.display(mem),
                        field_name.display(env.symbol_pool())
                    ),
                );
            }
        }
    }
    for mem in &arg_old {
        if !access.old_memory.contains(mem) {
            if access.used_memory.contains(mem) || access.frame_spec.reads_all {
                env.error(
                    loc,
                    &format!(
                        "stored function writes resource `{}` \
                         but only `reads_of` (not `modifies_of`) is \
                         declared for field `{}`",
                        env.display(mem),
                        field_name.display(env.symbol_pool())
                    ),
                );
            } else {
                env.error(
                    loc,
                    &format!(
                        "stored function accesses resource `{}` \
                         which is not declared in \
                         `modifies_of`/`reads_of` for field `{}`",
                        env.display(mem),
                        field_name.display(env.symbol_pool())
                    ),
                );
            }
        }
    }
}

/// Validates that closures passed to functions with `modifies_of`/`reads_of` declarations
/// respect the declared memory bounds. For each call site where a function-typed
/// argument is passed to a parameter with `modifies_of`/`reads_of`, the argument's memory
/// footprint must be a subset of the declared memory.
fn validate_closure_access_of_compliance(env: &GlobalEnv) {
    let fun_ids: Vec<QualifiedId<FunId>> = env
        .get_modules()
        .filter(|m| m.is_target())
        .flat_map(|m| {
            m.get_functions()
                .filter(|f| !f.is_inline() && !f.is_native())
                .map(|f| f.get_qualified_id())
                .collect_vec()
        })
        .collect_vec();

    for caller_id in fun_ids {
        let (body, caller_spec_used, caller_spec_old, caller_param_access, caller_params) = {
            let caller_env = env.get_function(caller_id);
            let body = match caller_env.get_def() {
                Some(body) => body.clone(),
                None => continue,
            };
            (
                body,
                caller_env.get_spec_used_memory().clone(),
                caller_env.get_spec_old_memory().clone(),
                caller_env.get_fun_param_access_of().to_vec(),
                caller_env.get_parameters(),
            )
        };

        body.visit_post_order(&mut |exp| {
            // Check struct pack operations: when a closure is stored into a struct
            // field with reads_of/modifies_of declarations, validate compliance.
            if let ExpData::Call(call_id, Operation::Pack(mid, sid, variant), args) = exp {
                let struct_qid = mid.qualified(*sid);
                let field_access = {
                    let struct_env = env.get_struct(struct_qid);
                    struct_env.get_field_access_of().to_vec()
                };
                if !field_access.is_empty() {
                    let struct_env = env.get_struct(struct_qid);
                    let fields: Vec<_> =
                        struct_env.get_fields_optional_variant(*variant).collect();
                    let loc = env.get_node_loc(*call_id);
                    for (arg_idx, arg) in args.iter().enumerate() {
                        if arg_idx >= fields.len() {
                            continue;
                        }
                        let field = &fields[arg_idx];
                        if !field.get_type().is_function() {
                            continue;
                        }
                        validate_field_access(
                            env,
                            &loc,
                            field.get_name(),
                            &field_access,
                            arg,
                            &caller_spec_used,
                            &caller_spec_old,
                            &caller_param_access,
                            &caller_params,
                        );
                    }
                }
            }
            // Check field mutation: when a function-valued field is overwritten,
            // validate that the new value complies with the field's access declarations.
            if let ExpData::Mutate(mutate_id, lhs, rhs) = exp {
                let (mid, sid, fid) = match lhs.as_ref() {
                    ExpData::Call(_, Operation::Select(mid, sid, fid), _) => {
                        (mid, sid, fid)
                    },
                    _ => return true,
                };
                let struct_qid = mid.qualified(*sid);
                let (field_access, field_name, is_fn) = {
                    let struct_env = env.get_struct(struct_qid);
                    let access = struct_env.get_field_access_of().to_vec();
                    let field = struct_env.get_field(*fid);
                    (access, field.get_name(), field.get_type().is_function())
                };
                if is_fn {
                    let loc = env.get_node_loc(*mutate_id);
                    validate_field_access(
                        env,
                        &loc,
                        field_name,
                        &field_access,
                        rhs,
                        &caller_spec_used,
                        &caller_spec_old,
                        &caller_param_access,
                        &caller_params,
                    );
                }
            }
            if let ExpData::Call(call_id, Operation::MoveFunction(mid, fid), args) = exp {
                let callee_id = mid.qualified(*fid);
                let (callee_param_access, callee_params) = {
                    let callee_env = env.get_function(callee_id);
                    let access = callee_env.get_fun_param_access_of().to_vec();
                    // Skip validation only for transparent callees with no declarations.
                    // For opaque callees, missing declarations means "pure" — validate
                    // that closures don't access memory.
                    if access.is_empty() && !callee_env.is_opaque() {
                        return true;
                    }
                    (access, callee_env.get_parameters())
                };

                // For each argument, check if it maps to a parameter with access_of
                for (arg_idx, arg) in args.iter().enumerate() {
                    if arg_idx >= callee_params.len() {
                        continue;
                    }
                    let Parameter(param_name, param_ty, _) = &callee_params[arg_idx];
                    if !param_ty.is_function() {
                        continue;
                    }

                    // Find the access_of entry for this parameter.
                    // Missing declaration is treated as empty (pure): the parameter
                    // is not allowed to access any memory.
                    let empty_access;
                    let access = match callee_param_access
                        .iter()
                        .find(|a| a.fun_param == *param_name)
                    {
                        Some(a) => a,
                        None => {
                            empty_access = FunParamAccessOf {
                                loc: Loc::default(),
                                fun_param: *param_name,
                                modifies_params: vec![],
                                frame_spec: FrameSpec::default(),
                                used_memory: BTreeSet::new(),
                                old_memory: BTreeSet::new(),
                            };
                            &empty_access
                        },
                    };

                    // modifies_all subsumes everything (read + write)
                    if access.frame_spec.modifies_all {
                        continue;
                    }

                    // Compute the argument's memory footprint
                    let (arg_used, arg_old) = compute_arg_memory(
                        env,
                        arg,
                        &caller_spec_used,
                        &caller_spec_old,
                        &caller_param_access,
                        &caller_params,
                    );

                    // Check arg_used ⊆ access.used_memory (reads_all subsumes this)
                    if !access.frame_spec.reads_all {
                        for mem in &arg_used {
                            if !access.used_memory.contains(mem) {
                                let call_loc = env.get_node_loc(*call_id);
                                env.error(
                                    &call_loc,
                                    &format!(
                                        "function argument accesses resource `{}` \
                                         which is not declared in `modifies_of`/`reads_of` for `{}`",
                                        env.display(mem),
                                        param_name.display(env.symbol_pool())
                                    ),
                                );
                            }
                        }
                    }

                    // Check arg_old ⊆ access.old_memory
                    for mem in &arg_old {
                        if !access.old_memory.contains(mem) {
                            let call_loc = env.get_node_loc(*call_id);
                            if access.used_memory.contains(mem)
                                || access.frame_spec.reads_all
                            {
                                env.error(
                                    &call_loc,
                                    &format!(
                                        "function argument writes resource `{}` \
                                         but only `reads_of` (not `modifies_of`) is declared for `{}`",
                                        env.display(mem),
                                        param_name.display(env.symbol_pool())
                                    ),
                                );
                            } else {
                                env.error(
                                    &call_loc,
                                    &format!(
                                        "function argument accesses resource `{}` \
                                         which is not declared in `modifies_of`/`reads_of` for `{}`",
                                        env.display(mem),
                                        param_name.display(env.symbol_pool())
                                    ),
                                );
                            }
                        }
                    }
                }
            }
            true
        });
    }
}

/// Computes the memory footprint of a function-typed argument expression.
/// Returns (used_memory, old_memory).
fn compute_arg_memory(
    env: &GlobalEnv,
    arg: &Exp,
    caller_spec_used: &BTreeSet<QualifiedInstId<StructId>>,
    caller_spec_old: &BTreeSet<QualifiedInstId<StructId>>,
    caller_param_access: &[FunParamAccessOf],
    caller_params: &[Parameter],
) -> (
    BTreeSet<QualifiedInstId<StructId>>,
    BTreeSet<QualifiedInstId<StructId>>,
) {
    match arg.as_ref() {
        // Direct closure: use the closure target's spec memory
        ExpData::Call(_, Operation::Closure(mid, fid, _), _) => {
            let target_id = mid.qualified(*fid);
            let target_env = env.get_function(target_id);
            let used = target_env.get_spec_used_memory().clone();
            let old = target_env.get_spec_old_memory().clone();
            (used, old)
        },
        // Lambda with inline spec: compute memory from the lambda body + spec
        ExpData::Lambda(_, _, body, _, spec_opt) => {
            let mut used = body.directly_used_memory(env);
            let mut old = BTreeSet::new();
            if let Some(spec_exp) = spec_opt {
                used.extend(spec_exp.directly_used_memory(env));
                let (_, spec_old) = compute_direct_old_usage(spec_exp, env);
                old.extend(spec_old);
            }
            let (_, body_old) = compute_direct_old_usage(body, env);
            old.extend(body_old);
            (used, old)
        },
        // Parameter forwarding: use the caller's modifies_of/reads_of for that parameter
        ExpData::Temporary(_, idx) => {
            // Map temp index to caller parameter name, then find matching access_of
            let param_name = caller_params.get(*idx).map(|Parameter(name, _, _)| *name);
            if let Some(name) = param_name {
                if let Some(access) = caller_param_access.iter().find(|a| a.fun_param == name) {
                    return (access.used_memory.clone(), access.old_memory.clone());
                }
            }
            // Parameter without modifies_of/reads_of or a local — overapproximate
            (caller_spec_used.clone(), caller_spec_old.clone())
        },
        // Anything else: overapproximate with caller's total memory footprint
        _ => (caller_spec_used.clone(), caller_spec_old.clone()),
    }
}

// -------------------------------------------------------------------------------------------
// Expressions Conversion

/// The expression converter takes a Move expression and converts it to a
/// specification expression. It expects the expression to be checked to be pure.
struct SpecConverter<'a> {
    env: &'a mut GlobalEnv,
    /// Whether we are in a specification expression. Conversion applies only if.
    in_spec: bool,
    /// The mapping from Move function to spec function ids.
    function_mapping: &'a BTreeMap<QualifiedId<FunId>, QualifiedId<SpecFunId>>,
    /// If non-empty, Temporary expressions should be mapped to symbolic LocalVar
    /// expressions. This is the convention for specification function parameters.
    symbolized_parameters: Vec<Symbol>,
    /// NodeIds which are exempted from stripping references. For compatibility
    /// reasons nodes which fetch temporaries should not be stripped as the reference
    /// info is needed for correct treatment of the `old(..)` expression.
    reference_strip_exempted: BTreeSet<NodeId>,
    /// Set true if the expression contains imperative expressions that currently cannot be translated into a spec function
    contains_imperative_expression: bool,
    /// Set to true when rewriting spec during inlining phase
    for_inline: bool,
    /// > 0 while descending into args of a behavioral-predicate call;
    /// preserves `Borrow(Mut, ...)` selectors so the model rewriter can
    /// detect them and route pre-state through `save_param`.
    bp_arg_depth: usize,
}

impl<'a> SpecConverter<'a> {
    fn new(
        env: &'a mut GlobalEnv,
        function_mapping: &'a BTreeMap<QualifiedId<FunId>, QualifiedId<SpecFunId>>,
        in_spec: bool,
    ) -> Self {
        Self {
            env,
            in_spec,
            function_mapping,
            symbolized_parameters: vec![],
            reference_strip_exempted: Default::default(),
            contains_imperative_expression: false,
            for_inline: false,
            bp_arg_depth: 0,
        }
    }

    fn new_for_inline(
        env: &'a mut GlobalEnv,
        function_mapping: &'a BTreeMap<QualifiedId<FunId>, QualifiedId<SpecFunId>>,
        in_spec: bool,
    ) -> Self {
        Self {
            env,
            in_spec,
            function_mapping,
            symbolized_parameters: vec![],
            reference_strip_exempted: Default::default(),
            contains_imperative_expression: false,
            for_inline: true,
            bp_arg_depth: 0,
        }
    }

    fn symbolized_parameters(self, symbolized_parameters: Vec<Symbol>) -> Self {
        Self {
            symbolized_parameters,
            ..self
        }
    }
}

impl ExpRewriterFunctions for SpecConverter<'_> {
    fn rewrite_exp(&mut self, exp: Exp) -> Exp {
        use ExpData::*;
        use Operation::*;
        if !self.in_spec {
            // If not in spec mode, check whether we need to switch to it,
            // and descent
            if matches!(exp.as_ref(), ExpData::SpecBlock(..)) {
                self.in_spec = true;
                let result = self.rewrite_exp_descent(exp);
                self.in_spec = false;
                result
            } else {
                let exp = self.rewrite_exp_descent(exp);
                if !self.for_inline {
                    if let ExpData::Invoke(id, call, args) = exp.as_ref() {
                        if let ExpData::Call(_, Closure(mid, fid, mask), captured) = call.as_ref() {
                            let mut new_args = vec![];
                            let mut captured_num = 0;
                            let mut free_num = 0;
                            let fun = self.env.get_function(mid.qualified(*fid));
                            for i in 0..fun.get_parameter_count() {
                                if mask.is_captured(i) {
                                    new_args.push(captured[captured_num].clone());
                                    captured_num += 1;
                                } else {
                                    new_args.push(args[free_num].clone());
                                    free_num += 1;
                                }
                            }
                            return Call(*id, MoveFunction(*mid, *fid), new_args.clone())
                                .into_exp();
                        }
                    }
                }
                exp
            }
        } else {
            // Simplification which need to be done before descent
            let exp = match exp.as_ref() {
                IfElse(id, _, if_true, if_false)
                    if matches!(if_true.as_ref(), Call(_, Tuple, _))
                        && matches!(if_false.as_ref(), Call(_, Abort(_), _)) =>
                {
                    // The code pattern produced by an `assert!`: `if (c) () else abort`.
                    // Reduce to unit
                    Call(*id, Tuple, vec![]).into_exp()
                },
                Temporary(id, _) | LocalVar(id, _) => {
                    self.reference_strip_exempted.insert(*id);
                    exp
                },
                _ => exp,
            };

            // Track BP-arg depth so `Borrow(Mut, ...)` selectors survive
            // for the model rewriter.
            let is_behavior = matches!(exp.as_ref(), Call(_, Behavior(_, _), _));
            if is_behavior {
                self.bp_arg_depth += 1;
            }
            let exp = self.rewrite_exp_descent(exp);
            if is_behavior {
                self.bp_arg_depth -= 1;
            }

            // Simplification after descent
            match exp.as_ref() {
                Temporary(id, idx) => {
                    // For specification functions, parameters are represented as LocalVar,
                    // so apply mapping if present.
                    if let Some(sym) = self.symbolized_parameters.get(*idx) {
                        LocalVar(*id, *sym).into_exp()
                    } else {
                        exp.clone()
                    }
                },
                Call(id, BorrowGlobal(ReferenceKind::Immutable), args) => {
                    // Map borrow_global to specification global
                    Call(*id, Global(None), args.clone()).into_exp()
                },
                Call(_, Borrow(ReferenceKind::Mutable), _) if self.bp_arg_depth > 0 => {
                    // Preserve in BP args so the model rewriter sees the
                    // stateful mut-ref shape.
                    exp.clone()
                },
                Call(_, Borrow(_), args) | Call(_, Deref, args) => {
                    // Skip local borrow
                    args[0].clone()
                },
                Call(id, MoveFunction(mid, fid), args) => {
                    // During inlining process, we skip replacing move fun with spec fun
                    // Later phase will do it for spec blocks
                    if !self.for_inline {
                        // Rewrite to associated spec function
                        let spec_fun_id = self
                            .function_mapping
                            .get(&mid.qualified(*fid))
                            .unwrap_or_else(|| {
                                panic!(
                                    "associated spec fun for {}",
                                    self.env.get_function(mid.qualified(*fid)).get_name_str()
                                )
                            });

                        Call(
                            *id,
                            SpecFunction(
                                spec_fun_id.module_id,
                                spec_fun_id.id,
                                MemoryRange::default(),
                            ),
                            args.clone(),
                        )
                        .into_exp()
                    } else {
                        exp.clone()
                    }
                },
                // Deal with removing various occurrences of Abort and spec blocks
                SpecBlock(id, ..) => {
                    // Replace direct call by unit
                    Call(*id, Tuple, vec![]).into_exp()
                },
                IfElse(id, _, if_true, if_false)
                    if matches!(if_true.as_ref(), Call(_, Tuple, _))
                        && matches!(if_false.as_ref(), Call(_, Abort(_), _)) =>
                {
                    // The code pattern produced by an `assert!`: `if (c) () else abort`.
                    // Reduce to unit as well
                    Call(*id, Tuple, vec![]).into_exp()
                },
                Sequence(id, exps) => {
                    // Remove aborts, units, and spec blocks
                    let mut reduced_exps = exps
                        .iter()
                        .take(exps.len() - 1)
                        .flat_map(|e| {
                            if matches!(
                                e.as_ref(),
                                SpecBlock(..) | Call(_, Abort(_), _) | Call(_, Tuple, _)
                            ) {
                                None
                            } else {
                                Some(e.clone())
                            }
                        })
                        .collect_vec();
                    reduced_exps.push(exps.last().unwrap().clone());
                    if reduced_exps.len() != exps.len() {
                        if reduced_exps.len() == 1 {
                            reduced_exps.pop().unwrap()
                        } else {
                            self.contains_imperative_expression = true;
                            Sequence(*id, reduced_exps).into_exp()
                        }
                    } else {
                        if reduced_exps.len() != 1 {
                            self.contains_imperative_expression = true;
                        }
                        exp.clone()
                    }
                },
                Invoke(id, call, args) => {
                    // Rewrite invoke into a spec function call
                    // this is for general function value
                    if let ExpData::Call(_, Closure(mid, fid, mask), captured) = call.as_ref() {
                        let spec_fun_id = self
                            .function_mapping
                            .get(&mid.qualified(*fid))
                            .unwrap_or_else(|| {
                                panic!(
                                    "associated spec fun for {}",
                                    self.env.get_function(mid.qualified(*fid)).get_name_str()
                                )
                            });
                        let spec_fun_decl: &SpecFunDecl = self.env.get_spec_fun(*spec_fun_id);
                        let mut new_args = vec![];
                        let mut captured_num = 0;
                        let mut free_num = 0;
                        for i in 0..spec_fun_decl.params.len() {
                            if mask.is_captured(i) {
                                new_args.push(captured[captured_num].clone());
                                captured_num += 1;
                            } else {
                                new_args.push(args[free_num].clone());
                                free_num += 1;
                            }
                        }

                        return Call(
                            *id,
                            SpecFunction(
                                spec_fun_id.module_id,
                                spec_fun_id.id,
                                MemoryRange::default(),
                            ),
                            new_args.clone(),
                        )
                        .into_exp();
                    }
                    exp.clone()
                },
                ExpData::Return(..)
                | ExpData::Loop(..)
                | ExpData::Assign(..)
                | ExpData::Mutate(..)
                | ExpData::LoopCont(..) => {
                    self.contains_imperative_expression = true;
                    exp.clone()
                },
                _ => exp.clone(),
            }
        }
    }

    fn rewrite_node_id(&mut self, id: NodeId) -> Option<NodeId> {
        if !self.in_spec || self.reference_strip_exempted.contains(&id) {
            // Skip the processing below
            return None;
        }
        if let Some(new_ty) = self
            .env
            .get_node_type_opt(id)
            .map(|ty| ty.skip_reference().clone())
        {
            let new_id = self.env.new_node(self.env.get_node_loc(id), new_ty);
            if let Some(new_inst) = self.env.get_node_instantiation_opt(id).map(|inst| {
                inst.into_iter()
                    .map(|ty| ty.skip_reference().clone())
                    .collect_vec()
            }) {
                self.env.set_node_instantiation(new_id, new_inst);
            }
            if let Some(weight) = self.env.get_quant_weight(id) {
                self.env.set_quant_weight(new_id, weight);
            }
            Some(new_id)
        } else {
            None
        }
    }
}

// -------------------------------------------------------------------------------------------
// Processing Invariants

fn collect_global_invariants_to_env(env: &mut GlobalEnv) {
    let mut invariants = vec![];
    for module_env in env.get_modules() {
        for cond in &module_env.get_spec().conditions {
            if matches!(
                cond.kind,
                ConditionKind::GlobalInvariant(..) | ConditionKind::GlobalInvariantUpdate(..)
            ) {
                let id = env.new_global_id();
                invariants.push(GlobalInvariant {
                    id,
                    loc: cond.loc.clone(),
                    kind: cond.kind.clone(),
                    mem_usage: cond
                        .exp
                        .used_memory(env)
                        .into_iter()
                        .map(|(mem, _)| mem.clone())
                        .collect(),
                    declaring_module: module_env.get_id(),
                    cond: cond.exp.clone(),
                    properties: cond.properties.clone(),
                });
            }
        }
    }
    for invariant in invariants {
        env.add_global_invariant(invariant)
    }
}

fn check_data_invariants(struct_env: &StructEnv) {
    let env = struct_env.module_env.env;
    for cond in &struct_env.get_spec().conditions {
        if matches!(cond.kind, ConditionKind::StructInvariant) {
            let usage = cond.exp.used_memory(env);
            if !usage.is_empty() {
                env.error(
                    &cond.loc,
                    &format!(
                        "data invariants cannot depend on global state \
                    but found dependency to `{}`",
                        usage
                            .into_iter()
                            .map(|(sid, _)| env.display(&sid).to_string())
                            .join(",")
                    ),
                )
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use move_core_types::account_address::AccountAddress;
    use move_model::{
        ast::{Address, ModuleName, Spec},
        exp_rewriter::ExpRewriterFunctions,
        ty::BOOL_TYPE,
    };
    use std::collections::BTreeMap;

    fn fresh_env() -> GlobalEnv {
        let mut env = GlobalEnv::new();
        let loc = Loc::default();
        let addr = Address::Numerical(AccountAddress::ZERO);
        let module_name = ModuleName::new(addr, env.symbol_pool().make("test_mod"));
        env.add(
            loc,
            module_name,
            vec![],
            vec![],
            vec![],
            BTreeMap::new(),
            BTreeMap::new(),
            BTreeMap::new(),
            vec![],
            vec![],
            vec![],
            Spec::default(),
            vec![],
        );
        env
    }

    #[test]
    fn spec_converter_rewrite_node_id_preserves_quant_weight() {
        let mut env = fresh_env();

        let id = env.new_node(Loc::default(), BOOL_TYPE.clone());
        env.set_quant_weight(id, 17);

        let function_mapping: BTreeMap<QualifiedId<FunId>, QualifiedId<SpecFunId>> =
            BTreeMap::new();
        let mut converter = SpecConverter::new(&mut env, &function_mapping, true);
        let new_id = converter
            .rewrite_node_id(id)
            .expect("rewrite produced a new node id");

        assert_ne!(new_id, id);
        assert_eq!(env.get_quant_weight(new_id), Some(17));
    }

    #[test]
    fn spec_converter_rewrite_node_id_skips_outside_spec() {
        let mut env = fresh_env();

        let id = env.new_node(Loc::default(), BOOL_TYPE.clone());
        env.set_quant_weight(id, 9);

        let function_mapping: BTreeMap<QualifiedId<FunId>, QualifiedId<SpecFunId>> =
            BTreeMap::new();
        let mut converter = SpecConverter::new(&mut env, &function_mapping, false);
        assert!(converter.rewrite_node_id(id).is_none());

        assert_eq!(env.get_quant_weight(id), Some(9));
    }
}
