// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

//! This module implements the inlining optimization, whose aim is to reduce the number
//! of function calls at the cost of increasing code size.
//!
//! See the documentation for the `optimize` function for more details on what inlining is.

use crate::{
    env_pipeline::rewrite_target::{RewriteState, RewriteTarget, RewriteTargets, RewritingScope},
    file_format_generator::MAX_LOCAL_COUNT,
};
use codespan_reporting::diagnostic::Severity;
use move_binary_format::file_format::Visibility;
use move_model::{
    ast::{Exp, ExpData, Operation, Pattern, TempIndex},
    exp_rewriter::ExpRewriterFunctions,
    metadata::LanguageVersion,
    model::{
        FunId, FunctionEnv, GlobalEnv, Loc, ModuleEnv, ModuleId, NodeId, Parameter, QualifiedId,
    },
    ty::Type,
};
use petgraph::{algo::kosaraju_scc, prelude::DiGraphMap};
use std::collections::BTreeSet;

/// A conservative heuristic limit posed by the inlining optimization.
const MAX_INSTRUCTIONS_PER_FUNCTION: usize = 4096;
/// Number of times we want to apply "unrolling" of functions with inlining.
const UNROLL_DEPTH: usize = 10;

/// Optimize functions in target modules by applying inlining transformations.
/// With inlining, a call site of the form:
/// ```move
/// foo(a, b, c, ...)
/// ```
/// with
/// ```move
/// fun foo(x, y, z, ...) { body }
/// ```
/// becomes:
/// ```move
/// let (x, y, z, ...) = (a, b, c, ...); body
/// ```
///
/// The `across_package` value controls if inlining is performed across package boundaries.
/// With across-package inlining, calls to functions in other packages may be inlined,
/// which means that if the other package is upgraded, one would get the behavior
/// at the inline-time rather than the latest upgrade.
pub fn optimize(env: &mut GlobalEnv, across_package: bool) {
    let mut targets = RewriteTargets::create(env, RewritingScope::CompilationTarget);
    let skip_functions = find_cycles_in_call_graph(env, &targets);
    targets.filter(|target, _| {
        if let RewriteTarget::MoveFun(function_id) = target {
            let function = env.get_function(*function_id);
            // We will consider inlining the callees in a function on if it satisfies all of:
            // - is not a part of a cycle in the call graph
            // - is in a primary target module
            // - is not in a script module
            // - is not a test only function
            // - is not a native function
            // - is not an inline function
            !skip_functions.contains(function_id)
                && function.module_env.is_primary_target()
                && !function.module_env.is_script_module()
                && !function.is_test_only()
                && !function.is_native()
                && !function.is_inline()
        } else {
            false
        }
    });
    let mut todo: Vec<_> = targets.keys().collect();
    for _ in 0..UNROLL_DEPTH {
        if todo.is_empty() {
            break;
        }
        todo = inline_call_sites(env, &mut targets, todo, across_package);
    }
    // Update the changed function definitions due to inlining.
    targets.write_to_env(env);
    // Inlining can cause direct calls to `package` functions that were previously
    // indirect. Thus, it may require additional caller modules to become friends
    // of the callee modules.
    env.update_friend_decls_in_targets();
}

/// Inline call sites in the given `todo` list of functions.
/// When inlining, the original definition of the callee is used, resulting in
/// one-level of "unrolling".
fn inline_call_sites(
    env: &GlobalEnv,
    targets: &mut RewriteTargets,
    todo: Vec<RewriteTarget>,
    across_package: bool,
) -> Vec<RewriteTarget> {
    let mut changed_targets = Vec::new();
    for target in todo {
        let RewriteTarget::MoveFun(function_id) = target else {
            continue;
        };
        let function_env = env.get_function(function_id);
        if let Some(def) = get_latest_def(targets, &target, &function_env) {
            let caller_module = &function_env.module_env;
            debug_assert!(
                caller_module.is_primary_target(),
                "caller module `{}`",
                caller_module.get_full_name_str()
            );
            let current_caller_cost = env.function_size.borrow().get(&function_id).copied();
            let Some(caller_cost) = current_caller_cost else {
                continue;
            };
            let (call_sites, new_size) =
                identify_call_sites_to_inline(env, caller_module, def, caller_cost, across_package);
            // rewrite
            let mut rewriter = CallerRewriter { env, call_sites };
            let rewritten_def = rewriter.rewrite_exp(def.clone());
            // If nothing has changed, no need to update.
            if !ExpData::ptr_eq(&rewritten_def, def) {
                *targets.state_mut(&target) = RewriteState::Def(rewritten_def);
                env.function_size.borrow_mut().insert(function_id, new_size);
                changed_targets.push(target);
            }
        }
    }
    changed_targets
}

