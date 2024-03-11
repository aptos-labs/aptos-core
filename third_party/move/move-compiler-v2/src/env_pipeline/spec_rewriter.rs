// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

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
use log::info;
use move_model::{
    ast::{ConditionKind, Exp, ExpData, GlobalInvariant, Operation, SpecFunDecl},
    exp_rewriter::ExpRewriterFunctions,
    model::{FunId, GlobalEnv, NodeId, Parameter, QualifiedId, SpecFunId, StructEnv},
    symbol::Symbol,
    ty::ReferenceKind,
};
use std::{
    cell::RefCell,
    collections::{BTreeMap, BTreeSet, VecDeque},
};

pub fn run_spec_rewriter(env: &mut GlobalEnv) {
    info!("rewriting specifications");

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
                        spec_callees.extend(spec.called_funs_with_callsites().into_keys());
                        true // keep going
                    });
                    spec_callees
                } else {
                    BTreeSet::new()
                }
            },
            RewriteTarget::SpecFun(_) | RewriteTarget::SpecBlock(_) => target
                .called_funs_with_call_sites(env)
                .into_keys()
                .collect(),
        };
        for callee in callees {
            called_funs.insert(callee);
            let mut transitive = env
                .get_function(callee)
                .get_transitive_closure_of_called_functions();
            called_funs.append(&mut transitive);
        }
    }

    // For compatibility reasons with the v1 way how to compile spec
    // blocks of inline functions, we also need to add all 'lambda'
    // lifted functions.

    // Derive spec functions for all called Move functions,
    // building a mapping between function ids. Also add
    // those new spec functions to `targets` for subsequent
    // processing.
    let mut function_mapping = BTreeMap::new();
    for fun_id in called_funs {
        let spec_fun_id = derive_spec_fun(env, fun_id);
        function_mapping.insert(fun_id, spec_fun_id);
        // Add new spec fun to targets for later processing
        targets.entry(RewriteTarget::SpecFun(spec_fun_id));
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
            (MoveFun(_), Def(exp)) => {
                let mut converter = SpecConverter::new(env, &function_mapping, false);
                let new_exp = converter.rewrite_exp(exp.clone());
                if !ExpData::ptr_eq(&new_exp, &exp) {
                    *targets.state_mut(&target) = Def(new_exp)
                }
            },
            (SpecFun(id), Def(exp)) => {
                let mut converter = SpecConverter::new(env, &function_mapping, true)
                    .symbolized_parameters(get_param_names(&env.get_spec_fun(*id).params));
                let new_exp = converter.rewrite_exp(exp.clone());
                if !ExpData::ptr_eq(&new_exp, &exp) {
                    *targets.state_mut(&target) = Def(new_exp)
                }
            },
            (SpecBlock(sb_target), Spec(spec)) => {
                let mut converter = SpecConverter::new(env, &function_mapping, true);
                let (changed, new_spec) = converter.rewrite_spec_descent(sb_target, &spec);
                if changed {
                    *targets.state_mut(&target) = Spec(new_spec)
                }
            },
            _ => {},
        }
    }
    targets.write_to_env(env);

    // Now that all functions are defined, do a DFS traversal of all specification
    // functions to compute transitive callee and used memory.
    let mut todo: VecDeque<_> = env
        .get_modules()
        .flat_map(|m| {
            m.get_spec_funs()
                .map(|(id, _)| m.get_id().qualified(*id))
                .collect_vec()
        })
        .collect();

    let mut deps_scheduled = BTreeSet::new();
    let mut done = BTreeSet::new();
    while let Some(next) = todo.pop_front() {
        if done.contains(&next) {
            continue;
        }
        if deps_scheduled.insert(next) {
            // First time visiting this function, compute the initial direct usage.
            let decl = env.get_spec_fun(next);
            let (callees, used_memory) = if let Some(exp) = &decl.body {
                (exp.called_spec_funs(), exp.directly_used_memory(env))
            } else {
                Default::default()
            };
            // Schedule to process the deps, and after this to process  this one
            // again.
            todo.extend(callees.iter().filter(|id| !done.contains(*id)));
            todo.push_back(next);
            // Now store the usage.
            let decl_mut = env.get_spec_fun_mut(next);
            (decl_mut.callees, decl_mut.used_memory) = (callees, used_memory);
        } else {
            // The deps are processed, or we are hitting a cycle. In the case of
            // a cycle, we will have `!done.contains(&callee)`, but this is fine,
            // because the direct usage is sufficient to reach a fixpoint.
            for callee in env.get_spec_fun(next).callees.iter().cloned().collect_vec() {
                let callee_decl = env.get_spec_fun(callee);
                let mut transitive_callee = callee_decl.callees.clone();
                let mut transitive_usage = callee_decl.used_memory.clone();
                let decl_mut = env.get_spec_fun_mut(next);
                decl_mut.callees.append(&mut transitive_callee);
                decl_mut.used_memory.append(&mut transitive_usage);
            }
            done.insert(next);
        }
    }

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

