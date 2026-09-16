// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! This module implements the inlining optimization, whose aim is to reduce the number
//! of function calls (which comes at the cost of increasing code size).
//!
//! See the documentation for the `optimize` function for more details on what inlining is.
//!
//! Only callees in the caller's own module are inlined. Inlining across modules
//! would have to preserve every property the language and the VM scope to a
//! module: e.g., struct privacy, function visibility, which module emits an
//! event, which module an abort is attributed to, the frames counted by the
//! VM's reentrancy checks, independence from upgrades of other packages. Each
//! needs its own exception, some cannot be decided from the callee's body at
//! all but only from who calls it at runtime. We have determined that after all
//! these exceptions, the remaining benefit is not worth the complexity added.
//!
//! Within a module none of these properties can change, so a static call to a
//! callee without `#[module_lock]` is always safe to inline.

use crate::{
    env_pipeline::rewrite_target::{RewriteState, RewriteTarget, RewriteTargets, RewritingScope},
    file_format_generator::MAX_LOCAL_COUNT,
};
use codespan_reporting::diagnostic::Severity;
use move_model::{
    ast::{Exp, ExpData, Operation, Pattern, TempIndex},
    exp_rewriter::ExpRewriterFunctions,
    model::{
        FunId, FunctionEnv, FunctionSize, GlobalEnv, Loc, ModuleId, NodeId, Parameter, QualifiedId,
    },
    ty::Type,
};
use once_cell::sync::Lazy;
use petgraph::{algo::kosaraju_scc, prelude::DiGraphMap};
use std::{collections::BTreeSet, env};

/// [TODO]: tune the heuristic limits below
/// A conservative heuristic limit posed by the inlining optimization on how
/// large a caller function can grow to due to inlining.
pub static MAX_CALLER_CODE_SIZE: Lazy<usize> = Lazy::new(|| {
    env::var("MAX_CALLER_CODE_SIZE")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(1024)
});
/// A conservative heuristic limit posed by the inlining optimization on how
/// large a callee function can be for it to be considered for inlining.
pub static MAX_CALLEE_CODE_SIZE: Lazy<usize> = Lazy::new(|| {
    env::var("MAX_CALLEE_CODE_SIZE")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(128)
});
/// Number of times we want to apply "unrolling" of functions with inlining.
pub static UNROLL_DEPTH: Lazy<usize> = Lazy::new(|| {
    env::var("UNROLL_DEPTH")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(10)
});

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
/// If `allow_non_primary_targets` is true, also optimize non-primary target modules.
pub fn optimize(env: &mut GlobalEnv, allow_non_primary_targets: bool) {
    let mut targets = RewriteTargets::create(env, RewritingScope::CompilationTarget);
    let skip_functions = find_cycles_in_call_graph(env, &targets);
    targets.filter(|target, _| {
        if let RewriteTarget::MoveFun(function_id) = target {
            let function = env.get_function(*function_id);
            // We will consider inlining the callees in a function only if it satisfies all of:
            // - is not part of a cycle of same-module calls
            // - is in a primary target module (if `allow_non_primary_targets` is false)
            // - is not in a script module
            // - is not a test only function
            // - is not a verify only function
            // - is not a native function
            // - is not an inline function
            !skip_functions.contains(function_id)
                && (allow_non_primary_targets || function.module_env.is_primary_target())
                && !function.module_env.is_script_module()
                && !function.is_test_only()
                && !function.is_verify_only()
                && !function.is_native()
                && !function.is_inline()
        } else {
            // only move functions are considered for inlining optimization
            false
        }
    });
    let mut todo: Vec<_> = targets.keys().collect();
    // Each time you unroll, a call site may be substituted with the original body of the callee.
    for _ in 0..*UNROLL_DEPTH {
        if todo.is_empty() {
            break;
        }
        todo = inline_call_sites(env, &mut targets, todo);
    }
    // Update the changed function definitions due to inlining.
    targets.write_to_env(env);
}