fn get_latest_def<'a>(
    targets: &'a RewriteTargets,
    target: &'a RewriteTarget,
    function: &'a FunctionEnv,
) -> Option<&'a Exp> {
    if let RewriteState::Def(def) = targets.state(target) {
        // If the code has changed due to inlining, use updated definition.
        Some(def)
    } else if let Some(def) = function.get_def() {
        // If code is unchanged, use original definition.
        Some(def)
    } else {
        None
    }
}

fn find_cycles_in_call_graph(
    env: &GlobalEnv,
    targets: &RewriteTargets,
) -> BTreeSet<QualifiedId<FunId>> {
    let mut graph = DiGraphMap::<QualifiedId<FunId>, ()>::new();
    let mut cycle_nodes = BTreeSet::new();
    for target in targets.keys() {
        if let RewriteTarget::MoveFun(function) = target {
            graph.add_node(function);
        }
    }
    for caller in graph.nodes().collect::<Vec<_>>() {
        let caller_env = env.get_function(caller);
        for callee in caller_env
            .get_used_functions()
            .expect("used functions must be computed")
        {
            if callee == &caller {
                // self-recursion is added to the solution directly
                cycle_nodes.insert(caller);
            } else {
                // non-self-recursion edges
                graph.add_edge(caller, *callee, ());
            }
        }
    }
    for scc in kosaraju_scc(&graph) {
        if scc.len() > 1 {
            // cycle involving non-self-recursion
            cycle_nodes.extend(scc.into_iter());
        }
    }
    cycle_nodes
}

fn identify_call_sites_to_inline(
    env: &GlobalEnv,
    caller_module: &ModuleEnv,
    def: &Exp,
    caller_cost: (usize, usize),
    across_package: bool,
) -> (BTreeSet<NodeId>, (usize, usize)) {
    // find all eligible callsites to inline
    // find their costs and frequencies
    // sort by cost/frequency, pick as many as can fit within limit
    // [TODO]: artificially increase frequency for those within loops (^2 per loop depth?)
    let caller_mid = caller_module.get_id();
    let callees_with_callsites = def.called_funs_with_callsites();
    let mut inline_eligible_functions = callees_with_callsites
        .into_iter()
        .filter_map(|(callee, sites)| {
            let callee_env = env.get_function(callee);
            if callee_env.is_inline()
                || callee_env.is_native()
                || has_explicit_return(&callee_env)
                || has_privileged_operations(caller_mid, &callee_env)
                || has_invisible_calls(caller_module, &callee_env, across_package)
            {
                // cannot inline
                None
            } else {
                let (code_size, num_vars) = *env.function_size.borrow().get(&callee)?;
                let frequency = sites.len();
                debug_assert!(frequency > 0);
                let amortized_vars_per_site = num_vars as f64 / frequency as f64;
                Some((amortized_vars_per_site, sites, num_vars, code_size))
            }
        })
        .collect::<Vec<_>>();
    // sort inline eligible function by number of variables per call
    inline_eligible_functions.sort_by(|a, b| a.0.partial_cmp(&b.0).expect("no NaNs"));
    let (caller_code_size, caller_num_vars) = caller_cost;
    let mut var_budget = MAX_LOCAL_COUNT.saturating_sub(caller_num_vars);
    let mut code_size_budget = MAX_INSTRUCTIONS_PER_FUNCTION.saturating_sub(caller_code_size);
    let callsites_to_inline = inline_eligible_functions
        .into_iter()
        .take_while(|(_, sites, num_vars, code_size)| {
            if let (Some(new_var_budget), Some(new_code_size_budget)) = (
                var_budget.checked_sub(*num_vars),
                code_size_budget.checked_sub(*code_size * sites.len()),
            ) {
                var_budget = new_var_budget;
                code_size_budget = new_code_size_budget;
                true
            } else {
                false
            }
        })
        .map(|(_, sites, _, _)| sites)
        .flat_map(|sites| sites)
        .collect::<BTreeSet<_>>();
    (
        callsites_to_inline,
        (
            MAX_INSTRUCTIONS_PER_FUNCTION.saturating_sub(code_size_budget),
            MAX_LOCAL_COUNT.saturating_sub(var_budget),
        ),
    )
}