// -------------------------------------------------------------------------------------------
// Deriving Specification Functions

// Derive a specification function from a Move function. Initially the body is the
// original one, not yet converted to the specification representation.
fn derive_spec_fun(env: &mut GlobalEnv, fun_id: QualifiedId<FunId>) -> QualifiedId<SpecFunId> {
    let fun = env.get_function(fun_id);
    let (is_native, body) = if fun.is_native() {
        (true, None)
    } else {
        let exp = fun.get_def().expect("function body").clone();
        (false, Some(exp))
    };

    // For historical reasons, those names are prefixed with `$` even though there
    // is no name clash allowed.
    let name = env
        .symbol_pool()
        .make(&format!("${}", fun.get_name().display(env.symbol_pool())));
    // Eliminate references in parameters and result type
    let params = fun
        .get_parameters()
        .into_iter()
        .map(|Parameter(sym, ty, loc)| Parameter(sym, ty.skip_reference().clone(), loc))
        .collect();
    let result_type = fun.get_result_type().skip_reference().clone();

    let decl = SpecFunDecl {
        loc: fun.get_loc(),
        name,
        type_params: fun.get_type_parameters(),
        params,
        context_params: None,
        result_type,
        used_memory: BTreeSet::new(),
        uninterpreted: false,
        is_move_fun: true,
        is_native,
        body,
        callees: BTreeSet::new(),
        is_recursive: RefCell::new(None),
    };
    env.add_spec_function_def(fun_id.module_id, decl)
}

// -------------------------------------------------------------------------------------------
// Expressions Conversion

/// The expression converter takes a Move expression and converts it to a
/// specification expression. It expects the expression to be checked to be pure.
struct SpecConverter<'a> {
    env: &'a GlobalEnv,
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
}

impl<'a> SpecConverter<'a> {
    fn new(
        env: &'a GlobalEnv,
        function_mapping: &'a BTreeMap<QualifiedId<FunId>, QualifiedId<SpecFunId>>,
        in_spec: bool,
    ) -> Self {
        Self {
            env,
            in_spec,
            function_mapping,
            symbolized_parameters: vec![],
            reference_strip_exempted: Default::default(),
        }
    }

    fn symbolized_parameters(self, symbolized_parameters: Vec<Symbol>) -> Self {
        Self {
            symbolized_parameters,
            ..self
        }
    }
}

impl<'a> ExpRewriterFunctions for SpecConverter<'a> {
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
                self.rewrite_exp_descent(exp)
            }
        } else {
            // Simplification which need to be done before descent
            let exp = match exp.as_ref() {
                IfElse(id, _, if_true, if_false)
                    if matches!(if_true.as_ref(), Call(_, Tuple, _))
                        && matches!(if_false.as_ref(), Call(_, Abort, _)) =>
                {
                    // The code pattern produced by an `assert!`: `if (c) () else abort`.
                    // Reduce to unit
                    Call(*id, Tuple, vec![]).into_exp()
                },
                Temporary(id, _) => {
                    self.reference_strip_exempted.insert(*id);
                    exp
                },
                _ => exp,
            };

            let exp = self.rewrite_exp_descent(exp);

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
                Call(_, Borrow(_), args) | Call(_, Deref, args) => {
                    // Skip local borrow
                    args[0].clone()
                },
                Call(id, MoveFunction(mid, fid), args) => {
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
                        SpecFunction(spec_fun_id.module_id, spec_fun_id.id, None),
                        args.clone(),
                    )
                    .into_exp()
                },
                // Deal with removing various occurrences of Abort and spec blocks
                Call(id, Abort, _) | SpecBlock(id, ..) => {
                    // Replace direct call by unit
                    Call(*id, Tuple, vec![]).into_exp()
                },
                IfElse(id, _, if_true, if_false)
                    if matches!(if_true.as_ref(), Call(_, Tuple, _))
                        && matches!(if_false.as_ref(), Call(_, Abort, _)) =>
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
                                SpecBlock(..) | Call(_, Abort, _) | Call(_, Tuple, _)
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
                            Sequence(*id, reduced_exps).into_exp()
                        }
                    } else {
                        exp.clone()
                    }
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