/// Inline call sites in the given `todo` list of functions.
/// When inlining, the original definition of the callee is used, resulting in
/// one-level of "unrolling".
/// The function size estimates are also updated based on the inlining decisions.
/// Return the list of functions that were changed due to inlining.
fn inline_call_sites(
    env: &GlobalEnv,
    targets: &mut RewriteTargets,
    todo: Vec<RewriteTarget>,
) -> Vec<RewriteTarget> {
    let mut changed_targets = Vec::new();
    for target in todo {
        let RewriteTarget::MoveFun(function_id) = target else {
            continue;
        };
        let function_env = env.get_function(function_id);
        if let Some(def) = get_latest_function_definition(targets, &target, &function_env) {
            let current_caller_size = env
                .function_size_estimate
                .borrow()
                .get(&function_id)
                .copied();
            let Some(caller_size) = current_caller_size else {
                continue;
            };
            let (call_sites, new_size) = compute_call_sites_to_inline_and_new_function_size(
                env,
                function_env.module_env.get_id(),
                def,
                caller_size,
            );
            if call_sites.is_empty() {
                continue;
            }
            let mut rewriter = CallerRewriter { env, call_sites };
            // Rewrite the caller's body with inlined call sites.
            let rewritten_def = rewriter.rewrite_exp(def.clone());
            // If nothing has changed, no need to update.
            if !ExpData::ptr_eq(&rewritten_def, def) {
                *targets.state_mut(&target) = RewriteState::Def(rewritten_def);
                env.function_size_estimate
                    .borrow_mut()
                    .insert(function_id, new_size);
                changed_targets.push(target);
            }
        }
    }
    changed_targets
}

/// Get the "latest" definition of a `function`. If a function has some callsites
/// already inlined (unrolled up to some depth), then that definition is used.
fn get_latest_function_definition<'a>(
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

/// Find functions in cycles of same-module calls, including self-recursion.
/// Cross-module calls are never inlined, so cycles through them cannot be unrolled.
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
            } else if callee.module_id == caller.module_id {
                // non-self-recursion edges
                graph.add_edge(caller, *callee, ());
            }
        }
    }
    for scc in kosaraju_scc(&graph) {
        if scc.len() > 1 {
            // cycle involving non-self-recursion
            cycle_nodes.extend(scc);
        }
    }
    cycle_nodes
}

/// For a given caller function `def`, find all the call sites that are eligible for inlining,
/// compute their costs, and pick as many as can fit within the heuristic limits.
/// Return the set of call sites to inline, and the new estimated size of the caller function
/// after inlining those call sites.
fn compute_call_sites_to_inline_and_new_function_size(
    env: &GlobalEnv,
    caller_mid: ModuleId,
    def: &Exp,
    caller_function_size: FunctionSize,
) -> (BTreeSet<NodeId>, FunctionSize) {
    let callees = def.called_funs_with_callsites_and_loop_depth();
    // Find all the callees that are eligible for inlining.
    let inline_eligible_functions = callees
        .into_iter()
        .filter_map(|(callee, sites_and_loop_depth)| {
            let callee_env = env.get_function(callee);
            let callee_size = get_function_size_estimate(env, &callee);
            if !is_inlinable_callee(caller_mid, &callee_env, callee_size) {
                None
            } else {
                let callee_frequency = sites_and_loop_depth.len();
                assert!(callee_frequency > 0);
                // Note that the number of locals introduced by inlining a callee can be amortized
                // by the number of times the callee is called, because these variables have
                // non-overlapping lifetimes and will likely be coalesced.
                // We also currently either inline all the callsites of a callee or none (for simplicity).
                let locals_per_site = callee_size.num_locals;
                let amortized_locals_per_site = locals_per_site.div_ceil(callee_frequency);
                let max_loop_depth = sites_and_loop_depth
                    .iter()
                    .map(|(_, depth)| depth)
                    .max()
                    .copied()
                    .unwrap_or_default();
                let sites = sites_and_loop_depth
                    .iter()
                    .map(|(s, _)| s)
                    .copied()
                    .collect::<BTreeSet<_>>();
                let code_size = callee_size.code_size;
                Some((sites, CalleeInfo {
                    max_loop_depth,
                    amortized_locals_per_site,
                    code_size,
                    locals_per_site,
                }))
            }
        })
        .collect::<Vec<_>>();
    pick_from_eligible_and_compute_cost(inline_eligible_functions, caller_function_size)
}