fn check_if_optimizeable(env: &mut GlobalEnv, across_package: bool) {
    let mut final_module_loc = None;
    let mut total_eligible = 0;
    for module in env.get_target_modules() {
        let mut eligible = 0;
        let caller_mid = module.get_id();
        for caller in module.get_functions() {
            if caller.is_inline() || caller.is_native() {
                continue;
            }
            if let Some(body) = caller.get_def() {
                let callees_with_callsites = body.called_funs_with_callsites();
                for (callee, sites) in callees_with_callsites {
                    let callee_env = module.env.get_function(callee);
                    debug_assert!(
                        !callee_env.is_inline(),
                        "inline functions should already be inlined"
                    );
                    if callee_env.is_native() {
                        // native functions cannot be inlined
                        continue;
                    }
                    if !has_privileged_operations(caller_mid, &callee_env)
                        && !has_invisible_calls(&module, &callee_env, across_package)
                    {
                        // Eligible for inlining
                        eligible += sites.len();
                        let cost = *env.function_size.borrow().get(&callee).unwrap_or(&(0, 0));
                        for site in sites {
                            let loc = env.get_node_loc(site);
                            let site_is_inlined = if loc.is_inlined() {
                                " (call site is inlined)"
                            } else {
                                ""
                            };
                            env.warning(
                                &loc,
                                format!("can inline with cost: {:?}{}", cost, site_is_inlined)
                                    .as_str(),
                            );
                        }
                    }
                }
            }
        }
        let module_loc = module.get_loc();
        final_module_loc = Some(module_loc.clone());
        env.warning(
            &module_loc,
            format!("found {} call sites eligible for inlining", eligible).as_str(),
        );
        total_eligible += eligible;
    }
    if let Some(loc) = final_module_loc {
        env.warning(
            &loc,
            format!(
                "in total, found {} call sites eligible for inlining",
                total_eligible
            )
            .as_str(),
        );
    }
}

struct CallerRewriter<'env> {
    env: &'env GlobalEnv,
    // callsites to inline
    call_sites: BTreeSet<NodeId>,
}

impl ExpRewriterFunctions for CallerRewriter<'_> {
    fn rewrite_call(&mut self, call_id: NodeId, op: &Operation, args: &[Exp]) -> Option<Exp> {
        if self.call_sites.contains(&call_id) {
            if let Operation::MoveFunction(mid, fid) = op {
                let callee = self.env.get_function(mid.qualified(*fid));
                // create an alternate expression to the call obtained via inlining
                // `foo(x, y, z, ..)`, where `fun foo(...) { body }`, becomes
                // `let (...) = (x, y, z, ..); body`
                let type_args = self.env.get_node_instantiation(call_id);
                let call_site_loc = self.env.get_node_loc(call_id);
                let mut callee_rewriter = CalleeRewriter {
                    function_env: &callee,
                    type_args: &type_args,
                    call_site_loc: &call_site_loc,
                };
                let Some(body) = callee.get_def() else {
                    return None;
                };
                let rewritten_body = callee_rewriter.rewrite_exp(body.clone());
                let params_pattern = callee_rewriter.params_to_pattern(&callee);
                Some(Self::construct_inlined_call_expression(
                    &self,
                    &call_site_loc,
                    rewritten_body,
                    params_pattern,
                    args.to_vec(),
                ))
            } else {
                unreachable!("callsites should be calls to MoveFunction")
            }
        } else {
            None
        }
    }
}

impl CallerRewriter<'_> {
    fn construct_inlined_call_expression(
        &self,
        call_site_loc: &Loc,
        body: Exp,
        params_pattern: Pattern,
        args: Vec<Exp>,
    ) -> Exp {
        let body_node_id = body.as_ref().node_id();
        let body_type = self.env.get_node_type(body_node_id);
        let body_loc = self
            .env
            .get_node_loc(body_node_id)
            .inlined_from(call_site_loc);

        let new_block_id = self.env.new_node(body_loc, body_type);

        let optional_binding_exp = if args.is_empty() {
            None
        } else {
            let args_node_ids = args
                .iter()
                .map(|e| e.as_ref().node_id())
                .collect::<Vec<_>>();
            let args_types = args_node_ids
                .iter()
                .map(|id| self.env.get_node_type(*id))
                .collect::<Vec<_>>();
            let args_loc = Loc::enclosing(
                args_node_ids
                    .iter()
                    .map(|id| self.env.get_node_loc(*id))
                    .collect::<Vec<_>>()
                    .as_slice(),
            );
            let new_binding_id = self.env.new_node(args_loc, Type::Tuple(args_types));
            Some(ExpData::Call(new_binding_id, Operation::Tuple, args).into_exp())
        };

        ExpData::Block(new_block_id, params_pattern, optional_binding_exp, body).into_exp()
    }
}