/// Various information about a callee function that is eligible for inlining.
struct CalleeInfo {
    /// Maximum loop depth of any of the call sites of this callee.
    max_loop_depth: usize,
    /// Amortized number of locals per call site after inlining.
    amortized_locals_per_site: usize,
    /// Estimated code size of the callee function.
    code_size: usize,
    /// Estimate number of locals in the callee function.
    locals_per_site: usize,
}

/// Given a list of eligible callsites to inline, pick as many as possible within the
/// heuristic limits, and compute the new estimated size of the caller function.
/// Each element of `inline_eligible_callees` is a pair of:
/// - set of call sites of a callee
/// - information about the callee that can help in making inlining decisions
fn pick_from_eligible_and_compute_cost(
    mut inline_eligible_callees: Vec<(BTreeSet<NodeId>, CalleeInfo)>,
    caller_function_size: FunctionSize,
) -> (BTreeSet<NodeId>, FunctionSize) {
    inline_eligible_callees.sort_by(|a, b| {
        // Sort callees based on the following (tie-break leads to next criteria):
        // 1. Highest loop depth of any of the call sites
        // 2. Lower amortized locals per call site
        // 3. Lower code size of callee
        // 4. Lower frequency of callee
        // 5. Lower locals per call site
        b.1.max_loop_depth
            .cmp(&a.1.max_loop_depth)
            .then_with(|| {
                a.1.amortized_locals_per_site
                    .cmp(&b.1.amortized_locals_per_site)
            })
            .then_with(|| a.1.code_size.cmp(&b.1.code_size))
            .then_with(|| a.0.len().cmp(&b.0.len()))
            .then_with(|| a.1.locals_per_site.cmp(&b.1.locals_per_site))
    });
    let FunctionSize {
        code_size,
        num_locals,
    } = caller_function_size;
    let locals_budget = MAX_LOCAL_COUNT.saturating_sub(num_locals);
    let code_size_budget = MAX_CALLER_CODE_SIZE.saturating_sub(code_size);
    let mut locals_budget_remaining = locals_budget;
    let mut code_size_budget_remaining = code_size_budget;
    let mut call_sites_to_inline = BTreeSet::new();
    // Go over the sorted callees and greedily pick as many as fit within the budget.
    for (sites, callee_info) in inline_eligible_callees {
        if locals_budget_remaining == 0 || code_size_budget_remaining == 0 {
            // no more budget left
            break;
        }
        if let (Some(locals_reduced), Some(code_size_reduced)) = (
            locals_budget_remaining.checked_sub(callee_info.locals_per_site),
            code_size_budget_remaining.checked_sub(callee_info.code_size * sites.len()),
        ) {
            call_sites_to_inline.extend(sites);
            // Note that we reduce the remaining budget for number of locals once for
            // all the callsites of a callee, because we expect to coalesce the locals at
            // different callsites.
            // The remaining budget for code size is reduced for each callsite.
            locals_budget_remaining = locals_reduced;
            code_size_budget_remaining = code_size_reduced;
        } else {
            // cannot inline this callee due to budget limits
            // maybe try the next candidate
            continue;
        }
    }
    (
        call_sites_to_inline,
        FunctionSize::new(
            code_size + (code_size_budget - code_size_budget_remaining),
            num_locals + (locals_budget - locals_budget_remaining),
        ),
    )
}

fn get_function_size_estimate(env: &GlobalEnv, function: &QualifiedId<FunId>) -> FunctionSize {
    env.function_size_estimate
        .borrow()
        .get(function)
        .copied()
        .unwrap_or_default()
}

/// Returns whether `callee` is eligible for inlining into module `caller_mid`.
fn is_inlinable_callee(
    caller_mid: ModuleId,
    callee: &FunctionEnv,
    callee_size: FunctionSize,
) -> bool {
    // Only same-module callees are inlined; see the module documentation.
    callee.module_env.get_id() == caller_mid
        // The module lock is acquired by the callee's own frame.
        && !callee.has_module_lock()
        // Explicit inline functions are expanded earlier; native functions have no body.
        && !callee.is_inline()
        && !callee.is_native()
        // Bound code growth.
        && callee_size.code_size <= *MAX_CALLEE_CODE_SIZE
        // Inlined returns would exit the caller without additional rewriting.
        && !has_explicit_return(callee)
}

/// Does `function` have an explicit return statement in its body?
fn has_explicit_return(function: &FunctionEnv) -> bool {
    function
        .get_def()
        .is_some_and(|exp| exp.any(&mut |e| matches!(e, ExpData::Return(..))))
}

/// Rewriter for a caller function to inline the call sites in it.
struct CallerRewriter<'env> {
    env: &'env GlobalEnv,
    // The call sites that have been picked to inline
    call_sites: BTreeSet<NodeId>,
}

impl ExpRewriterFunctions for CallerRewriter<'_> {
    fn rewrite_call(&mut self, call_id: NodeId, op: &Operation, args: &[Exp]) -> Option<Exp> {
        if self.call_sites.contains(&call_id) {
            if let Operation::MoveFunction(mid, fid) = op {
                let callee = self.env.get_function(mid.qualified(*fid));
                let type_args = self.env.get_node_instantiation(call_id);
                let call_site_loc = self.env.get_node_loc(call_id);
                let mut callee_rewriter = CalleeRewriter {
                    function_env: &callee,
                    type_args: &type_args,
                    call_site_loc: &call_site_loc,
                };
                let body = callee.get_def()?;
                // Rewrite the body of the callee to adjust node ids, types, and locations.
                let rewritten_body = callee_rewriter.rewrite_exp(body.clone());
                // Construct a pattern for the parameters of the callee to create a `let` binding.
                let params_pattern = callee_rewriter.params_to_pattern(&callee);
                // Perform inlining transformation at the call site.
                Some(self.construct_inlined_call_expression(
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
    /// Given a transformed `body` of the callee, a tuple pattern `(x, y, ..)`
    /// corresponding to the parameters of the callee, and the arguments
    /// `a, b, ..` at the call site, construct the inlined expression:
    /// ```move
    /// let (x, y, ..) = (a, b, ..); body
    /// ```
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

/// Rewriter for the callee function at a given call site.
struct CalleeRewriter<'a> {
    /// The function environment of the callee.
    function_env: &'a FunctionEnv<'a>,
    /// The type arguments at the call site.
    type_args: &'a Vec<Type>,
    /// The location of the call site.
    call_site_loc: &'a Loc,
}

impl ExpRewriterFunctions for CalleeRewriter<'_> {
    /// Update node ids to new ones, and update their locations to reflect inlining.
    fn rewrite_node_id(&mut self, id: NodeId) -> Option<NodeId> {
        let loc = self.function_env.env().get_node_loc(id);
        let new_loc = loc.inlined_from(self.call_site_loc);
        ExpData::instantiate_node_new_loc(self.function_env.env(), id, self.type_args, &new_loc)
    }

    /// Update patterns to use new node ids.
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
            Pattern::Wildcard(_) => Some(Pattern::Wildcard(new_id)),
            Pattern::LiteralValue(_, val) => Some(Pattern::LiteralValue(new_id, val.clone())),
            Pattern::Range(_, lo, hi, inc) => {
                Some(Pattern::Range(new_id, lo.clone(), hi.clone(), *inc))
            },
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
    /// Construct a tuple pattern for the parameters of the callee function.
    /// E.g., for a function with parameters `x: T, y: bool`, this returns
    /// the pattern `(x, y)`, with `x` and `y` instantiated with respective
    /// types known at this call site.
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
                if self.function_env.symbol_pool().string(*sym).as_ref() == "_" {
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