struct CalleeRewriter<'a> {
    function_env: &'a FunctionEnv<'a>,
    type_args: &'a Vec<Type>,
    call_site_loc: &'a Loc,
}

impl ExpRewriterFunctions for CalleeRewriter<'_> {
    fn rewrite_node_id(&mut self, id: NodeId) -> Option<NodeId> {
        let loc = self.function_env.env().get_node_loc(id);
        let new_loc = loc.inlined_from(self.call_site_loc);
        ExpData::instantiate_node_new_loc(self.function_env.env(), id, self.type_args, &new_loc)
    }

    fn rewrite_pattern(&mut self, pat: &Pattern, _entering_scope: bool) -> Option<Pattern> {
        let old_id = pat.node_id();
        let new_id = ExpData::instantiate_node(self.function_env.env(), old_id, self.type_args)
            .unwrap_or(old_id);
        match pat {
            Pattern::Struct(_, struct_id, variant, pattern_vec) => {
                let new_struct_id = struct_id.clone().instantiate(self.type_args);
                Some(Pattern::Struct(
                    new_id,
                    new_struct_id,
                    *variant,
                    pattern_vec.clone(),
                ))
            },
            Pattern::Tuple(_, pattern_vec) => Some(Pattern::Tuple(new_id, pattern_vec.clone())),
            Pattern::Var(_, symbol) => Some(Pattern::Var(new_id, *symbol)),
            Pattern::Wildcard(_) => None,
            Pattern::Error(_) => None,
        }
    }

    /// Replaces the temporary referring to the parameter with the corresponding symbol.
    fn rewrite_temporary(&mut self, id: NodeId, idx: TempIndex) -> Option<Exp> {
        if let Some(Parameter(sym, ty, loc)) = self.function_env.get_parameters_ref().get(idx) {
            let inst_ty = ty.instantiate(self.type_args);
            let new_node_id = self.function_env.env().new_node(loc.clone(), inst_ty);
            Some(ExpData::LocalVar(new_node_id, *sym).into())
        } else {
            let loc = self.function_env.env().get_node_loc(id);
            self.function_env.env().diag(
                Severity::Bug,
                &loc,
                &format!(
                    "temporary with invalid index `{}` when applying inlining optimization",
                    idx
                ),
            );
            None
        }
    }
}

impl CalleeRewriter<'_> {
    fn params_to_pattern(&self, function: &FunctionEnv) -> Pattern {
        let params = function.get_parameters();
        let function_loc = function.get_id_loc();
        let tuple_args = params
            .iter()
            .map(|Parameter(sym, ty, loc)| {
                let id = self
                    .function_env
                    .env()
                    .new_node(loc.clone(), ty.instantiate(self.type_args));
                if self
                    .function_env
                    .env()
                    .language_version()
                    .is_at_least(LanguageVersion::V2_1)
                    && self.function_env.symbol_pool().string(*sym).as_ref() == "_"
                {
                    Pattern::Wildcard(id)
                } else {
                    Pattern::Var(id, *sym)
                }
            })
            .collect::<Vec<_>>();
        let tuple_types = params
            .iter()
            .map(|p| p.1.instantiate(self.type_args))
            .collect::<Vec<_>>();
        let id = self.function_env.env().new_node(
            function_loc.inlined_from(self.call_site_loc),
            Type::Tuple(tuple_types),
        );
        Pattern::Tuple(id, tuple_args)
    }
}

/// Does `callee` have any privileged operations on structs/enums that cannot be performed
/// directly in a caller with module id `caller_mid`?
fn has_privileged_operations(caller_mid: ModuleId, callee: &FunctionEnv) -> bool {
    let env = callee.env();
    // keep track of any privileged operations
    let mut found = false;
    let mut spec_blocks_seen = 0;
    if let Some(body) = callee.get_def() {
        body.visit_pre_post(&mut |post, exp: &ExpData| {
            if !post {
                if matches!(exp, ExpData::SpecBlock(..)) {
                    spec_blocks_seen += 1;
                }
                if spec_blocks_seen > 0 {
                    // within a spec block, we can have privileged operations
                    return true;
                }
                // not inside a spec block, see if there are any privileged operations
                match exp {
                    ExpData::Call(id, op, _) => match op {
                        Operation::Exists(_)
                        | Operation::BorrowGlobal(_)
                        | Operation::MoveFrom
                        | Operation::MoveTo => {
                            let inst = env.get_node_instantiation(*id);
                            if let Some((struct_env, _)) = inst[0].get_struct(env) {
                                let struct_mid = struct_env.module_env.get_id();
                                if struct_mid != caller_mid {
                                    found = true;
                                }
                            }
                        },
                        Operation::Select(mid, ..) => {
                            if *mid != caller_mid {
                                found = true;
                            }
                        },
                        Operation::SelectVariants(mid, ..) => {
                            if *mid != caller_mid {
                                found = true;
                            }
                        },
                        Operation::TestVariants(mid, ..) => {
                            if *mid != caller_mid {
                                found = true;
                            }
                        },
                        Operation::Pack(mid, ..) => {
                            if *mid != caller_mid {
                                found = true;
                            }
                        },
                        _ => {},
                    },
                    // various ways to unpack
                    ExpData::Assign(_, pat, _)
                    | ExpData::Block(_, pat, ..)
                    | ExpData::Lambda(_, pat, ..) => pat.visit_pre_post(&mut |post, pat| {
                        if !post {
                            if let Pattern::Struct(_, sid, ..) = pat {
                                let struct_mid = sid.module_id;
                                if struct_mid != caller_mid {
                                    found = true;
                                }
                            }
                        }
                    }),
                    ExpData::Match(_, discriminator, _) => {
                        let did = discriminator.node_id();
                        if let Type::Struct(mid, ..) = env.get_node_type(did).drop_reference() {
                            if mid != caller_mid {
                                found = true;
                            }
                        }
                    },
                    _ => {},
                }
            } else {
                if matches!(exp, ExpData::SpecBlock(..)) {
                    spec_blocks_seen -= 1;
                }
            }
            // skip scanning for privileged operations if we already found one
            !found
        });
    }
    found
}

/// Does `callee` have any calls to functions that are not visible from `caller_module`?
/// pre-requisite: `caller_module` must be a primary target
fn has_invisible_calls(
    caller_module: &ModuleEnv,
    callee: &FunctionEnv,
    across_package: bool,
) -> bool {
    debug_assert!(caller_module.is_primary_target());
    let env = callee.env();
    let caller_mid = caller_module.get_id();
    if let Some(body) = callee.get_def() {
        for called_fun_id in body.used_funs() {
            let called_function = env.get_function(called_fun_id);
            let called_mid = called_function.module_env.get_id();
            if called_mid == caller_mid {
                // same module, so visible
                continue;
            }
            // TODO(#13745): hack for checking if two modules are in the same package
            let same_package = caller_module.self_address()
                == called_function.module_env.self_address()
                && called_function.module_env.is_primary_target();

            match called_function.visibility() {
                Visibility::Public => {
                    if !same_package && !across_package {
                        // public function in a different package cannot be inlined due to change
                        // in semantics on package upgrade, but we allow it when across-package
                        // inlining is enabled
                        return true;
                    }
                },
                Visibility::Private => {
                    return true;
                },
                Visibility::Friend => {
                    // Note: `is_friend` implies `same_package`
                    let is_friend = called_function.module_env.has_friend(&caller_mid);
                    if is_friend || (called_function.has_package_visibility() && same_package) {
                        // 1. a call to a friend function whose module has the caller module as a friend
                        //    is visible and belongs to the same package
                        // 2. a call to a package function belonging to the same package is also visible
                        //    TODO: we may have to add explicit friend declarations in this case though?
                        continue;
                    }
                    return true;
                },
            }
        }
    }
    false
}

fn has_explicit_return(function: &FunctionEnv) -> bool {
    let Some(exp) = function.get_def() else {
        return false;
    };
    let mut found = false;
    exp.visit_pre_order(&mut |e: &ExpData| {
        if let ExpData::Return(..) = e {
            found = true;
        }
        !found
    });
    found
}

fn cost(callee: &FunctionEnv) -> usize {
    let mut cost = 0;
    if let Some(body) = callee.get_def() {
        body.visit_pre_order(&mut |exp: &ExpData| {
            match exp {
                ExpData::Match(.., arms) => {
                    cost += arms.len();
                },
                ExpData::Call(..)
                | ExpData::Invoke(..)
                | ExpData::Lambda(..)
                | ExpData::IfElse(..)
                | ExpData::Loop(..)
                | ExpData::LoopCont(..)
                | ExpData::Assign(..)
                | ExpData::Mutate(..) => {
                    cost += 1;
                },
                _ => {},
            }
            true
        });
    }
    cost
}
