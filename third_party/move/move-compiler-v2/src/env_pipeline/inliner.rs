// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Inlining Overview:
//! - We visit function calling inline functions reachable from compilation targets in a bottom-up
//!   fashion, storing rewritten functions in a map to simplify further processing.
//!   - Change to the program happens at the end.
//!
//! Summary of structs/impls in this file.  Note that these duplicate comments in the body of this file,
//! and ideally should be updated if those are changed significantly.
//! - function `run_inlining` is the main entry point for the inlining pass
//!
//! - struct `Inliner`
//!   - holds the map recording function bodies which are rewritten due to inlining so that we don't
//!     need to modify the program until the end.
//!   - `do_inlining_in` function is the entry point for each function needing inlining.
//!
//! - struct `OuterInlinerRewriter` uses trait `ExpRewriterFunctions` to rewrite each call in the
//!   target.
//!
//! - struct `InlinedRewriter` rewrites a call to an inlined function
//!   - `inline_call` is the external entry point for rewriting a call to an inline function.
//!
//!   - `construct_inlined_call_expression` is a helper to build the `Block` expression corresponding
//!      to { let params=actuals; body } used for both lambda inlining and inline function inlining.
//!
//! - struct `InlinedRewriter` uses trait `ExpRewriterFunctions` to rewrite the inlined function
//!      body.
//!   - `rewrite_exp` is the entry point to rewrite the body of an inline function.
//!
//! - struct ShadowStack implements the free variable shadowing stack:
//!   For a given set of "free" variables, the `ShadowStack` tracks which variables are
//!   still directly visible, and which variables have been hidden by local variable
//!   declarations with the same symbol.  In the latter case, the ShadowStack provides
//!   a "shadow" symbol which can be used in place of the original.
//!
//! - TODO(10858): add an anchor AST node so we can implement `Return` for inline functions and
//!   `Lambda`.

use super::lambda_lifter::{LambdaLifter, LambdaLiftingOptions};
use crate::{
    env_pipeline::{
        rewrite_target::{RewriteState, RewriteTarget, RewriteTargets, RewritingScope},
        spec_rewriter::run_spec_rewriter_inline,
    },
    experiments::Experiment,
    options::Options,
};
use codespan_reporting::diagnostic::Severity;
use itertools::Itertools;
use log::trace;
use move_core_types::ability::AbilitySet;
use move_model::{
    ast::{
        BehaviorKind, Condition, ConditionKind, Exp, ExpData, LambdaCaptureKind, MemoryLabel,
        MemoryRange, Operation, Pattern, QuantKind, Spec, SpecBlockTarget, SpecFunDecl, TempIndex,
        Value, VisitorPosition,
    },
    exp_generator::FunExpGenerator,
    exp_rewriter::{ExpRewriter, ExpRewriterFunctions, RewriteTarget as ExpRewriteTarget},
    model::{
        FunId, FunctionEnv, GlobalEnv, Loc, NodeId, Parameter, QualifiedId, SpecFunId,
        TypeParameter, TypeParameterKind,
    },
    pragmas::{ABORTS_IF_IS_PARTIAL_PRAGMA, CONDITION_INFERRED_PROP},
    pureness_checker::{FunctionPurenessChecker, FunctionPurenessCheckerMode},
    spec_derivation::{self, DerivedSpec},
    symbol::Symbol,
    ty::{PrimitiveType, ReferenceKind, Type},
    well_known,
};
use num::BigInt;
use std::{
    cell::{Cell, RefCell},
    collections::{BTreeMap, BTreeSet},
    fmt::Debug,
    iter,
    iter::{zip, IntoIterator, Iterator},
    rc::Rc,
    vec::Vec,
};

// TODO(#20371): derive exact global-state HOF effects instead of weakening.
const INLINE_HOF_WEAKENING_ISSUE: &str = "https://github.com/aptos-labs/aptos-core/issues/20371";
const FORWARDED_FOLD_WEAKENING_ISSUE: &str =
    "https://github.com/aptos-labs/aptos-core/issues/20383";
const FOLDS_OF_INVARIANT_MARKER: &str = "$inliner_folds_of_invariant";

type QualifiedFunId = QualifiedId<FunId>;
type CallSiteLocations = BTreeMap<(RewriteTarget, QualifiedFunId), BTreeSet<NodeId>>;

#[derive(Default)]
struct UnresolvedBehaviorNodes {
    nodes: RefCell<BTreeSet<NodeId>>,
    spec_funs: RefCell<BTreeSet<QualifiedId<SpecFunId>>>,
}

fn unresolved_behavior_nodes(env: &GlobalEnv) -> Rc<UnresolvedBehaviorNodes> {
    match env.get_extension::<UnresolvedBehaviorNodes>() {
        Some(nodes) => nodes,
        None => {
            env.set_extension(UnresolvedBehaviorNodes::default());
            env.get_extension::<UnresolvedBehaviorNodes>()
                .expect("extension just set")
        },
    }
}

fn mark_unresolved_behavior(env: &GlobalEnv, id: NodeId) {
    unresolved_behavior_nodes(env).nodes.borrow_mut().insert(id);
}

fn mark_unresolved_spec_fun(env: &GlobalEnv, id: QualifiedId<SpecFunId>) {
    unresolved_behavior_nodes(env)
        .spec_funs
        .borrow_mut()
        .insert(id);
}

fn is_unresolved_behavior(env: &GlobalEnv, exp: &ExpData) -> bool {
    env.get_extension::<UnresolvedBehaviorNodes>()
        .is_some_and(|unresolved| {
            unresolved.nodes.borrow().contains(&exp.node_id())
                || matches!(
                    exp,
                    ExpData::Call(_, Operation::SpecFunction(mid, sid, _), _)
                        if unresolved.spec_funs.borrow().contains(&mid.qualified(*sid))
                )
        })
}

fn weaken_or_mark_unresolved(env: &GlobalEnv, exp: Exp) -> Exp {
    let id = exp.node_id();
    if let ExpData::Call(_, Operation::SpecFunction(mid, sid, _), _) = exp.as_ref() {
        mark_unresolved_spec_fun(env, mid.qualified(*sid));
    }
    mark_unresolved_behavior(env, id);
    exp
}

const DEBUG: bool = false;

// ======================================================================================
// Entry

/// Run inlining on current program's AST.  For each function which is target of the compilation,
/// visit that function body and inline any calls to functions marked as "inline".
pub fn run_inlining(
    env: &mut GlobalEnv,
    scope: RewritingScope,
    keep_inline_functions: bool,
    lift_inline_funs: bool,
) {
    // Get function roots for running inlining.
    // Also generate errors for any invalid target inline functions.
    let mut targets = RewriteTargets::create(env, scope);
    check_and_maybe_filter_targets(env, &mut targets);
    let mut todo: BTreeSet<_> = targets.keys().collect();

    // Only look for inlining sites if we have targets to inline into.
    if !todo.is_empty() {
        // Recursively find callees of each target with a function body.

        // The call graph reachable from targets, represented by a map from each target to the set
        // of functions it calls.  The domain is limited to functions with function bodies.
        let mut call_graph: BTreeMap<RewriteTarget, BTreeSet<QualifiedFunId>> = BTreeMap::new();

        // For each function `caller` calling an inline function `callee`, we record the set of all
        // call sites where `caller` calls `callee` (for error messages).
        let mut inline_function_call_site_locations: CallSiteLocations = CallSiteLocations::new();

        // Update call_graph and inline_function_call_site_locations for all reachable calls.
        let mut visited_targets = BTreeSet::new();
        while let Some(target) = todo.pop_first() {
            if visited_targets.insert(target.clone()) {
                let callees_with_sites = target.used_funs_with_uses(env);
                for (callee, sites) in callees_with_sites {
                    todo.insert(RewriteTarget::MoveFun(callee));
                    targets.entry(RewriteTarget::MoveFun(callee));
                    call_graph.entry(target.clone()).or_default().insert(callee);
                    if env.get_function(callee).is_inline() {
                        inline_function_call_site_locations.insert((target.clone(), callee), sites);
                    }
                }
            }
        }

        // Get a list of all reachable targets calling inline functions, in bottom-up order.
        // If there are any cycles, this call displays an error to the user and returns None.
        if let Ok(mut targets_needing_inlining) =
            targets_needing_inlining_in_order(env, &call_graph, inline_function_call_site_locations)
        {
            // In verify mode, targets which do not call inline functions may
            // still contain calls to spec functions with literal lambda
            // arguments (e.g. a lemma or caller spec restating a lambda
            // passed to an inline function); they are rewritten by
            // specialization. They come last, after all expansion sites, so
            // their specializations unify with the expansion-site ones.
            if env.is_verify_mode() {
                let included: BTreeSet<RewriteTarget> =
                    targets_needing_inlining.iter().cloned().collect();
                targets_needing_inlining.extend(targets.keys().filter(|target| {
                    !included.contains(target) && has_literal_lambda_spec_call(env, target)
                }));
            }

            // We inline functions bottom-up, so that any inline function which itself has calls to
            // inline functions has already had its stuff inlined.
            let mut inliner = Inliner::new(env, targets, lift_inline_funs);
            for target in targets_needing_inlining.into_iter() {
                inliner.do_inlining_in(target);
            }

            // Now that all inlining finished, actually update definitions in env.
            inliner.inline_targets.write_to_env(env);
        }
    }

    // Delete all inline functions with bodies from the program rep, even if none were inlined,
    // since (1) they are no longer needed, and (2) they may have code constructs that codegen can't
    // deal with.
    //
    // This can be overridden by `keep_inline_functions`, which maybe helpful in debugging
    // scenarios since env dumping crashes if the functions are removed but still referenced
    // from somewhere.
    if !keep_inline_functions {
        // First construct a list of functions to remove.
        let mut inline_funs = BTreeSet::new();
        for module in env.get_modules() {
            for func in module.get_functions() {
                let id = func.get_qualified_id();
                if func.is_inline() && func.get_def().is_some() && !func.is_inline_verified() {
                    // Only delete functions with a body, and keep verified inline
                    // functions, whose bodies are checked against their specs.
                    inline_funs.insert(id);
                }
            }
        }
        env.retain_functions(|fun_id: &QualifiedFunId| !inline_funs.contains(fun_id));
    }
}

/// Check that inline functions are (1) not native, (2) have a body, (3) are not in a script,
/// (4) do not have certain attributes, and (5) do not have access specifiers.
/// Filter out inline functions from the targets if `Experiment::SKIP_INLINING_INLINE_FUNS` is on.
fn check_and_maybe_filter_targets(env: &GlobalEnv, targets: &mut RewriteTargets) {
    let keep_inline_functions = !env
        .get_extension::<Options>()
        .unwrap_or_default()
        .experiment_on(Experiment::SKIP_INLINING_INLINE_FUNS);
    targets.filter(|target: &RewriteTarget, _| {
        if let RewriteTarget::MoveFun(fnid) = target {
            let func = env.get_function(*fnid);
            if func.is_inline() {
                if func.get_def().is_none() {
                    let func_loc = func.get_loc();
                    let func_name = func.get_name_str();
                    if func.is_native() {
                        let msg = format!("Inline function `{}` must not be native", func_name);
                        env.error(&func_loc, &msg);
                    } else {
                        let msg = format!(
                            "No body found for non-native inline function `{}`",
                            func_name
                        );
                        env.diag(Severity::Bug, &func_loc, &msg);
                    }
                }

                if func.module_env.is_script_module() {
                    env.error(
                        &func.get_id_loc(),
                        "inline function cannot be defined in a script",
                    );
                }

                if func.has_attribute(|attr| {
                    let name = env.symbol_pool().string(attr.name());
                    name.as_str() == well_known::PERSISTENT_ATTRIBUTE
                        || name.as_str() == well_known::MODULE_LOCK_ATTRIBUTE
                }) {
                    env.error(
                        &func.get_id_loc(),
                        "inline functions cannot have the following attributes: `#[persistent]`, `#[module_lock]`",
                    );
                }

                if func.get_access_specifiers().is_some() {
                    env.warning(
                        &func.get_id_loc(),
                        "acquires annotations are not applicable to inline functions and should be removed",
                    );
                }

                keep_inline_functions
            } else {
                // not an inline function
                true
            }
        } else {
            // not a move function
            true
        }
    });
}

/// Returns for each argument position holding a literal lambda expression,
/// the position and the lambda. Spec function calls with such arguments are
/// rewritten by specialization in verify mode.
fn literal_lambda_bindings(args: &[Exp]) -> Vec<(usize, Exp)> {
    args.iter()
        .enumerate()
        .filter(|(_, arg)| matches!(arg.as_ref(), ExpData::Lambda(..)))
        .map(|(pos, arg)| (pos, arg.clone()))
        .collect()
}

/// Returns true if the target contains a call to a spec function with a
/// literal lambda argument. Such calls require specialization rewriting even
/// if the target does not call any inline function.
fn has_literal_lambda_spec_call(env: &GlobalEnv, target: &RewriteTarget) -> bool {
    let mut check_exp = |e: &ExpData| {
        matches!(e, ExpData::Call(_, Operation::SpecFunction(..), args)
            if !literal_lambda_bindings(args).is_empty())
    };
    match target {
        RewriteTarget::MoveFun(id) => env
            .get_function(*id)
            .get_def()
            .is_some_and(|def| def.any(&mut check_exp)),
        RewriteTarget::SpecFun(id) => env
            .get_spec_fun(*id)
            .body
            .as_ref()
            .is_some_and(|body| body.any(&mut check_exp)),
        RewriteTarget::SpecBlock(sb_target) => {
            let mut found = false;
            env.get_spec_block(sb_target)
                .visit_positions(&mut |pos, e| {
                    if matches!(pos, VisitorPosition::Pre) && check_exp(e) {
                        found = true;
                        None
                    } else {
                        Some(())
                    }
                });
            found
        },
    }
}

/// Return a list of all inline functions calling inline functions, in bottom-up order,
/// so that any inline function will be processed before any function calling it.
fn targets_needing_inlining_in_order(
    env: &GlobalEnv,
    call_graph: &BTreeMap<RewriteTarget, BTreeSet<QualifiedFunId>>,
    inline_function_call_site_locations: CallSiteLocations,
) -> Result<Vec<RewriteTarget>, ()> {
    let is_inline_fun = |fnid: &QualifiedFunId| env.get_function(*fnid).is_inline();
    let inline_fun_target_opt = |target: &RewriteTarget| {
        if let RewriteTarget::MoveFun(fnid) = target {
            if is_inline_fun(fnid) {
                Some(*fnid)
            } else {
                None
            }
        } else {
            None
        }
    };
    // Subset of the call graph limited to inline functions.
    let inline_function_call_graph: BTreeMap<QualifiedFunId, BTreeSet<QualifiedFunId>> = call_graph
        .iter()
        .filter_map(|(target, callees)| inline_fun_target_opt(target).map(|fid| (fid, callees)))
        .map(|(caller_fnid, callees)| {
            (
                caller_fnid,
                callees
                    .iter()
                    .filter(|callee_fnid| is_inline_fun(callee_fnid))
                    .cloned()
                    .collect(),
            )
        })
        .collect();

    // Set of inline functions calling at least one inline function.
    let inline_functions_calling_others: Vec<QualifiedFunId> = inline_function_call_graph
        .iter()
        .filter(|(_, callees)| !callees.is_empty())
        .map(|(caller_fnid, _)| caller_fnid)
        .cloned()
        .collect();

    // Check for cycles
    let cycles = check_for_cycles(&inline_function_call_graph);
    if !cycles.is_empty() {
        for cycle in cycles {
            let start_fnid = cycle.first().unwrap();
            let func_env = env.get_function(*start_fnid);
            let path_string: String = cycle
                .iter()
                .map(|fnid| env.get_function(*fnid).get_full_name_str())
                .collect::<Vec<String>>()
                .join("` -> `");
            let mut call_details: Vec<_> = cycle
                .iter()
                .zip(cycle.iter().skip(1).chain(iter::once(start_fnid)))
                .flat_map(|(f, g)| {
                    let sites_ids = inline_function_call_site_locations
                        .get(&(RewriteTarget::MoveFun(*f), *g))
                        .unwrap();
                    let f_str = env.get_function(*f).get_full_name_str();
                    let g_str = env.get_function(*g).get_full_name_str();
                    let msg = format!("call from `{}` to `{}`", f_str, g_str);
                    sites_ids
                        .iter()
                        .map(move |node_id| (env.get_node_loc(*node_id), msg.clone()))
                })
                .collect();
            let msg = format!(
                "cyclic recursion involving only inline functions is not allowed: `{}` -> `{}`",
                path_string,
                func_env.get_full_name_str()
            );
            let loc = call_details.first_mut().unwrap().0.clone();
            env.diag_with_labels(Severity::Error, &loc, &msg, call_details);
        }
        return Err(());
    }

    // Compute post-order of inline_functions which call others.  This lists each function
    // before any others which call it.
    let po_inline_functions = postorder(
        &inline_functions_calling_others,
        &inline_function_call_graph,
    );
    let mut result: Vec<RewriteTarget> = po_inline_functions
        .into_iter()
        .map(RewriteTarget::MoveFun)
        .collect();

    // Add subset of non-inline function targets which call inline functions.  Order
    // doesn't matter here.
    result.extend(
        call_graph
            .iter()
            .filter(|(target, callees)| {
                inline_fun_target_opt(target).is_none() && callees.iter().any(is_inline_fun)
            })
            .map(|(target, _)| target.clone()),
    );

    Ok(result)
}

/// Calculate a bottom-up traversal for entries, given the provided callgraph,
/// which maps callers to callees.
fn postorder<T: Ord + Copy + Debug>(
    entries: &Vec<T>,
    call_graph: &BTreeMap<T, BTreeSet<T>>,
) -> Vec<T> {
    let mut stack = Vec::new();
    let mut visited = BTreeSet::new();
    let mut grey = BTreeSet::new();
    let mut postorder_num_to_node = Vec::new();

    for entry in entries {
        if !visited.contains(&entry) {
            visited.insert(entry);
            stack.push(entry);
            while let Some(curr) = stack.pop() {
                if grey.contains(&curr) {
                    postorder_num_to_node.push(*curr);
                } else {
                    grey.insert(curr);
                    stack.push(curr);
                    if let Some(children) = call_graph.get(curr) {
                        for child in children {
                            if !visited.contains(child) {
                                visited.insert(child);
                                stack.push(child);
                            }
                        }
                    }
                }
            }
        }
    }
    postorder_num_to_node
}

/// Check for cycles in a call_graph, mapping callers to callees..
/// If there is a cycle, return at least one cyclical path.
fn check_for_cycles<T: Ord + Copy + Debug>(
    call_graph: &BTreeMap<T, BTreeSet<T>>,
) -> BTreeSet<Vec<T>> {
    let mut cycles: BTreeSet<Vec<T>> = BTreeSet::new();
    let mut reachable_from_map: BTreeMap<T, BTreeSet<Vec<T>>> = call_graph
        .iter()
        .map(|(node, set)| (*node, std::iter::repeat_n(vec![*node], set.len()).collect()))
        .collect();

    let mut changed = true;
    let mut new_paths: BTreeSet<Vec<T>> = BTreeSet::new();
    while changed {
        changed = false;
        for (start_node, path_set) in reachable_from_map.iter_mut() {
            for path in path_set.iter() {
                let path_last = path.last().unwrap();
                if let Some(succ_set) = call_graph.get(path_last) {
                    if succ_set.contains(start_node) {
                        // found a cycle, return it.
                        // TODO(10983): maybe find all cycles?
                        cycles.insert(path.to_vec());
                        return cycles;
                    }
                    for succ in succ_set.iter() {
                        let mut appended_path = path.clone();
                        appended_path.push(*succ);
                        if !path_set.contains(&appended_path) {
                            new_paths.insert(appended_path);
                        }
                    }
                }
            }
            if !new_paths.is_empty() {
                changed = true;
                path_set.append(&mut new_paths);
                new_paths = BTreeSet::new();
            }
        }
    }
    cycles
}

struct Inliner<'env> {
    env: &'env mut GlobalEnv,
    /// The set of rewrite targets the inliner works on.
    inline_targets: RewriteTargets,
    /// Flag to lift lambda expression arguments to inline functions
    lift_inline_funs: bool,
    /// Spec function specializations generated in this run, unified across
    /// contexts; see `SpecFunUnifierEntry`.
    spec_fun_unifier: Vec<SpecFunUnifierEntry>,
    /// Bespoke multi-capture fold recursions generated for `folds_of`
    /// resolutions in this run, unified across expansions; see
    /// `FoldsOfRecursionEntry`.
    folds_of_unifier: Vec<FoldsOfRecursionEntry>,
}

impl<'env> Inliner<'env> {
    fn new(
        env: &'env mut GlobalEnv,
        inline_targets: RewriteTargets,
        lift_inline_funs: bool,
    ) -> Self {
        Self {
            env,
            inline_targets,
            lift_inline_funs,
            spec_fun_unifier: vec![],
            folds_of_unifier: vec![],
        }
    }

    /// If the target has expressions containing calls to inline functions, then
    /// - makes a copy of the target with every call to any inline function `callee` replaced by
    ///   either
    ///   - the mapping found in `self.inline_results` for `callee`, or
    ///   - the original body of `callee` (as obtained from `self.env: &GlobalEnv`)
    /// - stores a mapping from `target` to inlining result in `self.inline_results`
    /// Otherwise, stores a mapping from `target` to `InlineResult::Unchanged` in
    /// `self.inline_results`
    ///
    /// This should be called on `target` only after all inline functions it calls are processed.
    /// It must not be called more than once for any given `target`.
    fn do_inlining_in(&mut self, target: RewriteTarget) {
        use RewriteState::*;
        use RewriteTarget::*;
        assert_eq!(self.inline_targets.entry(target.clone()).1, &Unchanged);
        match &target {
            MoveFun(func_id) => {
                let func_env = self.env.get_function(*func_id);
                let def_opt = func_env.get_def();
                if let Some(def) = def_opt {
                    if let Some(new_def) = self.do_rewrite_exp(def.clone(), Some(*func_id)) {
                        *self.inline_targets.state_mut(&target) = Def(new_def)
                    }
                }
            },
            SpecFun(func_id) => {
                let func_env = self.env.get_spec_fun(*func_id);
                if let Some(def) = func_env.body.clone() {
                    if let Some(new_def) = self.do_rewrite_exp(def, None) {
                        *self.inline_targets.state_mut(&target) = Def(new_def);
                    }
                }
            },
            SpecBlock(sb_target) => {
                let spec = self.env.get_spec_block(sb_target).clone();
                let fun_target_id = target.get_rewrite_target_fun_id();
                if let Some(new_spec) = self.do_rewrite_spec(sb_target, spec.clone(), fun_target_id)
                {
                    *self.inline_targets.state_mut(&target) = Spec(new_spec)
                }
            },
        }
    }

    fn do_rewrite_exp(&mut self, exp: Exp, target: Option<QualifiedFunId>) -> Option<Exp> {
        let mut rewriter = OuterInlinerRewriter::new(self, target);
        let rewritten = rewriter.rewrite_exp(exp.clone());
        if !ExpData::ptr_eq(&rewritten, &exp) {
            Some(rewritten)
        } else {
            None
        }
    }

    fn do_rewrite_spec(
        &mut self,
        target: &SpecBlockTarget,
        spec: Spec,
        fun_target: Option<QualifiedFunId>,
    ) -> Option<Spec> {
        let mut rewriter = OuterInlinerRewriter::new(self, fun_target);
        let (changed, new_spec) = rewriter.rewrite_spec_descent(target, &spec);
        if changed {
            Some(new_spec)
        } else {
            None
        }
    }
}

/// `OuterInlinerRewriter` implements `ExpRewriterFunctions` to processing functions which may have
/// inline function calls within them.  The only thing it rewrites are calls to inline functions; we
/// use the ExpRewriterFunctions trait to find such calls and reconstruct the outer function to
/// include them after rewriting.
struct OuterInlinerRewriter<'env, 'inliner> {
    /// Functions already processed all get an entry here, with a new function body after inline
    /// calls are substituted here.
    inliner: &'inliner mut Inliner<'env>,
    /// Caller of the inline function
    current_fun_target_opt: Option<QualifiedFunId>,
}

#[derive(Clone, Copy)]
struct InlineCallSummarySpec {
    result: QualifiedId<SpecFunId>,
    aborts: QualifiedId<SpecFunId>,
}

impl<'env, 'inliner> OuterInlinerRewriter<'env, 'inliner> {
    fn new(inliner: &'inliner mut Inliner<'env>, current_target: Option<QualifiedFunId>) -> Self {
        Self {
            inliner,
            current_fun_target_opt: current_target,
        }
    }

    /// Specializes a call to a spec function with literal lambda arguments
    /// and redirects the call, dropping the lambda arguments and appending
    /// the context arguments. On failure an error has been reported and the
    /// call is left unchanged.
    fn specialize_literal_lambda_call(
        &mut self,
        call_id: NodeId,
        qid: QualifiedId<SpecFunId>,
        range: &MemoryRange,
        args: &[Exp],
        bindings: Vec<(usize, Exp)>,
    ) -> Exp {
        let inliner = &mut *self.inliner;
        let loc = inliner.env.get_node_loc(call_id);
        let inst = inliner.env.get_node_instantiation(call_id);
        let mut specializer = SpecFunSpecializer::new(
            inliner.env,
            self.current_fun_target_opt,
            &mut inliner.spec_fun_unifier,
        );
        let Some(spec) = specializer.specialize(&loc, qid, inst, bindings, None) else {
            // Error already reported by `specialize`.
            return ExpData::Call(
                call_id,
                Operation::SpecFunction(qid.module_id, qid.id, range.clone()),
                args.to_vec(),
            )
            .into_exp();
        };
        let env: &GlobalEnv = self.inliner.env;
        let retained_args: Vec<Exp> = spec.retained.iter().map(|pos| args[*pos].clone()).collect();
        spec.make_call(env, loc, range, retained_args, &BTreeMap::new())
    }
}

impl ExpRewriterFunctions for OuterInlinerRewriter<'_, '_> {
    /// recognize call to inline function and rewrite it using `InlinedRewriter::inline_call`
    fn rewrite_call(&mut self, call_id: NodeId, oper: &Operation, args: &[Exp]) -> Option<Exp> {
        if let Operation::MoveFunction(module_id, fun_id) = oper {
            let qfid = module_id.qualified(*fun_id);
            let func_env = self.inliner.env.get_function(qfid);
            if func_env.is_inline() && !func_env.is_inline_opaque_retained() {
                // inline the function call
                let type_args = self.inliner.env.get_node_instantiation(call_id);
                let parameters = func_env.get_parameters();
                let func_loc = func_env.get_id_loc();
                let body_expr = if let RewriteState::Def(expr) = self
                    .inliner
                    .inline_targets
                    .state(&RewriteTarget::MoveFun(qfid))
                {
                    // `qfid` was previously inlined into, use the post-inlining copy of body.
                    Some(expr.clone())
                } else {
                    // `qfid` was not previously inlined into, look for the original body expr.
                    let func_env_def = func_env.get_def();
                    func_env_def.cloned()
                };
                // inline here
                if let Some(expr) = body_expr {
                    if DEBUG {
                        trace!(
                            "inlining function `{}` with args `{}`",
                            self.inliner.env.dump_fun(&func_env),
                            args.iter()
                                .map(|exp| format!("{}", exp.as_ref().display(self.inliner.env)))
                                .collect::<Vec<_>>()
                                .join(","),
                        );
                    }
                    let lift_inline_funs = self.inliner.lift_inline_funs;
                    let inline_call_summary = if self.inliner.env.is_verify_mode()
                        && func_env.module_env.is_std_vector()
                        && self
                            .inliner
                            .env
                            .symbol_pool()
                            .string(func_env.get_name())
                            .as_ref()
                            == "map_ref"
                    {
                        let result = well_known::find_spec_fun_in_module(
                            &func_env.module_env,
                            well_known::VECTOR_SPEC_MAP_REF,
                        );
                        let aborts = well_known::find_spec_fun_in_module(
                            &func_env.module_env,
                            well_known::VECTOR_SPEC_MAP_REF_ABORTS,
                        );
                        result.zip(aborts).and_then(|(result, aborts)| {
                            let callees = expr.called_spec_funs(self.inliner.env);
                            let calls =
                                |qid| callees.iter().any(|callee| callee.to_qualified_id() == qid);
                            (calls(result) && calls(aborts))
                                .then_some(InlineCallSummarySpec { result, aborts })
                        })
                    } else {
                        None
                    };
                    let inliner = &mut *self.inliner;
                    let rewritten = InlinedRewriter::inline_call(
                        inliner.env,
                        call_id,
                        &func_loc,
                        &expr,
                        type_args,
                        parameters,
                        args,
                        lift_inline_funs,
                        self.current_fun_target_opt,
                        &mut inliner.spec_fun_unifier,
                        &mut inliner.folds_of_unifier,
                        inline_call_summary,
                    );

                    if DEBUG {
                        trace!(
                            "After inlining, expr is `{}`",
                            rewritten.display(self.inliner.env)
                        );
                    }
                    Some(rewritten)
                } else {
                    None
                }
            } else {
                None
            }
        } else if let Operation::SpecFunction(mid, sid, labels) = oper {
            // In verify mode, calls to spec functions with literal lambda
            // arguments can appear in specifications outside of an inline
            // expansion, e.g. in a lemma restating a lambda passed to an
            // inline function. They are resolved by specialization through
            // the global unifier, so occurrences equivalent to an
            // expansion-site specialization share its specialized function.
            // In regular compilation, the call is left as is.
            if !self.inliner.env.is_verify_mode() {
                return None;
            }
            let bindings = literal_lambda_bindings(args);
            if bindings.is_empty() {
                None
            } else {
                Some(self.specialize_literal_lambda_call(
                    call_id,
                    mid.qualified(*sid),
                    labels,
                    args,
                    bindings,
                ))
            }
        } else {
            None
        }
    }
}

/// For a given set of "free" variables, the `ShadowStack` tracks which variables are
/// still directly visible, and which variables have been hidden by local variable
/// declarations with the same symbol.  In the latter case, the ShadowStack provides
/// a "shadow" symbol which can be used in place of the original.
struct ShadowStack {
    /// Unique shadow var for each "free" var, immutable for the life of the ShadowStack.
    shadow_symbols: BTreeMap<Symbol, Symbol>,

    /// Inverse of shadow_symbols for more efficient scoping
    shadow_symbols_inverse: BTreeMap<Symbol, Symbol>,

    /// Subset of free vars shadowed at each scope
    scoped_shadowed_vars: Vec<Vec<Symbol>>,

    /// Maps each of "free var" to a count of shadowing scopes surrounding the current point.
    /// - Entries are eagerly created to map each var to 0.
    /// - Entry for var incremented/decremented as each scope shadowing var is entered/exited.
    scoped_shadowed_count: BTreeMap<Symbol, usize>,
}

impl ShadowStack {
    pub fn new<'a, T>(env: &GlobalEnv, free_vars: T) -> Self
    where
        T: IntoIterator<Item = &'a Symbol>,
    {
        let shadow_symbols = Self::create_shadow_symbols(env, free_vars);
        let shadow_symbols_inverse = shadow_symbols
            .iter()
            .map(|(key, value)| (*value, *key))
            .collect();
        // Make a counter entry for every shadow symbol.
        let scoped_shadowed_count = shadow_symbols.keys().map(|sym| (*sym, 0)).collect();
        Self {
            shadow_symbols,
            shadow_symbols_inverse,
            scoped_shadowed_vars: Vec::new(),
            scoped_shadowed_count,
        }
    }

    /// Proactively create a shadow symbol for every free variable, storing them in a map.
    fn create_shadow_symbols<'a, T>(env: &GlobalEnv, free_vars: T) -> BTreeMap<Symbol, Symbol>
    where
        T: IntoIterator<Item = &'a Symbol>,
    {
        free_vars
            .into_iter()
            .map(|var| (*var, ShadowStack::create_shadow_symbol(env, var)))
            .collect()
    }

    /// Returns a shadow symbol sym' for sym which should be distinct from any user-definable vars.
    fn create_shadow_symbol(env: &GlobalEnv, sym: &Symbol) -> Symbol {
        let pool = env.symbol_pool();
        let shadow_name = (*pool.string(*sym)).clone() + "'";
        pool.make(&shadow_name)
    }

    /// If a var is a free variable which is currently shadowed, then gets the shadow variable;
    /// otherwise (not a free variable or not shadowed) returns None.
    ///
    /// If entering_scope, then the free variable is rewritten even if we're not yet in a scope,
    /// since we are about to enter one.
    pub fn get_shadow_symbol(&mut self, sym: Symbol, entering_scope: bool) -> Option<Symbol> {
        if self
            .scoped_shadowed_count
            .get(&sym)
            .map(|count| if entering_scope { *count + 1 } else { *count })
            .unwrap_or(0) // Not a free variable.
            > 0
        {
            let new_sym = self.shadow_symbols.get(&sym).expect(
                "Invariant violation: Shadow symbol not found in ShadowStack::get_shadow_symbol",
            );
            Some(*new_sym)
        } else {
            None
        }
    }

    /// Record that the provided symbols have local definitions, so should be shadowed.
    pub fn enter_scope<T>(&mut self, entering_vars: T)
    where
        T: IntoIterator<Item = Symbol>,
    {
        let entering_free_vars: Vec<Symbol> = entering_vars
            .into_iter()
            .filter(|s| self.shadow_symbols.contains_key(s))
            .collect();
        for free_var in &entering_free_vars {
            *self
                .scoped_shadowed_count
                .get_mut(free_var)
                .expect("Invariant violation: Free var not found in ShadowStack::enter_scope") += 1;
        }
        self.scoped_shadowed_vars.push(entering_free_vars);
    }

    /// Record that the provided symbols have local definitions, so should be shadowed.
    /// In this case, shadowed variables have already been renamed, so they must be mapped back.
    pub fn enter_scope_after_renaming<'a>(
        &mut self,
        entering_vars: impl Iterator<Item = &'a Symbol>,
    ) {
        let entering_free_vars: Vec<Symbol> = entering_vars
            .filter_map(|sym| self.shadow_symbols_inverse.get(sym))
            .cloned()
            .collect();
        self.enter_scope(entering_free_vars);
    }

    /// Unshadow the set of symbols from the most recent scope which has been entered and not exited
    /// yet.
    pub fn exit_scope(&mut self) {
        let exiting_free_vars = self
            .scoped_shadowed_vars
            .pop()
            .expect("Scope misalignment in inlining (too many scope exits).");
        for free_var in exiting_free_vars {
            *self
                .scoped_shadowed_count
                .get_mut(&free_var)
                .expect("Invariant violation: Free var not found in ShadowStack::exit_scope") -= 1;
        }
    }
}

/// `InlinedRewriter` transforms an inlined call into an expression to use in place of the call.  It
/// implements `ExpRewriterFunctions` to implement `rewrite_exp` which processes the inline function
/// body to substitute lambda-expression arguments in place, while rewriting variables in the
/// original body to avoid conflicts with the free variables in those lambda expressions.
/// The entry point is function `inline_call`, which processes parameters, rewrites the body,
/// and then uses function `construct_inlined_call_expression` to build the final expression to
/// substitute for the call; this function is also used for lambda expressions.  Various helper
/// functions convert `Tuple` patterns to/from variable lists as needed for different AST expressions.
struct InlinedRewriter<'env, 'rewriter> {
    env: &'env GlobalEnv,
    type_args: &'rewriter Vec<Type>,
    lambda_param_map: BTreeMap<Symbol, &'rewriter Exp>,
    inlined_formal_params: Vec<Parameter>,

    /// Shadow stack tracks whether free variables are hidden by local variable declarations.
    shadow_stack: ShadowStack,

    /// Track loop nesting, 0 outside a loop
    in_loop: usize,
    call_site_loc: &'rewriter Loc,
    /// Track whether in spec context during rewriting
    in_spec: usize,
    /// Map from parameter position to corresponding closure exp
    function_value_map: BTreeMap<usize, Exp>,
    /// Map from parameter position to corresponding spec function
    function_value_spec_map: BTreeMap<usize, (QualifiedId<SpecFunId>, QualifiedId<FunId>)>,
    /// Map from symbol to parameter pos
    sym_param_map: BTreeMap<Symbol, usize>,
    /// Whether to rewrite invoke for spec
    rewrite_invoke_for_spec: bool,
    /// Specializations for spec functions called with lambda-bound function
    /// parameters as arguments.
    spec_fun_specs: BTreeMap<SpecFunSpecKey, SpecFunSpecialization>,
    /// The function into which the expansion happens.
    target_fun: Option<QualifiedFunId>,
    /// For each function parameter with a unique application in the body,
    /// the state anchor label bound at that application site (verify mode).
    application_anchors: BTreeMap<Symbol, MemoryLabel>,
    /// The kind of the spec condition currently being rewritten, if any.
    current_condition_kind: Option<ConditionKind>,
    /// Whether the current loop invariant contains `folds_of`.
    current_condition_has_folds_of: bool,
    /// Whether an unresolved behavioral predicate occurred in the current
    /// spec block. Its behavioral loop invariants must be weakened together:
    /// a `folds_of` invariant can depend on an unresolved `ensures_of`
    /// invariant from the same inline expansion.
    unresolved_behavior_in_spec: bool,
    /// The resolutions of `folds_of` predicates over lambda-bound
    /// parameters, keyed by the predicate's node id (verify mode; see
    /// `resolve_folds_of_occurrences`).
    folds_of_resolutions: BTreeMap<NodeId, FoldsOfResolution>,
}

impl<'env, 'rewriter> InlinedRewriter<'env, 'rewriter> {
    fn lift_lambda_and_generate_spec_fun(
        env: &mut GlobalEnv,
        lift_inline_funs: bool,
        target_qualified_fun_id_opt: Option<QualifiedFunId>,
        lambda_args_matched: &[((usize, &Parameter), &Exp)],
    ) -> (
        BTreeMap<usize, Exp>,
        BTreeMap<Symbol, usize>,
        BTreeMap<usize, (QualifiedId<SpecFunId>, QualifiedId<FunId>)>,
    ) {
        let mut function_value_map: BTreeMap<usize, Exp> = BTreeMap::new();
        let mut sym_param_map: BTreeMap<Symbol, usize> = BTreeMap::new();
        let mut function_value_spec_map = BTreeMap::new();

        if lift_inline_funs && let Some(target_qualified_fun_id) = target_qualified_fun_id_opt {
            let mut lifted_lambda_funs: BTreeMap<usize, move_model::model::FunctionData> =
                BTreeMap::new();
            let options = LambdaLiftingOptions {
                include_inline_functions: true,
            };
            let fun_env = env.get_function(target_qualified_fun_id);
            for (para, lambda) in lambda_args_matched.iter().copied() {
                let mut lifter = LambdaLifter::new(
                    &options,
                    &fun_env,
                    Some(format!(
                        "_inline_{}_{}",
                        para.0,
                        env.get_node_loc(lambda.node_id()).span().start()
                    )),
                );
                let closure_exp = lifter.rewrite_exp(lambda.clone().clone());
                // Lifting may fail (e.g., modified captured variable in lambda).
                // Skip this lambda and fall back to non-lifted inlining.
                if lifter.lifted_len() != 1 {
                    continue;
                }
                let func_data = lifter.get_lifted_at(0).unwrap().generate_function_data(env);
                sym_param_map.insert(para.1 .0, para.0);
                function_value_map.insert(para.0, closure_exp.clone());
                lifted_lambda_funs.insert(para.0, func_data);
            }
            function_value_spec_map = run_spec_rewriter_inline(
                env,
                target_qualified_fun_id.module_id,
                lifted_lambda_funs,
            );
        }

        (function_value_map, sym_param_map, function_value_spec_map)
    }

    fn new(
        env: &'env GlobalEnv,
        type_args: &'rewriter Vec<Type>,
        inlined_formal_params: Vec<Parameter>,
        lambda_param_map: BTreeMap<Symbol, &'rewriter Exp>,
        lambda_free_vars: BTreeSet<Symbol>,
        call_site_loc: &'rewriter Loc,
        function_value_map: BTreeMap<usize, Exp>,
        function_value_spec_map: BTreeMap<usize, (QualifiedId<SpecFunId>, QualifiedId<FunId>)>,
        sym_param_map: BTreeMap<Symbol, usize>,
        rewrite_invoke_for_spec: bool,
        spec_fun_specs: BTreeMap<SpecFunSpecKey, SpecFunSpecialization>,
        target_fun: Option<QualifiedFunId>,
        application_anchors: BTreeMap<Symbol, MemoryLabel>,
        folds_of_resolutions: BTreeMap<NodeId, FoldsOfResolution>,
    ) -> Self {
        let shadow_stack = ShadowStack::new(env, &lambda_free_vars);
        Self {
            env,
            type_args,
            lambda_param_map,
            inlined_formal_params,
            shadow_stack,
            in_loop: 0,
            call_site_loc,
            in_spec: 0,
            function_value_map,
            function_value_spec_map,
            sym_param_map,
            rewrite_invoke_for_spec,
            spec_fun_specs,
            target_fun,
            application_anchors,
            current_condition_kind: None,
            current_condition_has_folds_of: false,
            unresolved_behavior_in_spec: false,
            folds_of_resolutions,
        }
    }

    /// Entry point for rewriting a call to an inline function.
    fn inline_call(
        env: &'env mut GlobalEnv,
        call_node_id: NodeId,
        func_loc: &Loc,
        body: &Exp,
        type_args: Vec<Type>,
        parameters: Vec<Parameter>,
        args: &[Exp],
        lift_inline_funs: bool,
        target_qualified_fun_id_opt: Option<QualifiedFunId>,
        spec_fun_unifier: &mut Vec<SpecFunUnifierEntry>,
        folds_of_unifier: &mut Vec<FoldsOfRecursionEntry>,
        inline_call_summary: Option<InlineCallSummarySpec>,
    ) -> Exp {
        let body = body.clone();
        let body = if env.is_verify_mode() {
            freshen_folds_anchor_labels(env, body)
        } else {
            body
        };
        // Preserve inline-entry `old(parameter)` semantics.
        let body = if env.is_verify_mode() {
            anchor_param_old_at_expansion_entry(
                env,
                body,
                &parameters,
                &env.get_node_loc(call_node_id),
            )
        } else {
            body
        };
        let args_matched: Vec<_> = zip(parameters.iter().enumerate(), args).collect();
        let (lambda_args_matched, regular_args_matched): (Vec<_>, Vec<_>) = args_matched
            .iter()
            .partition(|(_, arg)| matches!(arg.as_ref(), ExpData::Lambda(..)));
        let non_lambda_function_args =
            regular_args_matched.iter().filter_map(|(param, arg_exp)| {
                if matches!(param.1 .1, Type::Fun(..)) {
                    Some(arg_exp)
                } else {
                    None
                }
            });

        for arg_exp in non_lambda_function_args {
            env.error(
                &env.get_node_loc(arg_exp.as_ref().node_id()),
                "Currently, a function-typed parameter to an inline function \
                 must be a literal lambda expression",
            );
        }

        let lambda_param_map: BTreeMap<Symbol, &Exp> = lambda_args_matched
            .iter()
            .map(|(param, arg_exp)| (param.1 .0, *arg_exp))
            .collect();

        // Specialize spec functions which are called with lambda-bound function
        // parameters as arguments (e.g. a recursive `spec_fold(f, ..)` in a
        // loop invariant); see `SpecFunSpecializer`.
        let spec_fun_specs = SpecFunSpecializer::run(
            env,
            &body,
            &type_args,
            &parameters,
            &lambda_param_map,
            target_qualified_fun_id_opt,
            spec_fun_unifier,
        );
        for ((target, _, _), spec) in &spec_fun_specs {
            let is_summary = inline_call_summary
                .as_ref()
                .is_some_and(|summary| *target == summary.result || *target == summary.aborts);
            if !is_summary {
                spec.underivable_behavior.set(None);
            }
        }
        let inline_call_summary = inline_call_summary.and_then(|summary| {
            let lambda_sym = parameters.get(1)?.0;
            let find = |qid| {
                spec_fun_specs
                    .iter()
                    .find_map(|((target, _, bindings), spec)| {
                        (*target == qid && bindings.as_slice() == [(0, lambda_sym)])
                            .then(|| spec.clone())
                            .filter(|spec| spec.underivable_behavior(env).is_none())
                    })
            };
            Some((find(summary.result)?, find(summary.aborts)?))
        });

        // Lift lambda expressions and generate corresponding spec functions
        let (function_value_map, sym_param_map, function_value_spec_map) =
            Self::lift_lambda_and_generate_spec_fun(
                env,
                lift_inline_funs,
                target_qualified_fun_id_opt,
                &lambda_args_matched,
            );

        let (regular_params, regular_actuals): (Vec<(usize, &Parameter)>, Vec<&Exp>) =
            regular_args_matched.into_iter().unzip();
        let regular_params = regular_params
            .into_iter()
            .map(|(_, para)| para)
            .collect_vec();

        // If a caller is provided, collect its parameter symbols.
        let caller_param_symbols = target_qualified_fun_id_opt.map(|caller| {
            env.get_function(caller)
                .get_parameters()
                .iter()
                .map(|p| p.0)
                .collect_vec()
        });
        // Find free variables across all lambda expr arguments.
        // Perhaps we could minimize changes if we tracked each lambda arg individually in the inlined
        // method and only rewrite the context of each inlined lambda, but that seems quite difficult.
        // Instead, just group all the free vars together and shadow them all.
        let all_lambda_free_vars: BTreeSet<_> = lambda_args_matched
            .iter()
            .flat_map(|(_, exp)| {
                // If a caller is provided, compute free vars and used params.
                if let Some(caller_param_symbols) = &caller_param_symbols {
                    exp.free_vars_and_used_params(caller_param_symbols)
                        .into_iter()
                } else {
                    exp.free_vars().into_iter()
                }
            })
            .collect();

        // While we're looking at the lambdas, check for Return in their bodies.
        for (_, lambda_body) in lambda_args_matched {
            Self::check_for_return_break_continue_in_lambda(env, lambda_body);
        }

        // Record free variables in the parameters.
        let regular_params_overlapping_free_vars: Vec<_> = regular_params
            .iter()
            .filter_map(|param| {
                if all_lambda_free_vars.contains(&param.0) {
                    Some(param.0)
                } else {
                    None
                }
            })
            .collect();

        let call_site_loc = env.get_node_loc(call_node_id);

        // In verify mode, function parameters applied at exactly one
        // non-repeating site in the body get a state anchor label: the
        // application site is wrapped with a `SaveStateAnchor` marker, so
        // behavioral predicates over lambdas with global state effects can
        // be anchored there. A single lexical site inside a loop is not
        // unique dynamically: reusing its label would overwrite the snapshot
        // on every iteration. The same is true for an application nested in
        // a forwarding lambda passed to a non-inline callee: that opaque
        // callee can invoke the lambda any number of times.
        let mut application_anchors = BTreeMap::new();
        if env.is_verify_mode() {
            // Merely being lexically nested in a lambda is not enough to
            // make the count opaque: a forwarding lambda passed to another
            // inline function is expanded and analyzed normally. Track local
            // aliases of lambdas too: a non-inline call can receive a
            // forwarding lambda through `let g = |x| f(x); callee(g)`.
            let mut lambda_aliases = BTreeMap::new();
            body.visit_pre_order(&mut |e| {
                if let ExpData::Block(_, pat, Some(binding), _) = e {
                    for (sym, bound_exp) in pat.vars_and_exprs(binding) {
                        let lambda_id = bound_exp.and_then(|bound_exp| match bound_exp.as_ref() {
                            ExpData::Lambda(id, ..) => Some(*id),
                            ExpData::LocalVar(_, sym) => lambda_aliases.get(sym).copied(),
                            _ => None,
                        });
                        if let Some(lambda_id) = lambda_id {
                            lambda_aliases.insert(sym, lambda_id);
                        }
                    }
                }
                true
            });
            // Record when one lambda forwards another lambda through a local
            // alias. For example, in `let g = |x| f(x); let h = || g`, an
            // opaque call of `h` may repeatedly invoke the distinct lambda
            // `g` it returns.
            let mut forwarded_lambdas: BTreeMap<NodeId, BTreeSet<NodeId>> = BTreeMap::new();
            body.visit_pre_order(&mut |e| {
                if let ExpData::Lambda(id, _, lambda_body, _, _) = e {
                    let mut forwarded = BTreeSet::new();
                    lambda_body.visit_pre_order(&mut |inner| {
                        if let ExpData::LocalVar(_, sym) = inner {
                            if let Some(lambda_id) = lambda_aliases.get(sym) {
                                forwarded.insert(*lambda_id);
                            }
                        }
                        true
                    });
                    if !forwarded.is_empty() {
                        forwarded_lambdas.insert(*id, forwarded);
                    }
                }
                true
            });
            let mut opaque_forwarding_lambdas = BTreeSet::new();
            body.visit_pre_order(&mut |e| {
                if let ExpData::Call(_, Operation::MoveFunction(mid, fid), args) = e {
                    if !env.get_function(mid.qualified(*fid)).is_inline() {
                        for arg in args {
                            match arg.as_ref() {
                                ExpData::Lambda(id, ..) => {
                                    opaque_forwarding_lambdas.insert(*id);
                                },
                                ExpData::LocalVar(_, sym) => {
                                    if let Some(lambda_id) = lambda_aliases.get(sym) {
                                        opaque_forwarding_lambdas.insert(*lambda_id);
                                    }
                                },
                                _ => {},
                            }
                        }
                    }
                }
                true
            });
            // Transitively mark forwarding lambdas reachable from a lambda
            // supplied to an opaque call. Each may be invoked repeatedly by
            // that call, even if its body is not syntactically nested in the
            // lambda passed as the call argument.
            let mut pending: Vec<_> = opaque_forwarding_lambdas.iter().copied().collect();
            while let Some(lambda_id) = pending.pop() {
                if let Some(forwarded) = forwarded_lambdas.get(&lambda_id) {
                    for forwarded_id in forwarded {
                        if opaque_forwarding_lambdas.insert(*forwarded_id) {
                            pending.push(*forwarded_id);
                        }
                    }
                }
            }
            let mut application_counts: BTreeMap<Symbol, (usize, bool)> = BTreeMap::new();
            let mut loop_depth = 0;
            let mut opaque_lambda_depth = 0;
            body.visit_pre_post(&mut |post, e| {
                match e {
                    ExpData::Loop(..) if !post => loop_depth += 1,
                    ExpData::Loop(..) if post => loop_depth -= 1,
                    ExpData::Lambda(id, ..) if !post && opaque_forwarding_lambdas.contains(id) => {
                        opaque_lambda_depth += 1;
                    },
                    ExpData::Lambda(id, ..) if post && opaque_forwarding_lambdas.contains(id) => {
                        opaque_lambda_depth -= 1;
                    },
                    ExpData::Invoke(_, target, _) if !post => {
                        if let Some(sym) = param_sym(target.as_ref(), &parameters) {
                            if lambda_param_map.contains_key(&sym) {
                                let (count, may_repeat) =
                                    application_counts.entry(sym).or_default();
                                *count += 1;
                                *may_repeat |= loop_depth > 0 || opaque_lambda_depth > 0;
                            }
                        }
                    },
                    _ => {},
                }
                true
            });
            for (sym, (count, may_repeat)) in application_counts {
                if count == 1 && !may_repeat {
                    application_anchors
                        .insert(sym, MemoryLabel::new(env.new_global_id().as_usize()));
                }
            }
        }

        // Resolve `folds_of` predicates over lambda-bound parameters: derive
        // each target lambda's capture-accumulator transformer and specialize
        // the fold recursion over it (see `resolve_folds_of_occurrences`).
        // Verify mode only; in regular compilation the unresolved predicates
        // reduce their conditions to `true`.
        let (folds_of_resolutions, folds_of_deferral) = if env.is_verify_mode() {
            resolve_folds_of_occurrences(
                env,
                &body,
                &type_args,
                &parameters,
                args,
                &lambda_param_map,
                &all_lambda_free_vars,
                target_qualified_fun_id_opt,
                &call_site_loc,
                spec_fun_unifier,
                folds_of_unifier,
            )
        } else {
            (BTreeMap::new(), FoldsOfDeferralState::default())
        };
        // rewrite body with type_args, lambda params, and var renames to keep lambda free vars
        // free.
        let mut rewriter = InlinedRewriter::new(
            env,
            &type_args,
            parameters.clone(),
            lambda_param_map,
            all_lambda_free_vars,
            &call_site_loc,
            function_value_map,
            function_value_spec_map,
            sym_param_map,
            lift_inline_funs,
            spec_fun_specs,
            target_qualified_fun_id_opt,
            application_anchors,
            folds_of_resolutions,
        );

        // For now, just copy the actuals.  If FreezeRef is needed, we'll do it in
        // construct_inlined_call_expression.
        let rewritten_actuals: Vec<Exp> = regular_actuals.into_iter().cloned().collect();

        // Turn list of parameters into a pattern.  Also rewrite types as needed.
        // Shadow param vars as if we are in a let.
        let params_pattern =
            rewriter.parameter_list_to_pattern(env, func_loc, &call_site_loc, regular_params);

        // Enter the scope defined by the params.
        rewriter.shadowing_enter_scope(regular_params_overlapping_free_vars);

        // Rewrite body types, shadowed vars, replace invoked lambda params, etc.
        let rewritten_body = rewriter.rewrite_exp(body.clone());

        // Retain only anchors referenced after this expansion.
        let has_markers = rewritten_body
            .any(&mut |e| matches!(e, ExpData::Call(_, Operation::FoldsCaptureAnchor(..), _)));
        let rewritten_body = if has_markers {
            prune_folds_anchor_markers(rewritten_body)
        } else {
            rewritten_body
        };
        let rewritten_body = if let Some(label) = folds_of_deferral.entry_label {
            prepend_folds_anchor_marker(env, &call_site_loc, rewritten_body, label)
        } else {
            rewritten_body
        };

        let bound: BTreeSet<_> = params_pattern
            .vars()
            .into_iter()
            .map(|(_, sym)| sym)
            .collect();
        let (mutated_free_vars, mutated_temps) =
            spec_derivation::collect_mutated_free_vars_and_temps(env, &rewritten_body, &bound);
        // Summaries omit memory and capture effects.
        let inline_call_summary = inline_call_summary.filter(|_| {
            spec_derivation::exp_has_no_memory_effects(env, &rewritten_body)
                && mutated_free_vars.is_empty()
                && mutated_temps.is_empty()
        });
        let rewritten_body = if let Some((result_spec, aborts_spec)) = inline_call_summary {
            let self_sym = params_pattern.vars().first().map(|(_, sym)| *sym);
            if let (Some(self_sym), Some(Parameter(_, self_ty, _))) = (self_sym, parameters.first())
            {
                let self_ty = self_ty.skip_reference().instantiate(&type_args);
                let self_value =
                    ExpData::LocalVar(env.new_node(call_site_loc.clone(), self_ty), self_sym)
                        .into_exp();
                let len = ExpData::Call(
                    env.new_node(call_site_loc.clone(), Type::Primitive(PrimitiveType::U64)),
                    Operation::Len,
                    vec![self_value.clone()],
                )
                .into_exp();
                let result = result_spec.make_call(
                    env,
                    call_site_loc.clone(),
                    &MemoryRange::default(),
                    vec![self_value.clone(), len.clone()],
                    &BTreeMap::new(),
                );
                let aborts = aborts_spec.make_call(
                    env,
                    call_site_loc.clone(),
                    &MemoryRange::default(),
                    vec![self_value, len],
                    &BTreeMap::new(),
                );
                prepend_inline_call_summary(env, &call_site_loc, rewritten_body, result, aborts)
            } else {
                rewritten_body
            }
        } else {
            rewritten_body
        };

        InlinedRewriter::construct_inlined_call_expression(
            env,
            &call_site_loc,
            rewritten_body,
            params_pattern,
            rewritten_actuals,
        )
    }

    /// Enter a scope for parameters when inlining a call.  If any `entering_vars`
    /// are free variables tracked by `self.shadow_stack`, then note that they
    /// should be rewritten.
    fn shadowing_enter_scope(&mut self, entering_vars: Vec<Symbol>) {
        self.shadow_stack.enter_scope(entering_vars);
    }

    /// Check for and warn about Return inside a lambda.
    /// Also check for Break or Continue inside a lambda and not inside a loop.
    fn check_for_return_break_continue_in_lambda(env: &GlobalEnv, lambda_body: &Exp) {
        let mut in_loop = 0;
        lambda_body.visit_pre_post(&mut |post, e| {
            match e {
                ExpData::Loop(..) if !post => {
                    in_loop += 1;
                },
                ExpData::Loop(..) if post => {
                    in_loop -= 1;
                },
                ExpData::Return(node_id, _) if !post => {
                    let node_loc = env.get_node_loc(*node_id);
                    env.error(
                        &node_loc,
                        "Return not currently supported in function-typed arguments \
                         (lambda expressions)",
                    )
                },
                ExpData::LoopCont(node_id, _, is_continue) if !post && in_loop == 0 => {
                    let node_loc = env.get_node_loc(*node_id);
                    env.error(
                        &node_loc,
                        &format!(
                            "{} outside of a loop not supported in function-typed arguments \
                             (lambda expressions)",
                            if *is_continue { "Continue" } else { "Break" }
                        ),
                    )
                },
                _ => {},
            }
            true // keep going
        });
    }

    /// Convert a list of Parameters into a Pattern.
    /// Check for conflicts between lambda_free_vars and symbols in Parameters,
    /// replacing them by shadow symbols.
    /// Also remap types according to type_param_map as needed.
    fn parameter_list_to_pattern(
        &mut self,
        env: &'env GlobalEnv,
        function_loc: &Loc,
        call_site_loc: &Loc,
        parameters: Vec<&Parameter>,
    ) -> Pattern {
        let tuple_args: Vec<Pattern> = parameters
            .iter()
            .map(|param| {
                let Parameter(sym, ty, loc) = *param;
                let id = env.new_node(loc.clone(), ty.instantiate(self.type_args));
                if env.symbol_pool().string(*sym).as_ref() == "_" {
                    Pattern::Wildcard(id)
                } else if let Some(new_sym) = self.shadow_stack.get_shadow_symbol(*sym, true) {
                    Pattern::Var(id, new_sym)
                } else {
                    Pattern::Var(id, *sym)
                }
            })
            .collect();
        let tuple_type_list: Vec<Type> = parameters
            .iter()
            .map(|param| param.1.instantiate(self.type_args))
            .collect();
        let tuple_type: Type = Type::Tuple(tuple_type_list);
        let id = env.new_node(function_loc.clone().inlined_from(call_site_loc), tuple_type);
        Pattern::Tuple(id, tuple_args)
    }

    /// Build an expression corresponding to an inlined function (either lambda or inline function),
    /// essentially equivalent to { let pattern=args; body }.
    ///
    /// Body should already have types rewritten, other inlining complete, lambdas inlined, etc.  All
    /// types in args, body, parameters should also be rewritten (type params instantiated) as
    /// necessary.  parameters and args should be only non-lambda regular ordinary values (not
    /// types).
    fn construct_inlined_call_expression(
        env: &'env GlobalEnv,
        call_site_loc: &Loc,
        body: Exp,
        pattern: Pattern,
        args: Vec<Exp>,
    ) -> Exp {
        // Process Body
        let body_node_id = body.as_ref().node_id();
        let body_type = env.get_node_type(body_node_id);
        let body_loc = env
            .get_node_loc(body_node_id)
            .clone()
            .inlined_from(call_site_loc);

        let new_body_id = env.new_node(body_loc.clone(), body_type.clone());

        let pattern_type = env.get_node_type(pattern.node_id());

        let optional_new_args_expr = if args.is_empty() {
            None
        } else {
            let args_node_ids: Vec<NodeId> =
                args.iter().map(|exp| exp.as_ref().node_id()).collect();
            let mut args_types: Vec<Type> = args_node_ids
                .iter()
                .map(|node_id| env.get_node_type(*node_id))
                .collect();

            // Insert FreezeRef in args if needed
            let freezes_needed = InlinedRewriter::check_pattern_args_types_need_freezeref(
                &pattern_type,
                &args_types,
            );
            let rewritten_args: Vec<Exp> = if let Some(freeze_needed_vec) = freezes_needed {
                let (new_args_exps, new_args_types) = args
                    .iter()
                    .zip(freeze_needed_vec)
                    .map(|(exp, freeze_needed)| {
                        if freeze_needed {
                            let exp_node = exp.as_ref().node_id();
                            let exp_type = env.get_node_type(exp_node);
                            let new_type = if let Type::Reference(_refkind, box_type) = exp_type {
                                Type::Reference(ReferenceKind::Immutable, box_type.clone())
                            } else {
                                unreachable!("Should have been checked before");
                            };
                            let exp_loc = env.get_node_loc(exp_node);
                            let new_node = env.new_node(exp_loc, new_type.clone());
                            let new_exp_vec: Vec<Exp> = vec![exp.clone()];
                            (
                                Exp::from(ExpData::Call(
                                    new_node,
                                    Operation::Freeze(false),
                                    new_exp_vec,
                                )),
                                new_type,
                            )
                        } else {
                            (exp.clone(), env.get_node_type(exp.as_ref().node_id()))
                        }
                    })
                    .unzip();
                args_types = new_args_types;
                new_args_exps
            } else {
                args
            };

            let args_type = Type::Tuple(args_types);

            // TODO: try to find a more precise source code location corresponding to set of actual arguments.
            // E.g.,:
            //   let args_locs: Vec<Loc> = args_node_ids.iter().map(|node_id| env.get_node_loc(*node_id)).collect();
            //   let args_loc: Loc = Loc::merge(Vec<Loc>); or something  similar
            // For now, we just use the location of the first arg for the entire list.
            let args_loc = args_node_ids
                .first()
                .map(|node_id| env.get_node_loc(*node_id))
                .unwrap_or_else(|| call_site_loc.clone());

            let new_args_id = env.new_node(args_loc, args_type);
            let new_args_expr =
                ExpData::Call(new_args_id, Operation::Tuple, rewritten_args).into_exp();
            Some(new_args_expr)
        };

        let new_body = ExpData::Block(new_body_id, pattern, optional_new_args_expr, body);
        new_body.into_exp()
    }

    /// If `pattern-type` is a tuple of same length as `arg_vec`, and types differ just in mutability
    /// of the reference type, where the param is immutable and the arg is mutable, returns
    /// `Some(vec)` where such corresponding elements are true, indicating that a `FreezeRef` could
    /// be inserted to gain type compatibility.
    ///
    /// If there are no such parameters, returns None.
    ///
    /// (Helper for construct_inlined_call_expression.)
    fn check_pattern_args_types_need_freezeref(
        pattern_type: &Type,
        args_types: &Vec<Type>,
    ) -> Option<Vec<bool>> {
        match pattern_type {
            Type::Tuple(type_vec) => {
                InlinedRewriter::check_params_args_types_vectors_need_freezeref(
                    type_vec, args_types,
                )
            },
            _ => None,
        }
    }

    /// If any corresponding elements of `param_vec` and `arg_vec` differ just in mutability of the
    /// reference type, where the param is immutable and the arg is mutable, returns `Some(vec)`
    /// where such corresponding elements are true, indicating that a `FreezeRef` could be inserted
    /// to gain type compatibility.
    ///
    /// If there are no such parameters, returns None.
    ///
    /// (Helper for check_pattern_args_types_need_freezeref)
    fn check_params_args_types_vectors_need_freezeref(
        params_types: &[Type],
        args_types: &Vec<Type>,
    ) -> Option<Vec<bool>> {
        // element is Some(true) if a FreezeRef is needed, Some(false) if not, and None if types
        // are incompatible.
        if params_types.len() != args_types.len() {
            None
        } else {
            let compare_pairs: Vec<bool> = params_types
                .iter()
                .zip(args_types)
                .map(|(t1, t2)| {
                    if *t1 == *t2 {
                        false
                    } else if let (Type::Reference(kind1, box_t1), Type::Reference(kind2, box_t2)) =
                        (t1, t2)
                    {
                        *box_t1 == *box_t2
                            && *kind1 == ReferenceKind::Immutable
                            && *kind2 == ReferenceKind::Mutable
                    } else {
                        false
                    }
                })
                .collect();
            if compare_pairs.iter().all(|x| !x) {
                None
            } else {
                Some(compare_pairs)
            }
        }
    }

    /// Resolves an expression referencing a function parameter to the lambda
    /// argument it is bound to.
    fn resolve_lambda_target(&self, exp: &Exp) -> Option<&'rewriter Exp> {
        param_sym(exp.as_ref(), &self.inlined_formal_params)
            .and_then(|sym| self.lambda_param_map.get(&sym).copied())
    }

    /// If `exp` is a behavioral predicate whose target is a function parameter
    /// bound to a literal lambda argument, replaces it by the lambda's spec
    /// conditions (or a form derived from the lambda's body), with the lambda's
    /// parameters substituted by the predicate's arguments. This makes spec
    /// blocks in inline function bodies (in particular loop invariants) which
    /// constrain the behavior of function parameters verifiable at each
    /// expansion site, against the concrete lambda supplied there.
    fn try_inline_behavior_predicate(&mut self, exp: &Exp) -> Option<Exp> {
        let ExpData::Call(id, Operation::Behavior(kind, range), args) = exp.as_ref() else {
            return None;
        };
        let lambda = self.resolve_lambda_target(args.first()?)?.clone();
        // Loop invariants are verifier-only. Skip lambda-spec derivation in
        // regular builds.
        if !self.env.is_verify_mode()
            && matches!(
                self.current_condition_kind,
                Some(ConditionKind::LoopInvariant)
            )
        {
            return Some(exp.clone());
        }
        // Rewrite the predicate arguments in the regular way; the lambda's spec
        // material spliced below is caller scope and left untouched.
        let bp_args: Vec<Exp> = args[1..]
            .iter()
            .map(|arg| self.rewrite_exp(arg.clone()))
            .collect();
        let new_id = self.rewrite_node_id(*id).unwrap_or(*id);
        let anchor = param_sym(args[0].as_ref(), &self.inlined_formal_params)
            .and_then(|sym| self.application_anchors.get(&sym).copied());
        let context = if matches!(
            self.current_condition_kind,
            Some(ConditionKind::LoopInvariant)
        ) {
            BpContext::LoopInvariant
        } else {
            BpContext::Plain
        };
        let result = substitute_bp_by_lambda_spec(
            self.env,
            self.target_fun,
            anchor,
            context,
            context == BpContext::LoopInvariant,
            new_id,
            *kind,
            range,
            &lambda,
            bp_args,
            self.folds_of_resolutions.get(id),
        );
        if context == BpContext::LoopInvariant {
            if let Some(kind) = underivable_concrete_behavior(self.env, &result) {
                warn_underivable_concrete_behavior(self.env, &self.env.get_node_loc(new_id), kind);
                return Some(self.weaken_or_mark_unresolved(result));
            }
            if uses_generic_type_reflection(self.env, &result)
                || lambda_uses_generic_type_reflection(self.env, &lambda)
                || behavior_uses_generic_type_reflection(self.env, &result)
            {
                warn_generic_type_reflection_behavior(self.env, &self.env.get_node_loc(new_id));
                return Some(self.weaken_or_mark_unresolved(result));
            }
        }
        // Values in an inlined behavioral predicate can be related to their
        // snapshots only by Move equality. Record the spec functions used by
        // the substituted material so the backend emits congruence only for
        // those functions, instead of globally for every uninterpreted spec
        // function in the program.
        self.env.mark_move_equality_congruence_spec_funs_in(&result);
        // An intact predicate over a literal lambda signals failed resolution.
        if matches!(
            result.as_ref(),
            ExpData::Call(_, Operation::Behavior(..), args)
                if args.first().is_some_and(|t| matches!(t.as_ref(), ExpData::Lambda(..)))
        ) {
            return Some(self.weaken_or_mark_unresolved(result));
        }
        Some(result)
    }

    /// If `exp` is a call to a spec function with lambda-bound function
    /// parameters as arguments, redirects it to the specialization generated
    /// by `SpecFunSpecializer`, dropping the lambda arguments and appending
    /// the context arguments.
    fn try_specialize_spec_fun_call(&mut self, exp: &Exp) -> Option<Exp> {
        let ExpData::Call(id, Operation::SpecFunction(mid, sid, range), args) = exp.as_ref() else {
            return None;
        };
        let bindings =
            collect_lambda_bindings(args, &self.inlined_formal_params, &self.lambda_param_map);
        if bindings.is_empty() {
            return None;
        }
        let inst = self.instantiate_types(self.env.get_node_instantiation(*id));
        let key = (
            mid.qualified(*sid),
            inst,
            bindings.iter().map(|(pos, sym, _)| (*pos, *sym)).collect(),
        );
        let loc = self.env.get_node_loc(*id).inlined_from(self.call_site_loc);
        let Some(spec) = self.spec_fun_specs.get(&key).cloned() else {
            // Specialization failed; an error has been reported (in verify
            // mode). Substituting a constant for the call would be ill-typed
            // for a non-bool result type, and the lambda-bound parameters do
            // not survive the expansion: resolve the parameters to their
            // literal lambdas and leave the call otherwise intact, matching
            // the form the literal-lambda path leaves unchanged on failure.
            let lambda_args: BTreeMap<usize, Exp> = bindings
                .into_iter()
                .map(|(pos, _, lambda)| (pos, lambda))
                .collect();
            let new_args: Vec<Exp> = args
                .iter()
                .enumerate()
                .map(|(pos, arg)| match lambda_args.get(&pos) {
                    // The lambda is caller-scope material and is not rewritten.
                    Some(lambda) => lambda.clone(),
                    None => self.rewrite_exp(arg.clone()),
                })
                .collect();
            let new_id = self.rewrite_node_id(*id).unwrap_or(*id);
            return Some(
                ExpData::Call(
                    new_id,
                    Operation::SpecFunction(*mid, *sid, range.clone()),
                    new_args,
                )
                .into_exp(),
            );
        };
        let retained_args: Vec<Exp> = spec
            .retained
            .iter()
            .map(|pos| self.rewrite_exp(args[*pos].clone()))
            .collect();
        if matches!(
            self.current_condition_kind,
            Some(ConditionKind::LoopInvariant)
        ) {
            if let Some(kind) = spec.underivable_behavior(self.env) {
                warn_underivable_concrete_behavior(self.env, &loc, kind);
                let result = spec.make_call(self.env, loc, range, retained_args, &BTreeMap::new());
                return Some(self.weaken_or_mark_unresolved(result));
            }
        }
        Some(spec.make_call(self.env, loc, range, retained_args, &BTreeMap::new()))
    }

    fn weaken_or_mark_unresolved(&mut self, exp: Exp) -> Exp {
        self.unresolved_behavior_in_spec = true;
        weaken_or_mark_unresolved(self.env, exp)
    }

    /// Instantiates the given types with the type arguments of this expansion.
    fn instantiate_types(&self, tys: Vec<Type>) -> Vec<Type> {
        if self.type_args.is_empty() {
            tys
        } else {
            Type::instantiate_vec(tys, self.type_args)
        }
    }
}

/// Convert any non-`Tuple` pattern `pat` into a a singleton `Pattern::Tuple` if needed,
/// for convenience in matching it to a `Tuple` of expressions.
fn make_lambda_pattern_a_tuple(env: &GlobalEnv, pat: &Pattern) -> Pattern {
    if !matches!(pat, Pattern::Tuple(..)) {
        let id = pat.node_id();
        let new_id = env.new_node(
            env.get_node_loc(id),
            Type::Tuple(vec![env.get_node_type(id)]),
        );
        Pattern::Tuple(new_id, vec![pat.clone()])
    } else {
        pat.clone()
    }
}

/// Helpers for constructing boolean spec expressions.
trait BoolExpBuilder {
    fn new_bool_node(&self, loc: &Loc) -> NodeId;
    fn new_bool_const(&self, loc: &Loc, value: bool) -> Exp;
    fn new_bool_join(&self, loc: &Loc, oper: Operation, exps: Vec<Exp>, default: bool) -> Exp;
}

impl BoolExpBuilder for GlobalEnv {
    fn new_bool_node(&self, loc: &Loc) -> NodeId {
        self.new_node(loc.clone(), Type::Primitive(PrimitiveType::Bool))
    }

    fn new_bool_const(&self, loc: &Loc, value: bool) -> Exp {
        ExpData::Value(self.new_bool_node(loc), Value::Bool(value)).into_exp()
    }

    /// Joins `exps` with the given boolean operation, or returns `default` for
    /// an empty list.
    fn new_bool_join(&self, loc: &Loc, oper: Operation, exps: Vec<Exp>, default: bool) -> Exp {
        exps.into_iter()
            .reduce(|a, b| {
                ExpData::Call(self.new_bool_node(loc), oper.clone(), vec![a, b]).into_exp()
            })
            .unwrap_or_else(|| self.new_bool_const(loc, default))
    }
}

/// Reports a specification-related error in verify mode. In regular
/// compilation, spec contents are not enforced (they are dropped by code
/// generation), so the caller falls back silently instead.
fn spec_error(env: &GlobalEnv, loc: &Loc, msg: &str) {
    if env.is_verify_mode() {
        env.error(loc, msg);
    }
}

/// Like `spec_error`, with secondary labels.
fn spec_error_with_labels(env: &GlobalEnv, loc: &Loc, msg: &str, labels: Vec<(Loc, String)>) {
    if env.is_verify_mode() {
        env.diag_with_labels(Severity::Error, loc, msg, labels);
    }
}

/// The specification context into which a behavioral predicate over a lambda
/// is substituted. Determines how conditions referencing two states (from
/// global state effects of the lambda) are resolved.
#[derive(Clone, Copy, PartialEq, Eq)]
enum BpContext {
    /// A regular condition of the expansion: two-state conditions are
    /// anchored at the parameter's unique application site.
    Plain,
    /// A loop invariant: `old(..)` resolves to function entry, and memory
    /// effect operations are projected to point facts over the current state.
    LoopInvariant,
    /// The body of a specialized spec function: a single-state context
    /// without an application site or an `old(..)` scope; two-state
    /// conditions are rejected.
    SpecFunBody,
    /// A spec-function specialization used only to construct a fold
    /// transformer. Failure is propagated to `folds_of`, which warns and
    /// weakens its enclosing invariant instead of emitting a nested error.
    FoldTransformer,
}

/// Substitutes a behavioral predicate applied to the given lambda:
/// - `requires_of` becomes the conjunction of the lambda's `requires`
///   (`true` if there are none),
/// - `aborts_of` becomes the disjunction of the lambda's `aborts_if`
///   (`false` if the lambda has a spec without `aborts_if` conditions); for
///   a spec-less lambda, the abort condition is derived from the body's
///   operations (see `derive_aborts_condition`),
/// - `ensures_of` becomes the conjunction of the lambda's `ensures`, or,
///   for a spec-less lambda with a single result and no `&mut` parameters,
///   the exact relation `result_arg == <beta-reduced body>`,
/// - `result_of` becomes the functional `ensures result == E` from the
///   lambda's spec if it has this shape, otherwise the single derived
///   result value from the body, otherwise — for a pure, state-free body —
///   the beta-reduced body itself (covering shapes the derivation cannot
///   express as a single value, such as tuple-valued bodies),
/// - `folds_of`, in a loop invariant, becomes the fold equation and prefix
///   no-abort condition of the `folds_of` resolution recorded for this
///   occurrence by `resolve_folds_of_occurrences` (see there and
///   `build_folds_of_invariant`).
///
/// For a spec-less lambda writing captured variables of the enclosing
/// scope, the pointwise predicates follow capture-aware policies: the
/// cumulative capture effect is `folds_of` material, so `ensures_of` drops
/// the derived conjuncts mentioning the captures (a sound weakening), and
/// `aborts_of`/`result_of` — whose per-application values would have to
/// name a capture's evolving value — report an error pointing to
/// `folds_of`. `unchanged_of` and `requires_of` are unaffected (capture
/// writes do not touch global memory).
///
/// In the lambda's conditions, a parameter is substituted by the
/// predicate's corresponding input argument, `old(param)` of a `&mut`
/// parameter by the input argument and plain `param` by the post-state
/// argument (requiring the canonical dual-argument form of the predicate),
/// and `result` by the result arguments. Since the predicate's arguments
/// are state-independent data, the `old(..)` wrapper is dropped in the
/// substitution; `old(..)` over anything else than a lambda parameter is
/// not supported here.
fn substitute_bp_by_lambda_spec(
    env: &GlobalEnv,
    target_fun: Option<QualifiedFunId>,
    anchor: Option<MemoryLabel>,
    context: BpContext,
    weakenable_condition: bool,
    id: NodeId,
    kind: BehaviorKind,
    range: &MemoryRange,
    lambda: &Exp,
    bp_args: Vec<Exp>,
    folds_of: Option<&FoldsOfResolution>,
) -> Exp {
    let loc = env.get_node_loc(id);
    let lambda_loc = env.get_node_loc(lambda.node_id());
    // On failure, the predicate is left intact with the resolved lambda as
    // its target: an error has been reported (in verify mode), substituting
    // a boolean constant would be ill-typed for `result_of` over a non-bool
    // lambda, and the function parameter does not survive the expansion.
    // This matches the form the literal-lambda path of spec function
    // specialization leaves unchanged on failure.
    let intact = || -> Exp {
        let mut args = Vec::with_capacity(bp_args.len() + 1);
        args.push(lambda.clone());
        args.extend(bp_args.iter().cloned());
        ExpData::Call(id, Operation::Behavior(kind, range.clone()), args).into_exp()
    };
    let ExpData::Lambda(_, pat, lambda_body, _, spec_opt) = lambda.as_ref() else {
        env.diag(
            Severity::Bug,
            &loc,
            "invalid lambda target of behavioral predicate",
        );
        return intact();
    };
    if kind == BehaviorKind::FoldsOf {
        // Handled before the argument-layout split below, which follows the
        // target's parameter list and does not apply to `folds_of`'s
        // `(v, i)` / `(g, i)` arguments — and before the range check, since
        // a previously deferred occurrence carries its anchor label in the
        // range (see `FoldsOfDeferred`).
        if context != BpContext::LoopInvariant {
            spec_error_with_labels(
                env,
                &loc,
                "`folds_of` can only be used in a loop invariant",
                vec![(lambda_loc.clone(), "lambda argument".to_string())],
            );
            return intact();
        }
        let Some(resolution) = folds_of else {
            // In verify mode, the pre-pass (`resolve_folds_of_occurrences`)
            // has already reported why this occurrence could not be
            // resolved. Its summary cannot be used soundly, so remove this
            // predicate from the loop invariant in both verification and
            // regular compilation. Leaving it intact here relies on the
            // later unresolved-predicate pass, but that pass can no longer
            // identify this freshly rebuilt node after further inlining.
            return env.new_bool_const(&loc, true);
        };
        if bp_args.len() != 2 {
            // Arity errors have already been reported by type checking.
            return intact();
        }
        return match resolution {
            FoldsOfResolution::Direct(direct) => {
                build_folds_of_invariant(env, &loc, direct, &bp_args[0], &bp_args[1])
            },
            FoldsOfResolution::Deferred(deferred) => {
                build_folds_of_deferral(env, &loc, deferred, &bp_args[1])
            },
        };
    }
    if !range.is_default() {
        spec_error(
            env,
            &loc,
            "state labels are not supported on behavioral predicates \
                 over lambda arguments of inline functions",
        );
        return intact();
    }
    let Type::Fun(param_ty, result_ty, _) = env.get_node_type(lambda.node_id()) else {
        env.diag(
            Severity::Bug,
            &loc,
            "invalid type of lambda target of behavioral predicate",
        );
        return intact();
    };
    let param_tys = param_ty.flatten();
    let lambda_result_ty = (*result_ty).clone();
    let result_count = match &lambda_result_ty {
        Type::Tuple(tys) => tys.len(),
        _ => 1,
    };
    let tuple_pat = make_lambda_pattern_a_tuple(env, pat);
    let Pattern::Tuple(_, param_pats) = &tuple_pat else {
        unreachable!("lambda pattern normalized to tuple")
    };
    let mut param_syms: Vec<Symbol> = vec![];
    for (pos, p) in param_pats.iter().enumerate() {
        match p {
            Pattern::Var(_, sym) => param_syms.push(*sym),
            Pattern::Wildcard(_) => {
                // Wildcard parameters get fresh names so derived conditions
                // and post-state slots can refer to them.
                param_syms.push(
                    env.symbol_pool()
                        .make(&format!("$wp_p{}_{}", pos, id.as_usize())),
                );
            },
            _ => {
                spec_error(
                    env,
                    &env.get_node_loc(p.node_id()),
                    "lambdas with destructuring parameters are not supported \
                         with behavioral predicates",
                );
                return intact();
            },
        }
    }
    let param_count = param_tys.len();
    let mut_param_count = param_tys
        .iter()
        .filter(|ty| ty.is_mutable_reference())
        .count();

    // Split the predicate arguments into input, result, and `&mut`
    // post-state slots, following the argument layout established by
    // type checking (`compute_behavior_arg_types[_canonical]`).
    let expected_min = match kind {
        BehaviorKind::EnsuresOf => param_count + result_count,
        _ => param_count,
    };
    let has_posts = kind == BehaviorKind::EnsuresOf
        && mut_param_count > 0
        && bp_args.len() == expected_min + mut_param_count;
    if bp_args.len() != expected_min && !has_posts {
        // Arity errors have already been reported by type checking.
        return intact();
    }
    if kind == BehaviorKind::EnsuresOf && mut_param_count > 0 && !has_posts {
        spec_error_with_labels(
            env,
            &loc,
            "`ensures_of` over a lambda with `&mut` parameters requires the \
                 canonical form with explicit post-state arguments",
            vec![(lambda_loc.clone(), "lambda argument".to_string())],
        );
        return intact();
    }
    let inputs = &bp_args[0..param_count];
    let results = &bp_args[param_count..expected_min];
    let posts = &bp_args[expected_min..];

    // Build the substitution maps for the lambda's parameters: `pre` for
    // occurrences under `old(..)`, `curr` for plain occurrences.
    let mut curr = BTreeMap::new();
    let mut pre = BTreeMap::new();
    let mut post_iter = posts.iter();
    for ((sym, ty), input) in param_syms.iter().zip(&param_tys).zip(inputs) {
        let post = if ty.is_mutable_reference() {
            post_iter.next()
        } else {
            None
        };
        pre.insert(*sym, input.clone());
        curr.insert(*sym, post.unwrap_or(input).clone());
    }

    let lambda_spec = spec_opt.as_ref().and_then(|s| match s.as_ref() {
        ExpData::SpecBlock(_, spec) => Some(spec),
        _ => None,
    });
    // The lambda's mutated captures: free variables of the enclosing scope
    // its body writes, treated as implicit `&mut` parameters by the body
    // derivation. See the capture-aware policies in the function comment.
    let (mutated_captures, derivation_body) =
        prepare_mutated_captures(env, target_fun, lambda, lambda_body, &param_syms);
    // Derives a specification from the lambda's body via the source-level
    // weakest-precondition analysis, regardless of an attached spec. Only
    // invoked by the arms which actually consume the derivation.
    // Applications of the enclosing function's function-typed parameters
    // are deferred: their behavioral summaries stay label-free and
    // re-resolve when the enclosing function is itself expanded (or
    // translate directly for a genuine function-value parameter).
    let derive_from_body = || -> Option<DerivedSpec> {
        target_fun.and_then(|qid| {
            let fun_env = env.get_function(qid);
            let mut generator = FunExpGenerator::new(fun_env, loc.clone());
            let params: Vec<(Symbol, Type)> = param_syms
                .iter()
                .cloned()
                .zip(param_tys.iter().cloned())
                .collect();
            let mut var_types: BTreeMap<Symbol, Type> = params.iter().cloned().collect();
            for (sym, ty) in lambda.free_vars_with_types(env) {
                var_types.entry(sym).or_insert(ty);
            }
            for capture in &mutated_captures {
                var_types.entry(capture.sym).or_insert(capture.ty.clone());
            }
            let captures: Vec<(Symbol, Type)> = mutated_captures
                .iter()
                .map(|capture| (capture.sym, capture.ty.clone()))
                .collect();
            spec_derivation::derive_spec_with_captures(
                &mut generator,
                &params,
                &captures,
                &var_types,
                &lambda_result_ty,
                &derivation_body,
                &deferred_fun_param_temps(env, target_fun),
            )
        })
    };
    // For a spec-less lambda, derives a specification from its body; a
    // lambda with an attached spec is described by that spec instead.
    let derive = || -> Option<DerivedSpec> {
        if lambda_spec.is_some() {
            return None;
        }
        derive_from_body()
    };
    // Whether state-referencing `old(..)` is permitted in substituted
    // conditions: with an anchor it resolves to the application's pre-state;
    // in a loop invariant it resolves to function entry; in regular
    // compilation the conditions are dropped anyway.
    let allow_state_old =
        anchor.is_some() || context == BpContext::LoopInvariant || !env.is_verify_mode();
    let substitute_with = |exp: &Exp, allow_state_old: bool| {
        let mut subst = BpCondSubstituter {
            env,
            curr: &curr,
            pre: &pre,
            results,
            bp_loc: &loc,
            shadowed: vec![],
            allow_state_old,
            in_old: false,
        };
        subst.rewrite_exp(exp.clone())
    };
    let substitute = |exp: &Exp| substitute_with(exp, allow_state_old);
    let param_set: BTreeSet<Symbol> = param_syms.iter().copied().collect();
    // Resolves a substituted condition whose source (lambda spec or derived
    // conditions) references two states: in a loop invariant, its pre-state
    // resolves to function entry and whole-memory effect operations are
    // projected to point facts over the current state; otherwise the
    // condition is wrapped in the anchor of the parameter's unique
    // application site, or an error is reported.
    let finalize = |env: &GlobalEnv, needs_anchor: bool, cond: Exp| -> Exp {
        if !env.is_verify_mode() || !needs_anchor {
            return cond;
        }
        match context {
            BpContext::LoopInvariant => {
                let projected = project_effects_to_point_facts(env, cond);
                if let Some(msg) = loop_invariant_residual(&projected) {
                    env.diag_with_labels(
                        Severity::Warning,
                        &loc,
                        &format!(
                            "cannot derive `{}` exactly for this lambda \
                             argument: {}; weakening the enclosing loop \
                             invariant; see {}",
                            kind, msg, INLINE_HOF_WEAKENING_ISSUE
                        ),
                        vec![(lambda_loc.clone(), "lambda argument".to_string())],
                    );
                    return intact();
                }
                projected
            },
            BpContext::SpecFunBody => {
                spec_error_with_labels(
                    env,
                    &loc,
                    "a lambda with global state effects cannot be constrained \
                     in the body of a spec function",
                    vec![(lambda_loc.clone(), "lambda argument".to_string())],
                );
                cond
            },
            BpContext::FoldTransformer => cond,
            BpContext::Plain => {
                if let Some(label) = anchor {
                    ExpData::Call(
                        env.new_bool_node(&loc),
                        Operation::WithStateAnchor(label),
                        vec![cond],
                    )
                    .into_exp()
                } else {
                    spec_error_with_labels(
                        env,
                        &loc,
                        "the lambda has global state effects, which require a unique \
                         application of the function parameter in the inline function \
                         body to anchor the predicate's states",
                        vec![(lambda_loc.clone(), "lambda argument".to_string())],
                    );
                    cond
                }
            },
        }
    };
    // Substitutes conditions phrased over the application's pre-state
    // (`requires` and `aborts_if`): when they read global memory, the reads
    // are wrapped in `old(..)`, resolving at the anchor, or at function
    // entry in a loop invariant. Since every state read refers to the
    // application's pre-state, a state-reading condition needs the anchor
    // even without an `old(..)` of its own: without one it would silently
    // be evaluated at the assertion-site state. A spec function body is a
    // single-state context where the current state is the only meaningful
    // one.
    let pre_state_conditions = |source: Vec<&Exp>, join: Operation, empty: bool| -> Exp {
        let reads_state = source.iter().any(|c| reads_global_state(env, c));
        let wrap_old = env.is_verify_mode()
            && ((anchor.is_some() && context == BpContext::Plain)
                || context == BpContext::LoopInvariant)
            && reads_state;
        let needs_anchor = (reads_state
            && !matches!(context, BpContext::SpecFunBody | BpContext::FoldTransformer))
            || source
                .iter()
                .any(|c| condition_needs_anchor(env, c, &param_set));
        let conds = source
            .into_iter()
            .map(&substitute)
            .map(|c| {
                if wrap_old {
                    wrap_state_reads_in_old(env, c)
                } else {
                    c
                }
            })
            .collect();
        let joined = env.new_bool_join(&loc, join, conds, empty);
        finalize(env, needs_anchor, joined)
    };
    // A two-state spec function — one whose body (transitively) uses
    // `old(..)` — cannot appear in pre-state conditions: `requires` and
    // `aborts_if` describe the single pre-state of the application, so
    // there is no second state for the function's `old(..)` to refer to.
    // Reports an error and returns true if the source contains such a call.
    let reject_two_state_in_pre_state = |source: &[&Exp]| -> bool {
        if !env.is_verify_mode() || !source.iter().any(|c| calls_two_state_spec_fun(env, c)) {
            return false;
        }
        spec_error_with_labels(
            env,
            &loc,
            &format!(
                "the lambda's `{}` conditions call a two-state spec \
                 function (one using `old(..)`), but describe a single \
                 state",
                if kind == BehaviorKind::RequiresOf {
                    "requires"
                } else {
                    "aborts_if"
                }
            ),
            vec![(lambda_loc.clone(), "lambda argument".to_string())],
        );
        true
    };
    // The source conditions of the given kind for a predicate arm: from the
    // lambda's attached spec if present, otherwise from the derived spec.
    fn spec_or_derived<'a>(
        lambda_spec: Option<&'a Spec>,
        kind: ConditionKind,
        from_derived: Option<Vec<&'a Exp>>,
    ) -> Option<Vec<&'a Exp>> {
        if let Some(spec) = lambda_spec {
            Some(spec.filter_kind(kind).map(|c| &c.exp).collect())
        } else {
            from_derived
        }
    }

    match kind {
        BehaviorKind::RequiresOf => {
            if let Some(spec) = lambda_spec {
                // Requires conditions hold at the application's pre-state,
                // like abort conditions, and get the same state treatment:
                // without it, a state-reading `requires` would be evaluated
                // at the assertion-site state, letting a false `requires_of`
                // claim verify once memory changed in between.
                let source: Vec<&Exp> = spec
                    .filter_kind(ConditionKind::Requires)
                    .map(|c| &c.exp)
                    .collect();
                if reject_two_state_in_pre_state(&source) {
                    return intact();
                }
                pre_state_conditions(source, Operation::And, true)
            } else if let Some(msg) = requires_material_in_body(env, lambda_body) {
                // The lambda has precondition obligations of its own which
                // the derivation does not describe (`DerivedSpec::requires`
                // is reserved); substituting `true` would let a false
                // `requires_of` claim verify.
                spec_error_with_labels(env, &loc, &msg, vec![(
                    lambda_loc.clone(),
                    "lambda argument".to_string(),
                )]);
                intact()
            } else {
                // A lambda without callee-precondition material has no
                // precondition obligations of its own (abort behavior is
                // described by `aborts_of`).
                env.new_bool_const(&loc, true)
            }
        },
        BehaviorKind::AbortsOf => {
            if lambda_spec.is_none() && !mutated_captures.is_empty() {
                report_capture_writing_bp(env, &loc, &lambda_loc, kind, &mutated_captures);
                return intact();
            }
            let derived = derive();
            let source = spec_or_derived(
                lambda_spec,
                ConditionKind::AbortsIf,
                derived.as_ref().map(|d| d.aborts.iter().collect()),
            );
            if let Some(source) = source {
                // Abort conditions are phrased over the application's
                // pre-state and get the pre-state treatment.
                if reject_two_state_in_pre_state(&source) {
                    return intact();
                }
                pre_state_conditions(source, Operation::Or, false)
            } else {
                report_underivable_bp(env, &loc, &lambda_loc, kind, context);
                intact()
            }
        },
        BehaviorKind::EnsuresOf => {
            let inferred = env.symbol_pool().make(CONDITION_INFERRED_PROP);
            let has_explicit_ensures = lambda_spec.is_some_and(|spec| {
                spec.filter_kind(ConditionKind::Ensures)
                    .any(|cond| !cond.properties.contains_key(&inferred))
            });
            let has_reference_capture = lambda
                .free_vars_with_types(env)
                .into_iter()
                .any(|(_, ty)| ty.is_reference())
                || lambda_body
                    .used_temporaries_with_types(env)
                    .into_iter()
                    .any(|(_, ty)| ty.is_reference());
            if !has_explicit_ensures && mut_param_count > 0 && has_reference_capture {
                report_underivable_bp(env, &loc, &lambda_loc, kind, context);
                return intact();
            }
            let derived = derive();
            let mut source = spec_or_derived(
                lambda_spec,
                ConditionKind::Ensures,
                derived.as_ref().map(|d| d.ensures.iter().collect()),
            );
            // For a capture-writing lambda, drop the derived conjuncts
            // mentioning the captures: `ensures_of` has no way to name a
            // capture's pre-state, and the cumulative capture effect is
            // `folds_of` material. Dropping is a sound weakening.
            if lambda_spec.is_none() && !mutated_captures.is_empty() {
                let capture_syms: BTreeSet<Symbol> =
                    mutated_captures.iter().map(|capture| capture.sym).collect();
                source = source.map(|conds| {
                    conds
                        .into_iter()
                        .filter(|cond| cond.free_vars().is_disjoint(&capture_syms))
                        .collect()
                });
            }
            if let Some(source) = source {
                let needs_anchor = source
                    .iter()
                    .any(|c| condition_needs_anchor(env, c, &param_set));
                // If the only failure is a missing dynamic anchor, preserve
                // stateful `old(..)` material through substitution so the
                // primary anchor diagnostic is not followed by secondary
                // "old can only be applied" errors. Verification already
                // stops on that primary error.
                let conds = source
                    .into_iter()
                    .map(|c| substitute_with(c, allow_state_old || needs_anchor))
                    .collect();
                let joined = env.new_bool_join(&loc, Operation::And, conds, true);
                finalize(env, needs_anchor, joined)
            } else {
                report_underivable_bp(env, &loc, &lambda_loc, kind, context);
                intact()
            }
        },
        BehaviorKind::ResultOf => {
            // Prefer a functional `ensures`; otherwise derive the body value.
            let from_spec = lambda_spec.and_then(functional_result_ensures);
            let body_has_exact_value = spec_derivation::exp_has_exact_value_model(env, lambda_body);
            let derived = if from_spec.is_none() && body_has_exact_value {
                derive_from_body()
            } else {
                None
            };
            let from_body = derived
                .as_ref()
                .and_then(|d| d.results.as_ref())
                .and_then(|vals| match vals.as_slice() {
                    [val] => Some(val),
                    _ => None,
                });
            let source = from_spec.or(from_body);
            // Capture-independent results remain pointwise values.
            if !mutated_captures.is_empty() {
                let capture_syms: BTreeSet<Symbol> =
                    mutated_captures.iter().map(|capture| capture.sym).collect();
                if source.is_none_or(|val| !val.free_vars().is_disjoint(&capture_syms)) {
                    if weakenable_condition {
                        env.diag_with_labels(
                            Severity::Warning,
                            &loc,
                            &format!(
                                "cannot derive `result_of` exactly for this lambda argument: \
                                 the lambda mutates captured state; weakening condition; \
                                 see {INLINE_HOF_WEAKENING_ISSUE}"
                            ),
                            vec![(lambda_loc.clone(), "lambda argument".to_string())],
                        );
                    } else {
                        report_capture_writing_bp(env, &loc, &lambda_loc, kind, &mutated_captures);
                    }
                    return intact();
                }
            }
            if let Some(val) = source {
                let substituted = substitute(val);
                if env.is_verify_mode() {
                    if context == BpContext::LoopInvariant {
                        // Pre-state reads resolve to function entry; only
                        // intermediate-state material remains inexpressible
                        // in a value position.
                        if let Some(msg) = loop_invariant_residual(&substituted) {
                            spec_error_with_labels(env, &loc, msg, vec![(
                                lambda_loc.clone(),
                                "lambda argument".to_string(),
                            )]);
                        }
                    } else if condition_needs_anchor(env, val, &param_set)
                        || (context == BpContext::Plain && reads_global_state(env, val))
                    {
                        // A boolean anchor wrapper cannot be applied to a
                        // value; state-dependent results are not supported
                        // in this position. This includes bare (un-`old`-ed)
                        // memory reads, which describe the application's
                        // post-state: spliced into a plain assertion they
                        // would be evaluated at the assertion-site state,
                        // letting a false claim verify once memory changed
                        // in between. (In a spec function body — a
                        // single-state context — the current state is the
                        // only meaningful one, as for `aborts_of`.)
                        spec_error_with_labels(
                            env,
                            &loc,
                            "the lambda's result depends on global state, \
                             which cannot be anchored in a value position; \
                             use `ensures_of` with an explicit result \
                             argument instead",
                            vec![(lambda_loc.clone(), "lambda argument".to_string())],
                        );
                    }
                }
                substituted
            } else if body_has_exact_value
                && let Some(reduced) = beta_reduce_pure_lambda(
                    env,
                    &loc,
                    &tuple_pat,
                    &param_tys,
                    inputs,
                    lambda_body,
                    &lambda_result_ty,
                )
            {
                // Beta-reduction fallback: a pure, state-free body is a
                // value expression of the inputs alone; splice it under a
                // binding of the parameter pattern to the inputs. This
                // covers shapes the derivation cannot express as a single
                // result value, such as tuple-valued bodies or
                // destructurings of free tuple-typed variables. Being
                // state-free, no anchoring policy applies.
                reduced
            } else {
                if context == BpContext::SpecFunBody && !body_has_exact_value {
                    // The specialization records the residual predicate and
                    // lets its loop-invariant caller choose the fallback.
                } else if context == BpContext::LoopInvariant && !body_has_exact_value {
                    warn_underivable_concrete_behavior(env, &loc, BehaviorKind::ResultOf);
                } else if lambda_spec.is_some() {
                    // A spec is attached but has no functional value shape,
                    // and no value could be derived from the body either.
                    spec_error_with_labels(
                        env,
                        &loc,
                        "cannot resolve `result_of` for this lambda argument: \
                         the attached spec has no `ensures result == E` \
                         condition and no result value can be derived from \
                         the body; add such an ensures to the spec block, or \
                         use `ensures_of` with an explicit result argument",
                        vec![(lambda_loc.clone(), "lambda argument".to_string())],
                    );
                } else {
                    report_underivable_bp(env, &loc, &lambda_loc, kind, context);
                }
                intact()
            }
        },
        BehaviorKind::UnchangedOf => {
            // The frame is built from the body's derived `modifies` footprint;
            // an attached spec does not describe the footprint, so derivation
            // is required even when a spec is present. The `old(..)` wrappers
            // in the built conditions resolve to function entry in any spec
            // context of the expansion, so no anchor is needed; target
            // substitution permits state-referencing `old(..)` accordingly.
            if context == BpContext::SpecFunBody && env.is_verify_mode() {
                spec_error_with_labels(
                    env,
                    &loc,
                    "`unchanged_of` relates two states and cannot be used in \
                     the body of a spec function",
                    vec![(lambda_loc.clone(), "lambda argument".to_string())],
                );
                return intact();
            }
            if context == BpContext::FoldTransformer && env.is_verify_mode() {
                return intact();
            }
            let derived = derive_from_body();
            if let Some((targets, deferred)) =
                derived.and_then(|d| d.modifies.map(|targets| (targets, d.deferred_applications)))
            {
                // For a lambda forwarding to a function-typed parameter of
                // the enclosing function, the parameter application's
                // footprint is delegated: `unchanged_of` over the parameter
                // at the composed arguments, resolved when the enclosing
                // function is itself expanded. A genuine function-value
                // parameter has no expansion (and no `unchanged_of`
                // encoding), so its footprint stays unknown here.
                if !deferred.is_empty()
                    && !target_fun.is_some_and(|qid| env.get_function(qid).is_inline())
                {
                    spec_error_with_labels(
                        env,
                        &loc,
                        "cannot resolve `unchanged_of` for this lambda \
                         argument: the lambda applies a function-value \
                         parameter of the enclosing (non-inline) function, \
                         whose memory footprint is unknown",
                        vec![(lambda_loc.clone(), "lambda argument".to_string())],
                    );
                    return intact();
                }
                if !spec_derivation::exps_are_pure_single_state(
                    env,
                    deferred.iter().flat_map(|(_, inputs, _)| inputs),
                ) {
                    report_underivable_bp(env, &loc, &lambda_loc, kind, context);
                    return intact();
                }
                let mut conds: Vec<Exp> = targets
                    .iter()
                    .map(|target| mk_unchanged_condition(env, &loc, &substitute_with(target, true)))
                    .collect();
                for (target, inputs, _guard) in &deferred {
                    let mut bp_args = Vec::with_capacity(inputs.len() + 1);
                    // The parameter expression is enclosing-function scope
                    // and spliced untouched, like the predicate targets the
                    // regular substitution produces.
                    bp_args.push(target.clone());
                    bp_args.extend(inputs.iter().map(|input| substitute_with(input, true)));
                    conds.push(
                        ExpData::Call(
                            env.new_bool_node(&loc),
                            Operation::Behavior(BehaviorKind::UnchangedOf, MemoryRange::default()),
                            bp_args,
                        )
                        .into_exp(),
                    );
                }
                env.new_bool_join(&loc, Operation::And, conds, true)
            } else {
                report_underivable_bp(env, &loc, &lambda_loc, kind, context);
                intact()
            }
        },
        BehaviorKind::FoldsOf => {
            // Handled by the early return above, before the argument-layout
            // split.
            unreachable!("folds_of handled before argument-layout split")
        },
        BehaviorKind::WriteOf(_) => {
            env.diag(
                Severity::Bug,
                &loc,
                "unexpected internal behavioral predicate in source",
            );
            intact()
        },
    }
}

/// Builds the frame condition for a single modified memory target
/// `global<R>(A)` of an `unchanged_of` predicate:
/// `exists<R>(A) == old(exists<R>(A))
///  && (old(exists<R>(A)) ==> global<R>(A) == old(global<R>(A)))`.
fn mk_unchanged_condition(env: &GlobalEnv, loc: &Loc, target: &Exp) -> Exp {
    let ExpData::Call(target_id, Operation::Global(None), args) = target.as_ref() else {
        env.diag(
            Severity::Bug,
            loc,
            "invalid modifies target of derived lambda spec",
        );
        return env.new_bool_const(loc, true);
    };
    let resource_ty = env.get_node_type(*target_id);
    let inst = env.get_node_instantiation(*target_id);
    let addr = args[0].clone();
    let mk_read = |oper: Operation, ty: &Type| -> Exp {
        let id = env.new_node(loc.clone(), ty.clone());
        env.set_node_instantiation(id, inst.clone());
        ExpData::Call(id, oper, vec![addr.clone()]).into_exp()
    };
    let mk_old = |exp: Exp| -> Exp {
        let id = env.new_node(loc.clone(), env.get_node_type(exp.node_id()));
        ExpData::Call(id, Operation::Old, vec![exp]).into_exp()
    };
    let mk_eq = |lhs: Exp, rhs: Exp| -> Exp {
        ExpData::Call(env.new_bool_node(loc), Operation::Eq, vec![lhs, rhs]).into_exp()
    };
    let bool_ty = Type::Primitive(PrimitiveType::Bool);
    let exists = || mk_read(Operation::Exists(None), &bool_ty);
    let global = || mk_read(Operation::Global(None), &resource_ty);
    let exists_eq = mk_eq(exists(), mk_old(exists()));
    let value_eq = mk_eq(global(), mk_old(global()));
    let frame_implies = ExpData::Call(env.new_bool_node(loc), Operation::Implies, vec![
        mk_old(exists()),
        value_eq,
    ])
    .into_exp();
    ExpData::Call(env.new_bool_node(loc), Operation::And, vec![
        exists_eq,
        frame_implies,
    ])
    .into_exp()
}

/// Projects whole-memory effect operations in a substituted condition to
/// point facts over the current state, for consumption in loop invariants:
/// `update<R>(A, V)` and `publish<R>(A, V)` become
/// `exists<R>(A) && global<R>(A) == V`, and `remove<R>(A)` becomes
/// `!exists<R>(A)`. The effect operations assert "the post memory equals the
/// pre memory with exactly this change", which is false once further
/// iterations change other cells; the point facts carry the per-element
/// content. Value arguments keep their `old(..)`-wrapped pre-state reads,
/// which resolve to function entry. Only default-range operations are
/// projected; labeled ranges reference intermediate states and are left for
/// the residual check.
fn project_effects_to_point_facts(env: &GlobalEnv, exp: Exp) -> Exp {
    struct Projector<'a> {
        env: &'a GlobalEnv,
    }
    impl ExpRewriterFunctions for Projector<'_> {
        fn rewrite_call(&mut self, id: NodeId, oper: &Operation, args: &[Exp]) -> Option<Exp> {
            let (is_remove, range) = match oper {
                Operation::SpecUpdate(range) | Operation::SpecPublish(range) => (false, range),
                Operation::SpecRemove(range) => (true, range),
                _ => return None,
            };
            if !range.is_default() {
                return None;
            }
            let env = self.env;
            let loc = env.get_node_loc(id);
            let inst = env.get_node_instantiation(id);
            let Some(resource_ty) = inst.first().cloned() else {
                env.diag(
                    Severity::Bug,
                    &loc,
                    "missing resource instantiation on memory effect operation",
                );
                return None;
            };
            let addr = args[0].clone();
            let mk_read = |oper: Operation, ty: Type| -> Exp {
                let read_id = env.new_node(loc.clone(), ty);
                env.set_node_instantiation(read_id, inst.clone());
                ExpData::Call(read_id, oper, vec![addr.clone()]).into_exp()
            };
            let exists = mk_read(
                Operation::Exists(None),
                Type::Primitive(PrimitiveType::Bool),
            );
            if is_remove {
                return Some(
                    ExpData::Call(env.new_bool_node(&loc), Operation::Not, vec![exists]).into_exp(),
                );
            }
            let global = mk_read(Operation::Global(None), resource_ty);
            let value_eq = ExpData::Call(env.new_bool_node(&loc), Operation::Eq, vec![
                global,
                args[1].clone(),
            ])
            .into_exp();
            Some(
                ExpData::Call(env.new_bool_node(&loc), Operation::And, vec![
                    exists, value_eq,
                ])
                .into_exp(),
            )
        }
    }
    Projector { env }.rewrite_exp(exp)
}

/// Detects two-state material remaining in a loop-invariant condition after
/// effect projection which has no entry/current-state meaning, returning an
/// error message naming the boundary. Plain `old(..)` wrappers and unlabeled
/// memory reads resolve to function entry resp. the current state and pass.
fn loop_invariant_residual(exp: &Exp) -> Option<&'static str> {
    let mut residual = None;
    exp.visit_pre_order(&mut |e| {
        if let ExpData::Call(_, oper, _) = e {
            match oper {
                Operation::Behavior(_, range) | Operation::SpecFunction(_, _, range)
                    if !range.is_default() =>
                {
                    residual = Some(
                        "a lambda calling a function with global state effects \
                         cannot be constrained in a loop invariant: the callee's \
                         memory footprint is not projectable",
                    );
                },
                Operation::SpecPublish(..)
                | Operation::SpecRemove(..)
                | Operation::SpecUpdate(..)
                | Operation::Global(Some(..))
                | Operation::Exists(Some(..)) => {
                    residual = Some(
                        "a lambda with multiple dependent global state effects \
                         cannot be constrained in a loop invariant: intermediate \
                         memory states are not expressible",
                    );
                },
                _ => {},
            }
        }
        residual.is_none()
    });
    residual
}

/// If the spec has exactly one `ensures` condition of the shape
/// `result == E` (or `E == result`) where `E` does not mention `result`,
/// returns `E`.
fn functional_result_ensures(spec: &Spec) -> Option<&Exp> {
    let mut ensures = spec.filter_kind(ConditionKind::Ensures);
    let cond = ensures.next()?;
    if ensures.next().is_some() {
        return None;
    }
    let ExpData::Call(_, Operation::Eq, args) = cond.exp.as_ref() else {
        return None;
    };
    let is_result = |e: &Exp| matches!(e.as_ref(), ExpData::Call(_, Operation::Result(0), _));
    let uses_result =
        |e: &Exp| e.any(&mut |sub| matches!(sub, ExpData::Call(_, Operation::Result(_), _)));
    match (is_result(&args[0]), is_result(&args[1])) {
        (true, false) if !uses_result(&args[1]) => Some(&args[1]),
        (false, true) if !uses_result(&args[0]) => Some(&args[0]),
        _ => None,
    }
}

fn underivable_concrete_behavior(env: &GlobalEnv, exp: &Exp) -> Option<BehaviorKind> {
    let mut result = None;
    let mut pending = vec![exp.clone()];
    let mut visited = BTreeSet::new();
    while let Some(exp) = pending.pop() {
        exp.visit_pre_order(&mut |sub| {
            let ExpData::Call(_, Operation::Behavior(kind, _), args) = sub else {
                return true;
            };
            if let Some(ExpData::Lambda(_, _, body, _, _)) = args.first().map(|arg| arg.as_ref()) {
                if *kind == BehaviorKind::ResultOf
                    && !spec_derivation::exp_has_exact_value_model(env, body)
                {
                    result = Some(*kind);
                    return false;
                }
                return true;
            }
            let Some(ExpData::Call(_, Operation::Closure(mid, fid, _), _)) =
                args.first().map(|arg| arg.as_ref())
            else {
                return true;
            };
            if move_fun_behavior_is_underivable(env, mid.qualified(*fid), *kind) {
                result = Some(*kind);
                false
            } else {
                true
            }
        });
        if result.is_some() {
            break;
        }
        for callee in exp.called_spec_funs(env) {
            let id = callee.to_qualified_id();
            if visited.insert(id)
                && let Some(body) = env.get_spec_fun(id).body.clone()
            {
                pending.push(body);
            }
        }
    }
    result
}

fn move_fun_behavior_is_underivable(
    env: &GlobalEnv,
    qid: QualifiedFunId,
    kind: BehaviorKind,
) -> bool {
    let fun_env = env.get_function(qid);
    let spec = fun_env.get_spec();
    let has_mutable_parameter = fun_env
        .get_parameter_types()
        .iter()
        .any(Type::is_mutable_reference);
    let body_is_unavailable = fun_env.is_opaque() || fun_env.is_native_or_intrinsic();
    match kind {
        BehaviorKind::ResultOf => {
            (body_is_unavailable || !spec_derivation::move_fun_has_exact_value_model(env, qid))
                && (functional_result_ensures(&spec).is_none() || has_mutable_parameter)
        },
        BehaviorKind::AbortsOf => {
            fun_env.is_pragma_true(ABORTS_IF_IS_PARTIAL_PRAGMA, || false)
                || (body_is_unavailable
                    && spec.filter_kind(ConditionKind::AbortsIf).next().is_none()
                    && env
                        .get_intrinsics()
                        .get_abort_spec_fun_for_move_fun(&qid)
                        .is_none())
        },
        _ => false,
    }
}

fn spec_fun_call_has_underivable_behavior(
    env: &GlobalEnv,
    mid: move_model::model::ModuleId,
    sid: SpecFunId,
    args: &[Exp],
) -> bool {
    let decl = env.get_spec_fun(mid.qualified(sid));
    let Some(body) = &decl.body else {
        return false;
    };
    body.any(&mut |sub| {
        let ExpData::Call(_, Operation::Behavior(kind, _), bp_args) = sub else {
            return false;
        };
        let Some(sym) = bp_args.first().and_then(|target| match target.as_ref() {
            ExpData::LocalVar(_, sym) => Some(*sym),
            _ => None,
        }) else {
            return false;
        };
        let Some(pos) = decl.params.iter().position(|param| param.0 == sym) else {
            return false;
        };
        let Some(actual) = args.get(pos) else {
            return false;
        };
        match actual.as_ref() {
            ExpData::Call(_, Operation::Closure(fun_mid, fid, _), _) => {
                move_fun_behavior_is_underivable(env, fun_mid.qualified(*fid), *kind)
            },
            ExpData::Lambda(_, _, body, _, _) if *kind == BehaviorKind::ResultOf => {
                !spec_derivation::exp_has_exact_value_model(env, body)
            },
            _ => false,
        }
    })
}

fn uses_generic_type_reflection(env: &GlobalEnv, exp: &Exp) -> bool {
    exp.called_spec_funs(env)
        .iter()
        .any(|qid| env.spec_fun_uses_generic_type_reflection(qid))
}

fn is_type_reflection_fun(fun: &FunctionEnv<'_>) -> bool {
    fun.is_well_known(well_known::TYPE_NAME_MOVE)
        || fun.is_well_known(well_known::TYPE_INFO_MOVE)
        || fun.is_well_known(well_known::TYPE_NAME_GET_MOVE)
}

fn move_fun_uses_type_reflection(env: &GlobalEnv, qid: QualifiedFunId) -> bool {
    let fun = env.get_function(qid);
    is_type_reflection_fun(&fun)
        || fun.get_def().is_some_and(|def| {
            def.any(&mut |sub| {
                let ExpData::Call(_, Operation::MoveFunction(mid, fid), _) = sub else {
                    return false;
                };
                is_type_reflection_fun(&env.get_function(mid.qualified(*fid)))
            })
        })
}

fn behavior_uses_generic_type_reflection(env: &GlobalEnv, exp: &Exp) -> bool {
    exp.any(&mut |sub| {
        let ExpData::Call(_, Operation::Behavior(_, _), args) = sub else {
            return false;
        };
        let Some(ExpData::Call(target_id, Operation::Closure(mid, fid, _), _)) =
            args.first().map(|arg| arg.as_ref())
        else {
            return false;
        };
        env.get_node_instantiation(*target_id)
            .iter()
            .any(Type::is_type_parameter)
            && move_fun_uses_type_reflection(env, mid.qualified(*fid))
    })
}

fn lambda_uses_generic_type_reflection(env: &GlobalEnv, lambda: &Exp) -> bool {
    let ExpData::Lambda(_, _, body, _, _) = lambda.as_ref() else {
        return false;
    };
    body.any(&mut |exp| {
        let ExpData::Call(id, Operation::MoveFunction(mid, fid), _) = exp else {
            return false;
        };
        let fun = env.get_function(mid.qualified(*fid));
        let inst = env.get_node_instantiation(*id);
        let specs_use_reflection = fun
            .get_spec()
            .conditions
            .iter()
            // Conditions are stored in the callee's type-parameter context.
            // Apply the type arguments from this call before determining
            // whether reflection remains generic at the inline site.
            .any(|cond| {
                cond.exp.called_spec_funs(env).iter().any(|qid| {
                    env.spec_fun_uses_generic_type_reflection(&qid.clone().instantiate(&inst))
                })
            });
        let is_generic_call = inst.iter().any(Type::is_type_parameter);
        let body_uses_reflection = fun.get_def().is_some_and(|def| {
            def.any(&mut |sub| {
                let ExpData::Call(sub_id, Operation::MoveFunction(sub_mid, sub_fid), _) = sub
                else {
                    return false;
                };
                is_type_reflection_fun(&env.get_function(sub_mid.qualified(*sub_fid)))
                    && env
                        .get_node_instantiation(*sub_id)
                        .iter()
                        .any(Type::is_type_parameter)
            })
        });
        specs_use_reflection
            || (is_generic_call && (is_type_reflection_fun(&fun) || body_uses_reflection))
    })
}

/// Weakens only conjunction arms containing unresolved behavioral material.
fn weaken_unresolved_conjuncts(env: &GlobalEnv, exp: &Exp) -> (Exp, bool) {
    if let ExpData::Call(id, Operation::And, args) = exp.as_ref() {
        let mut changed = false;
        let args = args
            .iter()
            .map(|arg| {
                let (arg, arg_changed) = weaken_unresolved_conjuncts(env, arg);
                changed |= arg_changed;
                arg
            })
            .collect();
        if changed {
            return (ExpData::Call(*id, Operation::And, args).into_exp(), true);
        }
        return (exp.clone(), false);
    }
    if exp.any(&mut |sub| {
        is_unresolved_behavior(env, sub)
            || matches!(
                sub,
                ExpData::Call(_, Operation::SpecFunction(mid, sid, _), args)
                    if spec_fun_call_has_underivable_behavior(env, *mid, *sid, args)
            )
    }) {
        let loc = env.get_node_loc(exp.node_id());
        (env.new_bool_const(&loc, true), true)
    } else {
        (exp.clone(), false)
    }
}

fn depends_on_behavior(env: &GlobalEnv, exp: &Exp) -> bool {
    let mut pending = vec![exp.clone()];
    let mut visited = BTreeSet::new();
    while let Some(exp) = pending.pop() {
        if exp.any(&mut |sub| matches!(sub, ExpData::Call(_, Operation::Behavior(..), _))) {
            return true;
        }
        for callee in exp.called_spec_funs(env) {
            let id = callee.to_qualified_id();
            if visited.insert(id)
                && let Some(body) = env.get_spec_fun(id).body.clone()
            {
                pending.push(body);
            }
        }
    }
    false
}

fn warn_underivable_concrete_behavior(env: &GlobalEnv, loc: &Loc, kind: BehaviorKind) {
    if !env.is_verify_mode() {
        return;
    }
    let missing = if kind == BehaviorKind::ResultOf {
        "a nested result cannot be modeled from its specification"
    } else {
        "a nested call has no complete abort specification"
    };
    env.diag(
        Severity::Warning,
        loc,
        &format!(
            "cannot derive `{kind}` exactly for this lambda argument: {missing}; \
             weakening the enclosing loop invariant; see \
             {INLINE_HOF_WEAKENING_ISSUE}"
        ),
    );
}

fn warn_generic_type_reflection_behavior(env: &GlobalEnv, loc: &Loc) {
    if !env.is_verify_mode() {
        return;
    }
    env.diag(
        Severity::Warning,
        loc,
        &format!(
            "generic type reflection is not yet supported through behavioral predicates; \
             weakening the enclosing loop invariant; see {INLINE_HOF_WEAKENING_ISSUE}"
        ),
    );
}

/// The beta reduction of applying a lambda with a pure, state-free body to
/// the given inputs: the body spliced under a binding of the parameter
/// pattern to the input arguments. Such a body is a value expression of
/// the inputs alone, so no state anchoring policy applies; the spec
/// rewriter later converts remaining code-level constructs (dereferences,
/// calls to Move functions) to their spec forms. Returns `None` if a
/// parameter is `&mut` (its input slot carries the pre-value, not a
/// reference), or if the body is impure in specification mode, accesses
/// global state directly or through callees, or contains constructs
/// without a spec form (loops, function values).
fn beta_reduce_pure_lambda(
    env: &GlobalEnv,
    loc: &Loc,
    tuple_pat: &Pattern,
    param_tys: &[Type],
    inputs: &[Exp],
    body: &Exp,
    result_ty: &Type,
) -> Option<Exp> {
    if param_tys.iter().any(|ty| ty.is_mutable_reference()) {
        return None;
    }
    // Note: the checker's return value does not reflect all specification
    // mode violations (e.g. `return`); the action callback does.
    let mut is_pure = true;
    let mut checker =
        FunctionPurenessChecker::new(FunctionPurenessCheckerMode::Specification, |_, _, _| {
            is_pure = false;
        });
    checker.check_exp(env, body);
    if !is_pure || !spliceable_as_spec_value(env, body) {
        return None;
    }
    if inputs.is_empty() {
        return Some(body.clone());
    }
    let (pat, binding) = if inputs.len() == 1 {
        let pat = match tuple_pat {
            Pattern::Tuple(_, ps) if ps.len() == 1 => ps[0].clone(),
            _ => tuple_pat.clone(),
        };
        (pat, inputs[0].clone())
    } else {
        let tys = inputs
            .iter()
            .map(|e| env.get_node_type(e.node_id()))
            .collect::<Vec<_>>();
        let id = env.new_node(loc.clone(), Type::Tuple(tys));
        (
            tuple_pat.clone(),
            ExpData::Call(id, Operation::Tuple, inputs.to_vec()).into_exp(),
        )
    };
    let block_id = env.new_node(loc.clone(), result_ty.clone());
    Some(ExpData::Block(block_id, pat, Some(binding), body.clone()).into_exp())
}

/// Whether a lambda body can be spliced into a spec value position as-is:
/// it must not access global state — directly, or through callees, which
/// must qualify as pure spec calls without memory usage
/// (`try_as_pure_spec_call`), or through spec function calls, whose state
/// dependencies are established by the transitive inline-time body scan
/// (`spec_fun_state_usage`) — and must not contain loops or function
/// values (conservatively rejected: they have no spec value form).
fn spliceable_as_spec_value(env: &GlobalEnv, body: &Exp) -> bool {
    let mut memo = BTreeMap::new();
    let mut ok = true;
    body.visit_pre_order(&mut |e| {
        match e {
            ExpData::Call(id, oper, _) => match oper {
                Operation::BorrowGlobal(_)
                | Operation::Global(_)
                | Operation::Exists(_)
                | Operation::MoveTo
                | Operation::MoveFrom => ok = false,
                Operation::MoveFunction(mid, fid) => {
                    let inst = env.get_node_instantiation(*id);
                    if spec_derivation::try_as_pure_spec_call(env, *mid, *fid, &inst).is_none() {
                        ok = false;
                    }
                },
                Operation::SpecFunction(mid, fid, range) => {
                    let usage = spec_fun_state_usage(env, mid.qualified(*fid), &mut memo);
                    if !range.is_default() || usage.reads_memory || usage.uses_old {
                        ok = false;
                    }
                },
                Operation::Behavior(_, range) => {
                    if !range.is_default() {
                        ok = false;
                    }
                },
                Operation::Closure(..) => ok = false,
                _ => {},
            },
            ExpData::Loop(..)
            | ExpData::LoopCont(..)
            | ExpData::Lambda(..)
            | ExpData::Invoke(..) => ok = false,
            _ => {},
        }
        ok
    });
    ok
}

/// Reports a behavioral predicate over a lambda for which no spec is
/// available and none can be derived from the body. This is the seam where
/// an AST-level weakest-precondition inference for lambda bodies would
/// plug in. (For lambdas passed to non-inline functions — which are still
/// lambda-lifted — the bytecode-level `LambdaSpecInferenceProcessor` in
/// `spec_inference.rs` performs this role.)
fn report_underivable_bp(
    env: &GlobalEnv,
    loc: &Loc,
    lambda_loc: &Loc,
    kind: BehaviorKind,
    context: BpContext,
) {
    if context == BpContext::FoldTransformer {
        return;
    }
    if context == BpContext::LoopInvariant
        && matches!(kind, BehaviorKind::UnchangedOf | BehaviorKind::EnsuresOf)
    {
        // Only generic loop invariants may soundly drop these predicates.
        if env.is_verify_mode() {
            let message = if kind == BehaviorKind::UnchangedOf {
                format!(
                    "the memory footprint of this lambda argument cannot be \
                     determined exactly; weakening `unchanged_of` condition; \
                     see {INLINE_HOF_WEAKENING_ISSUE}"
                )
            } else {
                format!(
                    "the behavior of this lambda argument cannot be determined \
                     exactly; weakening loop invariant containing `ensures_of`; \
                     see {INLINE_HOF_WEAKENING_ISSUE}"
                )
            };
            env.diag_with_labels(Severity::Warning, loc, &message, vec![(
                lambda_loc.clone(),
                "lambda argument".to_string(),
            )]);
        }
        return;
    }
    let msg = format!(
        "cannot resolve `{}` for this lambda argument: \
         add a spec block to the lambda \
         (e.g. `|x| .. spec {{ aborts_if ..; ensures ..; }}`)",
        kind
    );
    spec_error_with_labels(env, loc, &msg, vec![(
        lambda_loc.clone(),
        "lambda argument".to_string(),
    )]);
}

/// Checks whether the body of a spec-less lambda contains precondition
/// material which `requires_of` would have to reflect: a call to a function
/// with a `requires` condition, or an application of a function value whose
/// `requires` is not known. Since the body derivation does not describe
/// preconditions (`DerivedSpec::requires` is reserved), `requires_of`
/// cannot honestly resolve to `true` for such a lambda; returns the error
/// message to report. Note that the actual callee preconditions are still
/// checked at their call sites within the beta-reduced expansion; this only
/// concerns the truth value of the `requires_of` predicate itself.
fn requires_material_in_body(env: &GlobalEnv, body: &Exp) -> Option<String> {
    let mut found = None;
    body.visit_pre_order(&mut |e| {
        match e {
            ExpData::Call(_, Operation::MoveFunction(mid, fid), _) => {
                let callee = env.get_function(mid.qualified(*fid));
                if callee
                    .get_spec()
                    .filter_kind(ConditionKind::Requires)
                    .next()
                    .is_some()
                {
                    found = Some(format!(
                        "cannot resolve `requires_of` for this lambda argument: \
                         the lambda's body calls `{}`, which has a `requires` \
                         condition; add a spec block with `requires` to the \
                         lambda (e.g. `|x| .. spec {{ requires ..; }}`)",
                        callee.get_full_name_str(),
                    ));
                }
            },
            ExpData::Invoke(_, target, _) if !matches!(target.as_ref(), ExpData::Lambda(..)) => {
                found = Some(
                    "cannot resolve `requires_of` for this lambda argument: \
                     the lambda's body applies a function value whose \
                     `requires` is not known; add a spec block with \
                     `requires` to the lambda (e.g. `|x| .. spec { requires ..; }`)"
                        .to_string(),
                );
            },
            _ => {},
        }
        found.is_none()
    });
    found
}

/// The global-state dependencies of a spec function at inline time: whether
/// its body (transitively) reads global memory, and whether it
/// (transitively) uses `old(..)` — i.e. is a two-state function.
///
/// `SpecFunDecl::used_memory`/`uses_old` cannot be consulted here: they are
/// computed by the spec rewriter, which runs *after* the inliner. Instead
/// the bodies are scanned transitively, memoized per declaration in `memo`
/// (with a monotone fixed-point closure for recursion cycles). A bodiless
/// (native or uninterpreted) spec function
/// is a fixed function of its arguments and hence state-independent,
/// consistent with `spec_derivation::exp_is_memory_free`; the `$fun`
/// companions of pure Move functions are memory-free by construction
/// (`spec_rewriter::run_pure_fun_companion_derivation`).
#[derive(Clone, Copy, Default, Eq, PartialEq)]
struct SpecFunStateUsage {
    reads_memory: bool,
    uses_old: bool,
}

impl SpecFunStateUsage {
    fn or(self, other: SpecFunStateUsage) -> SpecFunStateUsage {
        SpecFunStateUsage {
            reads_memory: self.reads_memory || other.reads_memory,
            uses_old: self.uses_old || other.uses_old,
        }
    }
}

type SpecFunStateUsageMemo = BTreeMap<QualifiedId<SpecFunId>, SpecFunStateUsage>;

fn spec_fun_state_usage(
    env: &GlobalEnv,
    qid: QualifiedId<SpecFunId>,
    memo: &mut SpecFunStateUsageMemo,
) -> SpecFunStateUsage {
    if let Some(usage) = memo.get(&qid) {
        return *usage;
    }
    // Mark in-progress; recursive calls initially see no additional usage.
    memo.insert(qid, SpecFunStateUsage::default());
    let usage = match env.get_spec_fun(qid).body.clone() {
        Some(body) => exp_spec_fun_state_usage(env, &body, true, memo),
        None => SpecFunStateUsage::default(),
    };
    memo.insert(qid, usage);

    // Close the reachable call graph to a fixed point. The initial
    // recursion guard above can temporarily under-approximate an edge in a
    // cycle: if F calls G before F's direct memory read and G calls F, G is
    // first recorded as state-free. Re-evaluating every discovered body with
    // the current memo monotonically propagates F's eventual usage back into
    // G (and through any larger SCC).
    loop {
        let mut changed = false;
        let discovered = memo.keys().copied().collect_vec();
        for candidate in discovered {
            let direct = match env.get_spec_fun(candidate).body.clone() {
                Some(body) => exp_spec_fun_state_usage(env, &body, true, memo),
                None => SpecFunStateUsage::default(),
            };
            let previous = *memo
                .get(&candidate)
                .expect("discovered spec function has memoized state usage");
            let closed = previous.or(direct);
            if closed != previous {
                memo.insert(candidate, closed);
                changed = true;
            }
        }
        if !changed {
            break;
        }
    }

    *memo
        .get(&qid)
        .expect("queried spec function has memoized state usage")
}

/// The global-state dependencies of an expression through the spec
/// functions it calls, plus — when `include_direct` is set, for spec
/// function *bodies* — its direct memory reads and `old(..)` uses. At the
/// condition level the direct parts are judged by the existing policy
/// helpers (`old(..)` over a lambda parameter, for example, is
/// state-independent after substitution), so only the indirect parts are
/// collected there.
fn exp_spec_fun_state_usage(
    env: &GlobalEnv,
    exp: &Exp,
    include_direct: bool,
    memo: &mut SpecFunStateUsageMemo,
) -> SpecFunStateUsage {
    let mut usage = SpecFunStateUsage::default();
    exp.visit_pre_order(&mut |e| {
        if let ExpData::Call(_, oper, _) = e {
            match oper {
                Operation::SpecFunction(mid, fid, _) => {
                    usage = usage.or(spec_fun_state_usage(env, mid.qualified(*fid), memo));
                },
                Operation::Global(..) | Operation::Exists(..) if include_direct => {
                    usage.reads_memory = true;
                },
                // A behavioral evaluator receives the target function's
                // current memory even when its range is default. Record that
                // dependency here so it also propagates through spec-function
                // wrappers. An explicit range additionally introduces a
                // non-current state dependency.
                Operation::Behavior(_, range) if include_direct => {
                    usage.reads_memory = true;
                    usage.uses_old |= !range.is_default();
                },
                Operation::Old if include_direct => {
                    usage.uses_old = true;
                },
                // Mutation ops relate two whole memories.
                Operation::SpecPublish(..)
                | Operation::SpecRemove(..)
                | Operation::SpecUpdate(..)
                    if include_direct =>
                {
                    usage.reads_memory = true;
                    usage.uses_old = true;
                },
                _ => {},
            }
        }
        !(usage.reads_memory && usage.uses_old)
    });
    usage
}

/// Whether a condition (transitively) calls a two-state spec function — one
/// whose body uses `old(..)`. Such a call cannot be honestly evaluated in a
/// single-state (pre-state) position, and in a two-state position its
/// pre-state must be bound by an anchor.
fn calls_two_state_spec_fun(env: &GlobalEnv, exp: &Exp) -> bool {
    exp_spec_fun_state_usage(env, exp, false, &mut BTreeMap::new()).uses_old
}

/// Whether a lambda spec condition (before substitution) references two
/// states of its own: `old(..)` over anything but a lambda parameter,
/// memory mutation builtins, explicit labels, or calls to two-state spec
/// functions (whose bodies use `old(..)`, detected by the transitive
/// inline-time scan). Such conditions require a state anchor at the
/// parameter's application site when substituted into a single-state spec
/// context.
fn condition_needs_anchor(env: &GlobalEnv, exp: &Exp, params: &BTreeSet<Symbol>) -> bool {
    let mut memo = BTreeMap::new();
    exp.any(&mut |e| {
        if let ExpData::Call(_, oper, args) = e {
            match oper {
                Operation::Old => !matches!(
                    args[0].as_ref(),
                    ExpData::LocalVar(_, sym) if params.contains(sym)
                ),
                Operation::SpecPublish(..)
                | Operation::SpecRemove(..)
                | Operation::SpecUpdate(..)
                | Operation::Global(Some(..))
                | Operation::Exists(Some(..)) => true,
                Operation::Behavior(_, range) => !range.is_default(),
                Operation::SpecFunction(mid, fid, range) => {
                    !range.is_default()
                        || spec_fun_state_usage(env, mid.qualified(*fid), &mut memo).uses_old
                },
                _ => false,
            }
        } else {
            false
        }
    })
}

/// Whether a condition reads global memory (labeled or not) — directly,
/// through a behavioral evaluator, or through a spec function whose body
/// (transitively) reads memory.
fn reads_global_state(env: &GlobalEnv, exp: &Exp) -> bool {
    let mut memo = BTreeMap::new();
    exp.any(&mut |e| {
        if let ExpData::Call(_, oper, args) = e {
            match oper {
                Operation::Global(..) | Operation::Exists(..) => true,
                Operation::Behavior(..) => match args.first().map(|arg| arg.as_ref()) {
                    Some(ExpData::Call(_, Operation::Closure(mid, fid, _), _)) => {
                        !spec_derivation::fun_has_no_memory_effects(env, mid.qualified(*fid))
                    },
                    // A behavioral predicate over a forwarded inline
                    // function parameter is deliberately left unresolved;
                    // its concrete lambda is classified when the enclosing
                    // inline function is expanded at the next callsite.
                    Some(ExpData::LocalVar(..) | ExpData::Temporary(..)) => false,
                    // For any other target shape, retain the conservative
                    // state-reading classification.
                    _ => true,
                },
                Operation::SpecFunction(mid, fid, _) => {
                    spec_fun_state_usage(env, mid.qualified(*fid), &mut memo).reads_memory
                },
                _ => false,
            }
        } else {
            false
        }
    })
}

/// Wraps global memory reads in `old(..)`, for abort and requires
/// conditions consumed under a state anchor (they refer to the
/// application's pre-state). A read is a direct `global`/`exists` access or
/// a behavioral evaluator, or a call to a memory-reading spec function; in
/// the latter two cases the *call itself* is wrapped, so its whole evaluation
/// — including its memory reads — resolves at the anchored pre-state.
/// Wrapping happens top-down without descending into wrapped reads or
/// pre-existing `old(..)` wrappers: everything below them already resolves
/// at the pre-state.
fn wrap_state_reads_in_old(env: &GlobalEnv, exp: Exp) -> Exp {
    struct Wrapper<'a> {
        env: &'a GlobalEnv,
        memo: SpecFunStateUsageMemo,
    }
    impl ExpRewriterFunctions for Wrapper<'_> {
        fn rewrite_exp(&mut self, exp: Exp) -> Exp {
            if let ExpData::Call(id, oper, _) = exp.as_ref() {
                let wrap = match oper {
                    Operation::Old => return exp,
                    Operation::Global(None) | Operation::Exists(None) => true,
                    Operation::Behavior(_, range) if range.is_default() => true,
                    Operation::SpecFunction(mid, fid, range) if range.is_default() => {
                        spec_fun_state_usage(self.env, mid.qualified(*fid), &mut self.memo)
                            .reads_memory
                    },
                    _ => false,
                };
                if wrap {
                    let old_id = self
                        .env
                        .new_node(self.env.get_node_loc(*id), self.env.get_node_type(*id));
                    return ExpData::Call(old_id, Operation::Old, vec![exp]).into_exp();
                }
            }
            self.rewrite_exp_descent(exp)
        }
    }
    Wrapper {
        env,
        memo: BTreeMap::new(),
    }
    .rewrite_exp(exp)
}

/// Substitutes lambda parameters and `result` in a lambda spec condition by
/// the arguments of a behavioral predicate. See
/// `substitute_bp_by_lambda_spec` for the mapping rules.
struct BpCondSubstituter<'a> {
    env: &'a GlobalEnv,
    curr: &'a BTreeMap<Symbol, Exp>,
    pre: &'a BTreeMap<Symbol, Exp>,
    results: &'a [Exp],
    bp_loc: &'a Loc,
    shadowed: Vec<BTreeSet<Symbol>>,
    /// Whether `old(..)` over non-parameter (state) content is permitted;
    /// within such an `old`, parameters substitute to their pre-state values.
    allow_state_old: bool,
    in_old: bool,
}

impl BpCondSubstituter<'_> {
    fn is_shadowed(&self, sym: &Symbol) -> bool {
        self.shadowed.iter().any(|scope| scope.contains(sym))
    }
}

impl ExpRewriterFunctions for BpCondSubstituter<'_> {
    fn rewrite_exp(&mut self, exp: Exp) -> Exp {
        if let ExpData::Call(id, Operation::Old, args) = exp.as_ref() {
            if let ExpData::LocalVar(_, sym) = args[0].as_ref() {
                if !self.is_shadowed(sym) {
                    if let Some(repl) = self.pre.get(sym) {
                        // The predicate's argument is state-independent data,
                        // so the `old` wrapper is dropped.
                        return repl.clone();
                    }
                }
            }
            if self.allow_state_old && reads_global_state(self.env, &args[0]) {
                // State-referencing `old(..)`: keep the wrapper (it resolves
                // to the anchored pre-state); parameters inside substitute to
                // their pre-state values.
                let saved = self.in_old;
                self.in_old = true;
                let inner = self.rewrite_exp(args[0].clone());
                self.in_old = saved;
                return ExpData::Call(*id, Operation::Old, vec![inner]).into_exp();
            }
            spec_error_with_labels(
                self.env,
                &self.env.get_node_loc(*id),
                "in the spec of a lambda constrained by a behavioral predicate \
                 of an inline function, `old(..)` can only be applied directly \
                 to a lambda parameter",
                vec![(
                    self.bp_loc.clone(),
                    "the behavioral predicate substituted here".to_string(),
                )],
            );
            return exp;
        }
        self.rewrite_exp_descent(exp)
    }

    fn rewrite_enter_scope<'b>(
        &mut self,
        _id: NodeId,
        vars: impl Iterator<Item = &'b (NodeId, Symbol)>,
    ) {
        self.shadowed.push(vars.map(|(_, sym)| *sym).collect());
    }

    fn rewrite_exit_scope(&mut self, _id: NodeId) {
        self.shadowed.pop();
    }

    fn rewrite_local_var(&mut self, _id: NodeId, sym: Symbol) -> Option<Exp> {
        if self.is_shadowed(&sym) {
            None
        } else if self.in_old {
            self.pre.get(&sym).cloned()
        } else {
            self.curr.get(&sym).cloned()
        }
    }

    fn rewrite_call(&mut self, id: NodeId, oper: &Operation, _args: &[Exp]) -> Option<Exp> {
        if let Operation::Result(idx) = oper {
            if let Some(result) = self.results.get(*idx) {
                return Some(result.clone());
            }
            self.env.diag(
                Severity::Bug,
                &self.env.get_node_loc(id),
                "unexpected `result` in substituted lambda spec condition",
            );
        }
        None
    }
}

// ======================================================================================
// Resolution of `folds_of` over lambda arguments

/// The substitution material for one `folds_of<f>(..)` occurrence over a
/// lambda-bound function parameter, computed by
/// `resolve_folds_of_occurrences` before the body of the inline function is
/// rewritten (verify mode only) and consumed by the loop-invariant
/// substitution arm of `substitute_bp_by_lambda_spec`.
enum FoldsOfResolution {
    /// Fully resolved against the concrete lambda (possibly with mutated
    /// captures); substituted by the fold equation and prefix no-abort
    /// condition (`build_folds_of_invariant`).
    Direct(FoldsOfDirect),
    /// Deferred through a pure forwarding lambda to a function-typed
    /// parameter of the enclosing inline function; substituted by a
    /// rewritten anchored occurrence over that parameter
    /// (`build_folds_of_deferral`), which resolves — possibly deferring
    /// again — when the enclosing function is itself expanded.
    Deferred(FoldsOfDeferred),
}

/// The material of a fully resolved `folds_of` occurrence.
struct FoldsOfDirect {
    /// The surface form: how the iterations' arguments are obtained.
    form: FoldsOfForm,
    /// The lambda's mutated captures with their snapshot symbols, in stable
    /// symbol order (the order of the fold recursion's `init` slots). Empty
    /// for a lambda without captures, which degenerates to the prefix
    /// no-abort condition alone.
    captures: Vec<FoldsOfCapture>,
    /// The specialization of the fold recursion over the lambda's derived
    /// accumulator transformer; `None` iff there are no captures.
    spec: Option<SpecFunSpecialization>,
    /// Anchored reads lifted into recursion context parameters.
    anchored_ctx_values: BTreeMap<Symbol, Exp>,
    /// The lambda's abort disjuncts, phrased in lambda scope over the
    /// iteration's pre-state: lambda parameters and captures appear as
    /// plain variables.
    aborts: Vec<Exp>,
    /// The lambda's parameter symbols, substituted by the iteration's
    /// arguments in the abort disjuncts.
    param_syms: Vec<Symbol>,
}

/// A `folds_of` occurrence deferred through a pure forwarding lambda. Its
/// arguments are composed in the expansion scope and its capture base is
/// retained by `label` until the concrete lambda is known.
struct FoldsOfDeferred {
    /// The forwarded parameter, in the enclosing function's scope (spliced
    /// untouched, like other deferred predicate targets).
    target: Exp,
    /// The iteration index binder of the composed components.
    j_sym: Symbol,
    /// Forwarded arguments at iteration `j_sym`.
    components: Vec<Exp>,
    /// Abort disjuncts outside the forwarded application.
    prelude_aborts: Vec<Exp>,
    /// The anchor label.
    label: MemoryLabel,
}

/// The form of a `folds_of` occurrence.
enum FoldsOfForm {
    /// The element form `folds_of<f>(v, i)`: unary `f` applied to
    /// `v[0..i]`. The fold call takes the vector, and iteration `j`'s
    /// argument is `v[j]`.
    Element,
    /// The general form `folds_of<f>(g, i)`: `f` applied to
    /// `g(0), .., g(i - 1)`. The index lambda's components — normalized to
    /// the expansion result's scope — provide iteration `j`'s arguments,
    /// referencing `j` through the stored index symbol; the fold call does
    /// not take a vector.
    General { j_sym: Symbol, args: Vec<Exp> },
}

/// A mutated capture of a `folds_of` target lambda.
struct FoldsOfCapture {
    /// The captured variable.
    sym: Symbol,
    /// The accumulator value type: the capture's type, or the referenced
    /// value type for a capture of `&mut` reference type.
    ty: Type,
    /// For a capture of `&mut` reference type (accumulation through the
    /// reference), the reference type. The capture's current value is then
    /// the dereferenced target, and the snapshot records the referenced
    /// value at expansion entry.
    ref_ty: Option<Type>,
    /// The capture in the enclosing function.
    current: Exp,
    /// Anchor for the capture's initial value.
    snapshot_label: MemoryLabel,
}

impl FoldsOfCapture {
    /// The expression denoting the capture's current value: the variable
    /// itself, or the dereferenced target for a `&mut` capture.
    fn current_value(&self, env: &GlobalEnv, loc: &Loc) -> Exp {
        let var = self.current.clone();
        if self.ref_ty.is_some() {
            ExpData::Call(
                env.new_node(loc.clone(), self.ty.clone()),
                Operation::Deref,
                vec![var],
            )
            .into_exp()
        } else {
            var
        }
    }

    /// The capture's value at the fold anchor.
    fn snapshot_value(&self, env: &GlobalEnv, loc: &Loc) -> Exp {
        let value = self.current_value(env, loc);
        let old = ExpData::Call(
            env.new_node(loc.clone(), self.ty.clone()),
            Operation::Old,
            vec![value],
        )
        .into_exp();
        ExpData::Call(
            env.new_node(loc.clone(), self.ty.clone()),
            Operation::WithStateAnchor(self.snapshot_label),
            vec![old],
        )
        .into_exp()
    }
}

/// A mutated capture normalized for source-level derivation.
struct MutatedCapture {
    sym: Symbol,
    name: Symbol,
    ty: Type,
    current: Exp,
}

/// A bespoke multi-capture fold recursion generated for a `folds_of`
/// resolution, unified across expansions of one inliner run: occurrences
/// over the same element and accumulator types whose transformer material
/// is spec-equivalent share one generated recursion, so facts proven about
/// one expansion apply to spec-equivalent lambdas elsewhere. No surface
/// declaration backs these — the accumulator is the capture tuple, and
/// tuples are not expressible as spec function type arguments or lambda
/// parameters — so the recursion takes per-capture `init` parameters and
/// returns the tuple (see `generate_multi_capture_recursion`).
struct FoldsOfRecursionEntry {
    /// The element type of the folded vector for the element form; `None`
    /// for the general (index) form, whose iteration arguments are embedded
    /// in the key.
    elem_ty: Option<Type>,
    /// The accumulator value types, in capture order.
    acc_tys: Vec<Type>,
    /// The matching key: the transformer material as a literal
    /// `|c1..ck, e| (E1..Ek)` (element form) or `|c1..ck, j| (E1..Ek)`
    /// (general form, iteration arguments composed in) lambda. Never
    /// translated; only compared via `is_spec_equivalent`.
    key: Exp,
    /// The generated recursion as a callable specialization: no retained
    /// positions — call sites pass `(v, init1..initk, end)` resp.
    /// `(init1..initk, end)` — with the context arguments appended by
    /// `make_call`.
    spec: SpecFunSpecialization,
}

/// Gives mutated parameter temporaries stable symbols for derivation while
/// retaining their original expressions for generated invariants. Ambiguous
/// direct assignments to parameter-named symbols are omitted.
fn prepare_mutated_captures(
    env: &GlobalEnv,
    target_fun: Option<QualifiedFunId>,
    lambda: &Exp,
    body: &Exp,
    param_syms: &[Symbol],
) -> (Vec<MutatedCapture>, Exp) {
    let bound: BTreeSet<Symbol> = param_syms.iter().copied().collect();
    let (mutated, mutated_temps) =
        spec_derivation::collect_mutated_free_vars_and_temps(env, body, &bound);
    let target_params: BTreeSet<Symbol> = target_fun
        .map(|qid| {
            env.get_function(qid)
                .get_parameters()
                .iter()
                .map(|Parameter(sym, ..)| *sym)
                .collect()
        })
        .unwrap_or_default();
    let types: BTreeMap<Symbol, Type> = lambda.free_vars_with_types(env).into_iter().collect();
    let loc = env.get_node_loc(lambda.node_id());
    let mut captures: Vec<MutatedCapture> = mutated
        .into_iter()
        .filter(|sym| !target_params.contains(sym))
        .filter_map(|sym| {
            types.get(&sym).map(|ty| MutatedCapture {
                sym,
                name: sym,
                ty: ty.clone(),
                current: ExpData::LocalVar(env.new_node(loc.clone(), ty.clone()), sym).into_exp(),
            })
        })
        .collect();
    let mut temp_map = BTreeMap::new();
    if let Some(qid) = target_fun {
        let params = env.get_function(qid).get_parameters();
        for idx in mutated_temps {
            let Some(Parameter(name, ty, _)) = params.get(idx) else {
                continue;
            };
            let sym = env.symbol_pool().make(&format!(
                "$lambda_capture_{}_{}",
                lambda.node_id().as_usize(),
                idx
            ));
            temp_map.insert(idx, sym);
            captures.push(MutatedCapture {
                sym,
                name: *name,
                ty: ty.clone(),
                current: ExpData::Temporary(env.new_node(loc.clone(), ty.clone()), idx).into_exp(),
            });
        }
    }
    captures.sort_by_key(|capture| capture.sym);
    let normalized = if temp_map.is_empty() {
        body.clone()
    } else {
        let mut replacer = |id: NodeId, target: ExpRewriteTarget| match target {
            ExpRewriteTarget::Temporary(idx) => temp_map
                .get(&idx)
                .map(|sym| ExpData::LocalVar(id, *sym).into_exp()),
            _ => None,
        };
        ExpRewriter::new(env, &mut replacer).rewrite_exp(body.clone())
    };
    (captures, normalized)
}

/// The temporary indices of the target function's function-typed
/// parameters. Applications of these in analyzed lambda bodies are
/// *forwarded* applications: the body derivation defers them (label-free
/// summaries, recorded in `DerivedSpec::deferred_applications`), since the
/// resulting predicates over the parameter re-resolve when the target
/// function is itself expanded — the transitivity of behavioral predicates
/// through forwarding wrappers — or translate directly for a genuine
/// function-value parameter.
fn deferred_fun_param_temps(
    env: &GlobalEnv,
    target_fun: Option<QualifiedFunId>,
) -> BTreeMap<TempIndex, Symbol> {
    target_fun
        .map(|qid| {
            env.get_function(qid)
                .get_parameters()
                .iter()
                .enumerate()
                .filter(|(_, Parameter(_, ty, _))| matches!(ty.skip_reference(), Type::Fun(..)))
                .map(|(idx, Parameter(sym, ..))| (idx, *sym))
                .collect()
        })
        .unwrap_or_default()
}

/// The decomposition of a *pure forwarding lambda*: an effect-free body —
/// typically a pure prelude such as reference projections — which applies
/// a function-typed parameter of the enclosing function exactly once and
/// unconditionally, with exact pure argument values.
struct ForwardedApplication {
    /// The applied parameter, in the enclosing function's scope.
    target: Exp,
    /// The application's argument values, phrased over the lambda's
    /// parameter symbols.
    args: Vec<Exp>,
    /// The lambda's own abort disjuncts outside the application (the
    /// prelude's), phrased over the parameter symbols.
    prelude_aborts: Vec<Exp>,
}

/// Decomposes the derived specification of a lambda into a forwarded
/// application (D6): the derivation must have recorded exactly one
/// deferred application (see `deferred_fun_param_temps`), unconditional,
/// with an exact and empty memory footprint of its own. The abort
/// disjuncts are partitioned: the application's `aborts_of` is covered by
/// whatever the caller defers, the rest is prelude material. Returns
/// `None` when the shape does not hold (the caller falls back to the
/// regular resolution paths), including when a disjunct *mixes* prelude
/// material with behavioral predicates over the parameter (e.g.
/// result-dependent aborts), which the partition cannot attribute.
fn derive_forwarded_application(derived: &DerivedSpec) -> Option<ForwardedApplication> {
    let [(target, args, guard)] = derived.deferred_applications.as_slice() else {
        return None;
    };
    if guard.is_some() {
        // A conditional application is not a forwarder: deferred material
        // assumes one application per invocation.
        return None;
    }
    // Effect-free: an exact, empty memory footprint.
    if !derived.modifies.as_ref().is_some_and(|m| m.is_empty()) {
        return None;
    }
    let mut prelude_aborts = vec![];
    for disjunct in &derived.aborts {
        if is_aborts_of_over(disjunct, target) {
            continue;
        }
        if mentions_behavior_over(disjunct, target) {
            return None;
        }
        prelude_aborts.push(disjunct.clone());
    }
    Some(ForwardedApplication {
        target: target.clone(),
        args: args.clone(),
        prelude_aborts,
    })
}

/// Whether the expression is a default-range `aborts_of` predicate whose
/// target is the same function parameter as `target`.
fn is_aborts_of_over(exp: &Exp, target: &Exp) -> bool {
    match exp.as_ref() {
        ExpData::Call(_, Operation::Behavior(BehaviorKind::AbortsOf, range), args) => {
            range.is_default() && args.first().is_some_and(|t| same_param_ref(t, target))
        },
        _ => false,
    }
}

/// Whether the expression contains a behavioral predicate over the same
/// function parameter as `target`.
fn mentions_behavior_over(exp: &Exp, target: &Exp) -> bool {
    exp.any(&mut |e| {
        matches!(
            e,
            ExpData::Call(_, Operation::Behavior(..), args)
                if args.first().is_some_and(|t| same_param_ref(t, target))
        )
    })
}

/// The display name of a function parameter reference (a `Temporary` by
/// index into the enclosing function's parameters, or a free `LocalVar`).
fn param_name_of(env: &GlobalEnv, target_fun: Option<QualifiedFunId>, exp: &Exp) -> String {
    match exp.as_ref() {
        ExpData::LocalVar(_, sym) => sym.display(env.symbol_pool()).to_string(),
        ExpData::Temporary(_, idx) => target_fun
            .map(|qid| env.get_function(qid).get_parameters())
            .and_then(|params| {
                params
                    .get(*idx)
                    .map(|Parameter(sym, ..)| sym.display(env.symbol_pool()).to_string())
            })
            .unwrap_or_else(|| "?".to_string()),
        _ => "?".to_string(),
    }
}

/// Whether two expressions reference the same function parameter (as a
/// `Temporary` by index or a free `LocalVar` by symbol).
fn same_param_ref(a: &Exp, b: &Exp) -> bool {
    match (a.as_ref(), b.as_ref()) {
        (ExpData::Temporary(_, i), ExpData::Temporary(_, j)) => i == j,
        (ExpData::LocalVar(_, s), ExpData::LocalVar(_, t)) => s == t,
        _ => false,
    }
}

/// Whether the expression contains a behavioral predicate over any of the
/// enclosing function's function-typed parameters (per
/// `deferred_fun_param_temps`).
fn mentions_behavior_over_fun_param(exp: &Exp, params: &BTreeMap<TempIndex, Symbol>) -> bool {
    exp.any(&mut |e| {
        if let ExpData::Call(_, Operation::Behavior(..), args) = e {
            match args.first().map(|t| t.as_ref()) {
                Some(ExpData::Temporary(_, idx)) => params.contains_key(idx),
                Some(ExpData::LocalVar(_, sym)) => params.values().any(|s| s == sym),
                _ => false,
            }
        } else {
            false
        }
    })
}

/// Reports pointwise predicates that require a capture's inductive value.
fn report_capture_writing_bp(
    env: &GlobalEnv,
    loc: &Loc,
    lambda_loc: &Loc,
    kind: BehaviorKind,
    captures: &[MutatedCapture],
) {
    let names = captures
        .iter()
        .map(|capture| format!("`{}`", capture.name.display(env.symbol_pool())))
        .join(", ");
    spec_error_with_labels(
        env,
        loc,
        &format!(
            "`{}` cannot constrain a lambda which writes the captured \
             variable(s) {}: the captures' values at an application are not \
             expressible; use `folds_of` in a loop invariant to constrain \
             the cumulative effect",
            kind, names
        ),
        vec![(lambda_loc.clone(), "lambda argument".to_string())],
    );
}

/// Resolves `folds_of` predicates over lambda-bound parameters by deriving
/// their capture transformers and specializing the fold recursion.
fn resolve_folds_of_occurrences(
    env: &mut GlobalEnv,
    body: &Exp,
    type_args: &[Type],
    parameters: &[Parameter],
    actuals: &[Exp],
    lambda_param_map: &BTreeMap<Symbol, &Exp>,
    lambda_free_vars: &BTreeSet<Symbol>,
    target_fun: Option<QualifiedFunId>,
    call_site_loc: &Loc,
    unifier: &mut Vec<SpecFunUnifierEntry>,
    folds_of_unifier: &mut Vec<FoldsOfRecursionEntry>,
) -> (BTreeMap<NodeId, FoldsOfResolution>, FoldsOfDeferralState) {
    let mut resolutions = BTreeMap::new();
    let mut deferral = FoldsOfDeferralState::default();
    if lambda_param_map.is_empty() {
        return (resolutions, deferral);
    }
    // Collect occurrences before rewriting their enclosing body.
    let mut occurrences: Vec<(NodeId, Exp, Exp, Option<MemoryLabel>)> = vec![];
    body.visit_pre_order(&mut |e| {
        if let ExpData::Call(id, Operation::Behavior(BehaviorKind::FoldsOf, range), args) = e {
            if args.len() == 3 {
                if let Some(lambda) = param_sym(args[0].as_ref(), parameters)
                    .and_then(|sym| lambda_param_map.get(&sym).copied())
                {
                    occurrences.push((*id, lambda.clone(), args[1].clone(), range.pre));
                }
            }
        }
        true
    });
    for (id, lambda, second_arg, anchor) in occurrences {
        let loc = env.get_node_loc(id).inlined_from(call_site_loc);
        // Normalize the vector or index lambda into the expansion scope.
        let normalized = match normalize_index_lambda(
            env,
            &second_arg,
            type_args,
            parameters,
            actuals,
            lambda_free_vars,
        ) {
            Some(e) => e,
            None => continue, // Error reported.
        };
        let (index_lambda, element_vec) = if matches!(second_arg.as_ref(), ExpData::Lambda(..)) {
            (Some(normalized), None)
        } else {
            (None, Some(normalized))
        };
        if let Some(resolution) = resolve_folds_of_occurrence(
            env,
            &lambda,
            index_lambda,
            element_vec,
            anchor,
            target_fun,
            &loc,
            unifier,
            folds_of_unifier,
            &mut deferral,
        ) {
            resolutions.insert(id, resolution);
        }
    }
    (resolutions, deferral)
}

/// Per-expansion state of anchored `folds_of` material: the label allocated
/// for occurrences resolved or deferred by this expansion. One
/// `FoldsCaptureAnchor` at expansion entry serves all of them.
#[derive(Default)]
struct FoldsOfDeferralState {
    entry_label: Option<MemoryLabel>,
}

/// Normalizes the index lambda `g` of a general-form `folds_of` occurrence
/// from the inline function's body scope to the expansion result's scope:
/// type arguments are instantiated on the nodes, references to the inline
/// function's parameters are replaced by the actual arguments (caller-scope
/// expressions), and free locals colliding with a lambda free variable get
/// the shadow symbol the body rewriter binds them under. Fails (with an
/// error) if `g` references a function-typed parameter.
fn normalize_index_lambda(
    env: &GlobalEnv,
    g: &Exp,
    type_args: &[Type],
    parameters: &[Parameter],
    actuals: &[Exp],
    lambda_free_vars: &BTreeSet<Symbol>,
) -> Option<Exp> {
    struct Normalizer<'a> {
        env: &'a GlobalEnv,
        type_args: &'a [Type],
        parameters: &'a [Parameter],
        actuals: &'a [Exp],
        lambda_free_vars: &'a BTreeSet<Symbol>,
        shadowed: Vec<BTreeSet<Symbol>>,
        failed: bool,
    }
    impl ExpRewriterFunctions for Normalizer<'_> {
        fn rewrite_enter_scope<'b>(
            &mut self,
            _id: NodeId,
            vars: impl Iterator<Item = &'b (NodeId, Symbol)>,
        ) {
            self.shadowed.push(vars.map(|(_, sym)| *sym).collect());
        }

        fn rewrite_exit_scope(&mut self, _id: NodeId) {
            self.shadowed.pop();
        }

        fn rewrite_node_id(&mut self, id: NodeId) -> Option<NodeId> {
            ExpData::instantiate_node(self.env, id, self.type_args)
        }

        fn rewrite_local_var(&mut self, id: NodeId, sym: Symbol) -> Option<Exp> {
            if self.shadowed.iter().any(|scope| scope.contains(&sym)) {
                return None;
            }
            // Free locals of the inline body which collide with a lambda
            // free variable are bound under their shadow symbol in the
            // expansion.
            if self.lambda_free_vars.contains(&sym) {
                let shadow = ShadowStack::create_shadow_symbol(self.env, &sym);
                return Some(ExpData::LocalVar(id, shadow).into_exp());
            }
            None
        }

        fn rewrite_temporary(&mut self, id: NodeId, idx: TempIndex) -> Option<Exp> {
            let actual = self.actuals.get(idx);
            if actual.is_none_or(|a| matches!(a.as_ref(), ExpData::Lambda(..))) {
                // A function-typed parameter (bound to a lambda) has no
                // data value to splice.
                self.failed = true;
                spec_error(
                    self.env,
                    &self.env.get_node_loc(id),
                    &format!(
                        "the index function of `folds_of` cannot reference the \
                         function-typed parameter `{}`",
                        self.parameters
                            .get(idx)
                            .map(|Parameter(sym, ..)| sym
                                .display(self.env.symbol_pool())
                                .to_string())
                            .unwrap_or_else(|| "?".to_string())
                    ),
                );
                return None;
            }
            let actual = actual.expect("checked above");
            if spliceable_as_spec_value(self.env, actual) {
                return Some(actual.clone());
            }
            // A state-reading (or otherwise non-spliceable) actual: the
            // normalized material references the parameter's binding in the
            // expansion instead — the actual is evaluated exactly once
            // there, so per-iteration re-evaluation of the spliced
            // expression (and global state in derived transformer
            // material) is avoided. Caller restatements cannot unify with
            // such material, which is inherent: the caller cannot name the
            // once-evaluated value either.
            let Some(Parameter(sym, ..)) = self.parameters.get(idx) else {
                self.failed = true;
                return None;
            };
            let sym = if self.lambda_free_vars.contains(sym) {
                // The parameter binds under its shadow symbol in the
                // expansion (it collides with a lambda free variable).
                ShadowStack::create_shadow_symbol(self.env, sym)
            } else {
                *sym
            };
            Some(ExpData::LocalVar(id, sym).into_exp())
        }
    }
    let mut normalizer = Normalizer {
        env,
        type_args,
        parameters,
        actuals,
        lambda_free_vars,
        shadowed: vec![],
        failed: false,
    };
    let result = normalizer.rewrite_exp(g.clone());
    (!normalizer.failed).then_some(result)
}

/// Resolves a single `folds_of` occurrence over the given lambda; see
/// `resolve_folds_of_occurrences`.
fn resolve_folds_of_occurrence(
    env: &mut GlobalEnv,
    lambda: &Exp,
    index_lambda: Option<Exp>,
    element_vec: Option<Exp>,
    anchor: Option<MemoryLabel>,
    target_fun: Option<QualifiedFunId>,
    loc: &Loc,
    unifier: &mut Vec<SpecFunUnifierEntry>,
    folds_of_unifier: &mut Vec<FoldsOfRecursionEntry>,
    deferral: &mut FoldsOfDeferralState,
) -> Option<FoldsOfResolution> {
    let lambda_loc = env.get_node_loc(lambda.node_id());
    let lambda_label = || vec![(lambda_loc.clone(), "lambda argument".to_string())];
    let cannot_resolve = |env: &GlobalEnv, reason: &str| {
        spec_error_with_labels(
            env,
            loc,
            &format!(
                "cannot resolve `folds_of` for this lambda argument: {}",
                reason
            ),
            lambda_label(),
        );
    };
    let weaken_with_issue = |env: &GlobalEnv, reason: &str, issue: &str| {
        if env.is_verify_mode() {
            env.diag_with_labels(
                Severity::Warning,
                loc,
                &format!(
                    "cannot derive `folds_of` exactly for this lambda argument: \
                     {}; weakening the enclosing loop invariant; see {}",
                    reason, issue
                ),
                lambda_label(),
            );
        }
    };
    let weaken =
        |env: &GlobalEnv, reason: &str| weaken_with_issue(env, reason, INLINE_HOF_WEAKENING_ISSUE);
    let ExpData::Lambda(_, pat, lambda_body, _, _) = lambda.as_ref() else {
        env.diag(
            Severity::Bug,
            loc,
            "invalid lambda target of behavioral predicate",
        );
        return None;
    };
    // Normalize the lambda's parameters, mirroring
    // `substitute_bp_by_lambda_spec`.
    let Type::Fun(param_ty, result_ty, _) = env.get_node_type(lambda.node_id()) else {
        env.diag(
            Severity::Bug,
            loc,
            "invalid type of lambda target of behavioral predicate",
        );
        return None;
    };
    let param_tys = param_ty.flatten();
    if index_lambda.is_none() && param_tys.len() != 1 {
        // The element form requires a unary target; an error has been
        // reported by type checking of the predicate's arguments.
        return None;
    }
    if param_tys.iter().any(|ty| ty.is_mutable_reference()) {
        cannot_resolve(
            env,
            "a lambda with `&mut` parameters is not supported (the folded \
             iteration arguments evolve with the iteration)",
        );
        return None;
    }
    // For the general form, destructure the normalized index lambda into
    // its binder and per-parameter argument components.
    let form = match &index_lambda {
        None => FoldsOfForm::Element,
        Some(g) => {
            let ExpData::Lambda(_, g_pat, g_body, _, _) = g.as_ref() else {
                unreachable!("index lambda shape established by the scan")
            };
            let j_sym = match g_pat {
                Pattern::Var(_, sym) => *sym,
                Pattern::Wildcard(_) => env
                    .symbol_pool()
                    .make(&format!("$wp_j_{}", g.node_id().as_usize())),
                _ => {
                    spec_error(
                        env,
                        &env.get_node_loc(g_pat.node_id()),
                        "the index function of `folds_of` must have a single \
                         plain `u64` parameter",
                    );
                    return None;
                },
            };
            let args: Vec<Exp> = if param_tys.len() == 1 {
                vec![g_body.clone()]
            } else if let ExpData::Call(_, Operation::Tuple, comps) = g_body.as_ref() {
                comps.clone()
            } else {
                vec![]
            };
            if args.len() != param_tys.len() {
                cannot_resolve(
                    env,
                    &format!(
                        "the index function of `folds_of` must produce a \
                         literal tuple of the target's {} argument(s)",
                        param_tys.len()
                    ),
                );
                return None;
            }
            if !spec_derivation::exps_are_pure_single_state_for_folds(env, &args) {
                weaken(
                    env,
                    "the index function of `folds_of` accesses global state, \
                     whose per-iteration evaluation state is not expressible \
                     in a loop invariant",
                );
                return None;
            }
            FoldsOfForm::General { j_sym, args }
        },
    };
    let lambda_result_ty = (*result_ty).clone();
    let tuple_pat = make_lambda_pattern_a_tuple(env, pat);
    let Pattern::Tuple(_, param_pats) = &tuple_pat else {
        unreachable!("lambda pattern normalized to tuple")
    };
    let mut param_syms: Vec<Symbol> = vec![];
    for (pos, p) in param_pats.iter().enumerate() {
        match p {
            Pattern::Var(_, sym) => param_syms.push(*sym),
            Pattern::Wildcard(_) => {
                param_syms.push(env.symbol_pool().make(&format!(
                    "$wp_p{}_{}",
                    pos,
                    lambda.node_id().as_usize()
                )));
            },
            _ => {
                spec_error(
                    env,
                    &env.get_node_loc(p.node_id()),
                    "lambdas with destructuring parameters are not supported \
                     with behavioral predicates",
                );
                return None;
            },
        }
    }

    // Discover the mutated captures and derive the per-iteration effect,
    // with the captures as implicit `&mut` parameters. Writes to variables
    // naming a parameter of the enclosing function cannot be tracked (see
    // `prepare_mutated_captures`); report them precisely instead of
    // through the generic derivation failure below.
    let bound: BTreeSet<Symbol> = param_syms.iter().copied().collect();
    let all_mutated =
        spec_derivation::collect_mutated_free_vars_and_temps(env, lambda_body, &bound).0;
    let target_params: BTreeSet<Symbol> = target_fun
        .map(|qid| {
            env.get_function(qid)
                .get_parameters()
                .iter()
                .map(|Parameter(sym, ..)| *sym)
                .collect()
        })
        .unwrap_or_default();
    if let Some(sym) = all_mutated.iter().find(|sym| target_params.contains(sym)) {
        cannot_resolve(
            env,
            &format!(
                "the lambda writes `{}`, which names a parameter of the \
                 enclosing function; copy the parameter into a local \
                 variable and capture that instead",
                sym.display(env.symbol_pool())
            ),
        );
        return None;
    }
    let (mutated_captures, derivation_body) =
        prepare_mutated_captures(env, target_fun, lambda, lambda_body, &param_syms);
    if let FoldsOfForm::General { args, .. } = &form {
        // The index function's arguments are re-evaluated per iteration by
        // the substitution; a dependency on a variable the lambda writes
        // (directly, or through an actual argument spliced by the
        // normalization) would make that evaluation state-dependent.
        let capture_syms: BTreeSet<Symbol> =
            mutated_captures.iter().map(|capture| capture.sym).collect();
        if args
            .iter()
            .any(|comp| !comp.free_vars().is_disjoint(&capture_syms))
        {
            cannot_resolve(
                env,
                "the index function's arguments depend on a captured \
                 variable the lambda writes, whose per-iteration value is \
                 not expressible",
            );
            return None;
        }
    }
    if mutated_captures.len() > MAX_FOLD_CAPTURES {
        cannot_resolve(
            env,
            &format!(
                "the lambda writes {} captured variables, more than the \
                 supported maximum of {} (the generated fold recursion \
                 returns the capture tuple)",
                mutated_captures.len(),
                MAX_FOLD_CAPTURES
            ),
        );
        return None;
    }
    let derived = target_fun.and_then(|qid| {
        let fun_env = env.get_function(qid);
        let mut generator = FunExpGenerator::new(fun_env, loc.clone());
        let params: Vec<(Symbol, Type)> = param_syms
            .iter()
            .cloned()
            .zip(param_tys.iter().cloned())
            .collect();
        let mut var_types: BTreeMap<Symbol, Type> = params.iter().cloned().collect();
        for (sym, ty) in lambda.free_vars_with_types(env) {
            var_types.entry(sym).or_insert(ty);
        }
        for capture in &mutated_captures {
            var_types.entry(capture.sym).or_insert(capture.ty.clone());
        }
        let captures: Vec<(Symbol, Type)> = mutated_captures
            .iter()
            .map(|capture| (capture.sym, capture.ty.clone()))
            .collect();
        spec_derivation::derive_spec_with_captures(
            &mut generator,
            &params,
            &captures,
            &var_types,
            &lambda_result_ty,
            &derivation_body,
            &deferred_fun_param_temps(env, target_fun),
        )
    });
    let Some(derived) = derived else {
        weaken(
            env,
            "the per-iteration effect of the lambda's body cannot be derived \
             exactly",
        );
        return None;
    };
    if mutated_captures.is_empty()
        && !derived.aborts.is_empty()
        && target_fun.is_some_and(|qid| {
            env.get_function(qid)
                .is_pragma_true(ABORTS_IF_IS_PARTIAL_PRAGMA, || false)
        })
    {
        weaken(
            env,
            "the enclosing function has a partial abort specification, so \
             the lambda's complete prefix-abort history is not required",
        );
        return None;
    }
    // A pure forwarding lambda defers the occurrence: it is rewritten to
    // an anchored `folds_of` over the enclosing inline function's
    // parameter, with the iteration arguments composed through the
    // forwarder, and resolves against the concrete lambda when that
    // function is itself expanded (see `FoldsOfDeferred`).
    if let Some(forwarded) = derive_forwarded_application(&derived) {
        let target_name = param_name_of(env, target_fun, &forwarded.target);
        if !mutated_captures.is_empty() {
            let names = mutated_captures
                .iter()
                .map(|capture| format!("`{}`", capture.name.display(env.symbol_pool())))
                .join(", ");
            weaken_with_issue(
                env,
                &format!(
                    "the lambda both writes the captured variable(s) {} and \
                     forwards to the function-typed parameter `{}`; the \
                     cumulative capture effect cannot be split across the \
                     forwarding — perform the capture updates in the lambda \
                     eventually bound to `{}`, or restate this wrapper \
                     without the intermediate lambda",
                    names, target_name, target_name
                ),
                FORWARDED_FOLD_WEAKENING_ISSUE,
            );
            return None;
        }
        if !target_fun.is_some_and(|qid| env.get_function(qid).is_inline()) {
            cannot_resolve(
                env,
                &format!(
                    "the lambda forwards to the function-value parameter \
                     `{}` of a non-inline function: `folds_of` requires the \
                     cumulative effect of a concrete lambda, which a \
                     function value provides only through inline expansion",
                    target_name
                ),
            );
            return None;
        }
        if !spec_derivation::exps_are_pure_single_state_for_folds(
            env,
            forwarded.args.iter().chain(&forwarded.prelude_aborts),
        ) {
            weaken(
                env,
                "the forwarded application's arguments or the forwarder's \
                 own abort conditions access global state, whose \
                 per-iteration evaluation state is not expressible in a \
                 loop invariant",
            );
            return None;
        }
        // Compose the application's arguments and the prelude's aborts at
        // iteration `j`'s arguments: `v[j]` for the element form, the
        // index lambda's components for the general form.
        let (j_sym, iteration_args): (Symbol, Vec<Exp>) = match &form {
            FoldsOfForm::Element => {
                let Some(v_norm) = element_vec else {
                    unreachable!("element-form vector normalized by the scan")
                };
                let j_sym = env
                    .symbol_pool()
                    .make(&format!("$fwd_j_{}", env.new_global_id().as_usize()));
                let elem_ty = match env.get_node_type(v_norm.node_id()).skip_reference() {
                    Type::Vector(elem) => elem.as_ref().clone(),
                    _ => {
                        env.diag(
                            Severity::Bug,
                            loc,
                            "invalid vector argument type of folds_of",
                        );
                        return None;
                    },
                };
                let u64_ty = Type::new_prim(PrimitiveType::U64);
                let elem_at_j =
                    ExpData::Call(env.new_node(loc.clone(), elem_ty), Operation::Index, vec![
                        v_norm,
                        ExpData::LocalVar(env.new_node(loc.clone(), u64_ty), j_sym).into_exp(),
                    ])
                    .into_exp();
                (j_sym, vec![elem_at_j])
            },
            FoldsOfForm::General { j_sym, args } => (*j_sym, args.clone()),
        };
        let subst: BTreeMap<Symbol, Exp> = param_syms.iter().copied().zip(iteration_args).collect();
        let components: Vec<Exp> = forwarded
            .args
            .iter()
            .map(|arg| substitute_free_locals(arg, &subst))
            .collect();
        let prelude_aborts: Vec<Exp> = forwarded
            .prelude_aborts
            .iter()
            .map(|abort| substitute_free_locals(abort, &subst))
            .collect();
        // A re-deferred occurrence keeps its anchor (the marker travels
        // within this body); a fresh deferral anchors at this expansion's
        // marker.
        let label = anchor.unwrap_or_else(|| {
            *deferral
                .entry_label
                .get_or_insert_with(|| MemoryLabel::new(env.new_global_id().as_usize()))
        });
        return Some(FoldsOfResolution::Deferred(FoldsOfDeferred {
            target: forwarded.target,
            j_sym,
            components,
            prelude_aborts,
            label,
        }));
    }
    // The exact final capture values, in capture order, are the transformer
    // material.
    let capture_values: Vec<(Symbol, Exp)> = if mutated_captures.is_empty() {
        vec![]
    } else {
        let values = derived.mut_param_values.as_ref().and_then(|vals| {
            (vals.len() == mutated_captures.len()
                && vals
                    .iter()
                    .zip(&mutated_captures)
                    .all(|((sym, _), capture)| *sym == capture.sym))
            .then(|| vals.clone())
        });
        let Some(values) = values else {
            weaken(
                env,
                "the values written to the captured variables cannot be \
                 expressed over the iteration's pre-state",
            );
            return None;
        };
        values
    };
    // A capture value must be phrased over the iteration's pre-state, where
    // a capture's pre-value appears as `old(c)`. A *plain* (un-`old`-wrapped)
    // reference to a capture is a post-state self-reference: it arises when
    // the lambda accumulates into the capture through a function call (the
    // capture, or a `&mut` to it, is passed to a callee), where the updated
    // value is known only through the callee's `ensures_of` — not as a
    // value the fold transformer could restate.
    {
        let capture_syms: BTreeSet<Symbol> =
            mutated_captures.iter().map(|capture| capture.sym).collect();
        if capture_values
            .iter()
            .any(|(_, value)| spec_derivation::mentions_syms_outside_old(value, &capture_syms))
        {
            weaken(
                env,
                "the lambda accumulates into a captured variable through a \
                 function call whose effect on the capture cannot be \
                 summarized as a value; perform the update directly in the \
                 lambda body, use a helper function that returns the new \
                 value, or give the callee a functional `ensures` for the \
                 `&mut` parameter (e.g. `ensures p == f(old(p), ..)`, with \
                 `pragma opaque` and `[abstract]` if stated rather than \
                 proven), which the derivation can consume",
            );
            return None;
        }
    }
    // Transformer material referencing a behavioral predicate over a
    // function-typed parameter of the enclosing function is a *mixed
    // forwarder shape*: the capture's evolution depends on the parameter's
    // eventual lambda (e.g. `new_table.add(*key, f(value))`), which the
    // fold recursion generated here cannot parameterize over.
    {
        let fun_params = deferred_fun_param_temps(env, target_fun);
        if capture_values
            .iter()
            .any(|(_, value)| mentions_behavior_over_fun_param(value, &fun_params))
        {
            weaken_with_issue(
                env,
                "the values written to the captured variables depend on an \
                 application of a function-typed parameter of the enclosing \
                 function; this mixed shape cannot be deferred — perform \
                 the capture updates in the lambda eventually bound to that \
                 parameter, or restate this wrapper without the \
                 intermediate lambda",
                FORWARDED_FOLD_WEAKENING_ISSUE,
            );
            return None;
        }
    }
    // The transformer and the abort conditions must be pure and
    // single-state: their per-iteration evaluation points are not
    // expressible in a loop invariant.
    if !spec_derivation::exps_are_pure_single_state_for_folds(
        env,
        capture_values.iter().map(|(_, e)| e).chain(&derived.aborts),
    ) {
        weaken(
            env,
            "the lambda combines captured-variable writes or abort \
             conditions with global state access, whose per-iteration \
             evaluation state is not expressible in a loop invariant",
        );
        return None;
    }
    let inferred = env.symbol_pool().make(CONDITION_INFERRED_PROP);
    let capture_names: BTreeSet<_> = mutated_captures
        .iter()
        .map(|capture| capture.name)
        .collect();
    let capture_temps: BTreeSet<_> = mutated_captures
        .iter()
        .flat_map(|capture| capture.current.used_temporaries())
        .collect();
    let transformer_has_behavior = capture_values
        .iter()
        .any(|(_, value)| depends_on_behavior(env, value));
    let summary_affects_success = transformer_has_behavior || !derived.aborts.is_empty();
    let fold_summary_is_used = target_fun.is_some_and(|qid| {
        let fun = env.get_function(qid);
        fun.is_inline()
            || fun
                .get_spec()
                .conditions
                .iter()
                .filter(|cond| !cond.properties.contains_key(&inferred))
                .any(|cond| match cond.kind {
                    ConditionKind::Ensures | ConditionKind::LetPost(..) => {
                        summary_affects_success
                            || !cond.exp.free_vars().is_disjoint(&capture_names)
                            || !cond.exp.used_temporaries().is_disjoint(&capture_temps)
                            || cond.exp.any(&mut |exp| {
                                matches!(exp, ExpData::Call(_, Operation::Result(..), _))
                            })
                    },
                    ConditionKind::Update | ConditionKind::Emits | ConditionKind::AbortsWith => {
                        true
                    },
                    ConditionKind::AbortsIf => {
                        !matches!(cond.exp.as_ref(), ExpData::Value(_, Value::Bool(false)))
                    },
                    _ => false,
                })
    });
    let fun_params = deferred_fun_param_temps(env, target_fun);
    let transformer_depends_on_fun_param = capture_values
        .iter()
        .any(|(_, value)| mentions_behavior_over_fun_param(value, &fun_params));
    let transformer_has_underivable_behavior = capture_values
        .iter()
        .any(|(_, value)| underivable_concrete_behavior(env, value).is_some());
    if !fold_summary_is_used && !transformer_depends_on_fun_param {
        let reason = if transformer_has_underivable_behavior {
            "the capture transformer contains behavior which cannot be \
             summarized as a value"
        } else {
            "the enclosing function has no specification which requires the \
             fold summary"
        };
        weaken(env, reason);
        return None;
    }
    // A lambda without captures degenerates to the prefix no-abort
    // condition; no snapshots, transformer, or recursion needed.
    if mutated_captures.is_empty() {
        return Some(FoldsOfResolution::Direct(FoldsOfDirect {
            form,
            captures: vec![],
            spec: None,
            anchored_ctx_values: BTreeMap::new(),
            aborts: derived.aborts,
            param_syms,
        }));
    }
    // The accumulator slots: value type and, for `&mut` captures, the
    // reference type.
    let accumulators: Vec<(Symbol, Type, Option<Type>)> = mutated_captures
        .iter()
        .map(|capture| {
            let ref_ty = capture
                .ty
                .is_mutable_reference()
                .then(|| capture.ty.clone());
            (capture.sym, capture.ty.skip_reference().clone(), ref_ty)
        })
        .collect();
    // The transformer material per form: for the element form the derived
    // capture values as-is (over the lambda's element parameter); for the
    // general form with the index lambda's argument components composed in
    // (over its index binder).
    let (transformer_params, transformer_values): (Vec<Symbol>, Vec<(Symbol, Exp)>) = match &form {
        FoldsOfForm::Element => (param_syms.clone(), capture_values.clone()),
        FoldsOfForm::General { j_sym, args } => {
            let comp_map: BTreeMap<Symbol, Exp> = param_syms
                .iter()
                .zip(args)
                .map(|(sym, comp)| (*sym, comp.clone()))
                .collect();
            let composed = capture_values
                .iter()
                .map(|(sym, value)| (*sym, substitute_free_locals(value, &comp_map)))
                .collect();
            (vec![*j_sym], composed)
        },
    };
    // Recursive spec functions receive anchored reads as context parameters;
    // `old(..)` itself is meaningful only at the loop invariant.
    let (transformer_values, anchored_ctx_values) =
        abstract_fold_anchored_values(env, loc, transformer_values);

    // Resolve the recursion: a single capture specializes the generic
    // `spec_fold` (element form) resp. `spec_fold_idx` (general form)
    // declaration — restatable by callers — while multiple captures get a
    // bespoke generated recursion returning the capture tuple (tuples are
    // not expressible as spec type arguments).
    let spec = if accumulators.len() == 1 {
        let (capture_sym, acc_ty, _) = accumulators[0].clone();
        let (name, signature, inst) = match &form {
            FoldsOfForm::Element => (
                well_known::VECTOR_SPEC_FOLD,
                "spec fun spec_fold<T, Acc>(f: |Acc, &T| Acc, v: vector<T>, \
                 init: Acc, end: u64): Acc",
                vec![param_tys[0].skip_reference().clone(), acc_ty],
            ),
            FoldsOfForm::General { .. } => (
                well_known::VECTOR_SPEC_FOLD_IDX,
                "spec fun spec_fold_idx<Acc>(t: |Acc, u64| Acc, init: Acc, \
                 end: u64): Acc",
                vec![acc_ty],
            ),
        };
        let Some(fold_qid) = find_fold_recursion(env, target_fun, name) else {
            cannot_resolve(
                env,
                &format!(
                    "no `{}` declaration available: declare `{}` in this module",
                    name, signature
                ),
            );
            return None;
        };
        let shape_ok = match &form {
            FoldsOfForm::Element => has_spec_fold_shape(env, fold_qid),
            FoldsOfForm::General { .. } => has_spec_fold_idx_shape(env, fold_qid),
        };
        if !shape_ok {
            cannot_resolve(
                env,
                &format!(
                    "the resolved `{}` declaration does not have the expected \
                     signature `{}`",
                    name, signature
                ),
            );
            return None;
        }
        let transformer_ty = env.get_spec_fun(fold_qid).params[0].1.instantiate(&inst);
        let acc_sym = env
            .symbol_pool()
            .make(&format!("$acc_{}", env.new_global_id().as_usize()));
        let transformer =
            build_fold_transformer(env, loc, &transformer_ty, acc_sym, &transformer_params, &[
                (capture_sym, transformer_values[0].1.clone()),
            ])?;
        let mut specializer = SpecFunSpecializer::new(env, target_fun, unifier);
        specializer.bp_context = BpContext::FoldTransformer;
        let Some(spec) = specializer.specialize(loc, fold_qid, inst, vec![(0, transformer)], None)
        else {
            weaken(
                env,
                "the capture transformer contains behavior which cannot be \
                 summarized as a value",
            );
            return None;
        };
        spec
    } else {
        let elem_ty = match &form {
            FoldsOfForm::Element => Some(param_tys[0].skip_reference().clone()),
            FoldsOfForm::General { .. } => None,
        };
        generate_multi_capture_recursion(
            env,
            target_fun,
            loc,
            elem_ty.as_ref(),
            &accumulators,
            &transformer_values,
            transformer_params[0],
            folds_of_unifier,
        )?
    };
    // Deferred occurrences retain their original capture anchor.
    let snapshot_label = anchor.unwrap_or_else(|| {
        *deferral
            .entry_label
            .get_or_insert_with(|| MemoryLabel::new(env.new_global_id().as_usize()))
    });
    let capture_currents: BTreeMap<Symbol, Exp> = mutated_captures
        .iter()
        .map(|capture| (capture.sym, capture.current.clone()))
        .collect();
    let captures = accumulators
        .into_iter()
        .map(|(sym, ty, ref_ty)| FoldsOfCapture {
            sym,
            ty,
            ref_ty,
            current: capture_currents[&sym].clone(),
            snapshot_label,
        })
        .collect();
    Some(FoldsOfResolution::Direct(FoldsOfDirect {
        form,
        captures,
        spec: Some(spec),
        anchored_ctx_values,
        aborts: derived.aborts,
        param_syms,
    }))
}

/// Lifts anchored fold-transformer reads into recursion context parameters.
fn abstract_fold_anchored_values(
    env: &GlobalEnv,
    loc: &Loc,
    values: Vec<(Symbol, Exp)>,
) -> (Vec<(Symbol, Exp)>, BTreeMap<Symbol, Exp>) {
    struct Abstractor<'a> {
        env: &'a GlobalEnv,
        loc: &'a Loc,
        values: Vec<(Symbol, Exp)>,
    }
    impl ExpRewriterFunctions for Abstractor<'_> {
        fn rewrite_exp(&mut self, exp: Exp) -> Exp {
            if matches!(
                exp.as_ref(),
                ExpData::Call(_, Operation::WithStateAnchor(..), _)
            ) {
                let pos = self
                    .values
                    .iter()
                    .position(|(_, value)| {
                        value.as_ref().is_spec_equivalent(self.env, exp.as_ref())
                    })
                    .unwrap_or_else(|| {
                        let sym = self
                            .env
                            .symbol_pool()
                            .make(&format!("$fold_anchor_ctx_{}", self.values.len()));
                        self.values.push((sym, exp.clone()));
                        self.values.len() - 1
                    });
                let sym = self.values[pos].0;
                let ty = self.env.get_node_type(exp.node_id());
                return ExpData::LocalVar(self.env.new_node(self.loc.clone(), ty), sym).into_exp();
            }
            self.rewrite_exp_descent(exp)
        }
    }
    let mut abstractor = Abstractor {
        env,
        loc,
        values: vec![],
    };
    let values = values
        .into_iter()
        .map(|(sym, value)| (sym, abstractor.rewrite_exp(value)))
        .collect();
    (values, abstractor.values.into_iter().collect())
}

/// Resolves the fold recursion declaration of the given name:
/// `std::vector`'s if it declares one, otherwise a like-named declaration
/// in the expansion target's module.
fn find_fold_recursion(
    env: &GlobalEnv,
    target_fun: Option<QualifiedFunId>,
    name: &str,
) -> Option<QualifiedId<SpecFunId>> {
    let in_vector = match name {
        well_known::VECTOR_SPEC_FOLD => well_known::find_vector_spec_fold(env),
        well_known::VECTOR_SPEC_FOLD_IDX => well_known::find_vector_spec_fold_idx(env),
        _ => None,
    };
    in_vector.or_else(|| {
        target_fun.and_then(|qid| {
            well_known::find_spec_fun_in_module(&env.get_module(qid.module_id), name)
        })
    })
}

/// Whether the resolved fold recursion declaration has the expected
/// element-form signature
/// `spec fun spec_fold<T, Acc>(f: |Acc, &T| Acc, v: vector<T>, init: Acc, end: u64): Acc`
/// (the reference on the element parameter is optional).
fn has_spec_fold_shape(env: &GlobalEnv, qid: QualifiedId<SpecFunId>) -> bool {
    let decl = env.get_spec_fun(qid);
    let elem = Type::TypeParameter(0);
    let acc = Type::TypeParameter(1);
    let u64_ty = Type::new_prim(PrimitiveType::U64);
    if decl.type_params.len() != 2 || decl.params.len() != 4 || decl.result_type != acc {
        return false;
    }
    let Type::Fun(f_params, f_result, _) = &decl.params[0].1 else {
        return false;
    };
    let f_param_tys = f_params.clone().flatten();
    f_param_tys.len() == 2
        && f_param_tys[0] == acc
        && f_param_tys[1].skip_reference() == &elem
        && f_result.as_ref() == &acc
        && decl.params[1].1 == Type::Vector(Box::new(elem))
        && decl.params[2].1 == acc
        && decl.params[3].1 == u64_ty
}

/// Whether the resolved fold recursion declaration has the expected
/// general-form signature
/// `spec fun spec_fold_idx<Acc>(t: |Acc, u64| Acc, init: Acc, end: u64): Acc`.
fn has_spec_fold_idx_shape(env: &GlobalEnv, qid: QualifiedId<SpecFunId>) -> bool {
    let decl = env.get_spec_fun(qid);
    let acc = Type::TypeParameter(0);
    let u64_ty = Type::new_prim(PrimitiveType::U64);
    if decl.type_params.len() != 1 || decl.params.len() != 3 || decl.result_type != acc {
        return false;
    }
    let Type::Fun(t_params, t_result, _) = &decl.params[0].1 else {
        return false;
    };
    let t_param_tys = t_params.clone().flatten();
    t_param_tys.len() == 2
        && t_param_tys[0] == acc
        && t_param_tys[1] == u64_ty
        && t_result.as_ref() == &acc
        && decl.params[1].1 == acc
        && decl.params[2].1 == u64_ty
}

/// Builds the accumulator transformer literal `|acc, p1..pn| E` of a fold
/// specialization: the derived final capture value `E`, with the capture's
/// pre-state reference `old(c)` replaced by the fresh accumulator parameter
/// `acc`. The original lambda's parameter symbols are kept, so `E`
/// references them unchanged; free variables of the enclosing scope stay
/// free and become context arguments of the specialization. `fun_ty` is the
/// instantiated type of the recursion's eliminated function parameter,
/// typing the literal.
fn build_fold_transformer(
    env: &GlobalEnv,
    loc: &Loc,
    fun_ty: &Type,
    acc_sym: Symbol,
    param_syms: &[Symbol],
    capture_values: &[(Symbol, Exp)],
) -> Option<Exp> {
    let Type::Fun(t_params, _, _) = fun_ty else {
        env.diag(
            Severity::Bug,
            loc,
            "invalid type of fold recursion function parameter",
        );
        return None;
    };
    let t_param_tys = t_params.clone().flatten();
    debug_assert_eq!(t_param_tys.len(), 1 + param_syms.len());
    // Replace `old(c)` by the accumulator.
    let acc_ty = t_param_tys[0].clone();
    let (capture_sym, value) = &capture_values[0];
    let acc_var = ExpData::LocalVar(env.new_node(loc.clone(), acc_ty), acc_sym).into_exp();
    let pre_state_map: BTreeMap<Symbol, Exp> = [(*capture_sym, acc_var)].into_iter().collect();
    let body = replace_capture_pre_states(value, &pre_state_map);
    // Internal invariant: the derived value references the capture's
    // pre-state only as `old(c)`, so no reference to the capture and no
    // `old(..)` remains in the transformer.
    let residual = body.free_vars().contains(capture_sym)
        || body.any(&mut |e| matches!(e, ExpData::Call(_, Operation::Old, _)));
    if residual {
        env.diag(
            Severity::Bug,
            loc,
            "residual pre-state reference in derived fold transformer",
        );
        return None;
    }
    let param_pats: Vec<Pattern> = iter::once(&acc_sym)
        .chain(param_syms)
        .zip(&t_param_tys)
        .map(|(sym, ty)| Pattern::Var(env.new_node(loc.clone(), ty.clone()), *sym))
        .collect();
    let pat = Pattern::Tuple(
        env.new_node(loc.clone(), Type::Tuple(t_param_tys)),
        param_pats,
    );
    let lambda_id = env.new_node(loc.clone(), fun_ty.clone());
    Some(ExpData::Lambda(lambda_id, pat, body, LambdaCaptureKind::Default, None).into_exp())
}

/// Replaces the pre-state references `old(c)` of the given captures by the
/// mapped expressions: the accumulator parameter of a transformer, or the
/// recursion's bound accumulator components. The `old(c)` shape is
/// generated by the derivation itself, so a structural rewrite is exact.
fn replace_capture_pre_states(exp: &Exp, map: &BTreeMap<Symbol, Exp>) -> Exp {
    struct Replacer<'a> {
        map: &'a BTreeMap<Symbol, Exp>,
    }
    impl ExpRewriterFunctions for Replacer<'_> {
        fn rewrite_exp(&mut self, exp: Exp) -> Exp {
            if let ExpData::Call(_, Operation::Old, args) = exp.as_ref() {
                if let ExpData::LocalVar(_, sym) = args[0].as_ref() {
                    if let Some(repl) = self.map.get(sym) {
                        return repl.clone();
                    }
                }
            }
            self.rewrite_exp_descent(exp)
        }
    }
    Replacer { map }.rewrite_exp(exp.clone())
}

/// The maximum number of captured variables supported by `folds_of`: the
/// generated multi-capture recursion returns the capture tuple, whose width
/// Boogie's tuple encoding bounds.
const MAX_FOLD_CAPTURES: usize = 8;

/// Generates (or reuses) the bespoke fold recursion of a multi-capture
/// `folds_of` resolution over captures `c1..ck` with accumulator value
/// types `A1..Ak`:
/// ```text
/// spec fun spec_fold$gen$N(v: vector<T>, c1$init: A1, .., ck$init: Ak,
///                          end: u64, <ctx..>): (A1, .., Ak) {
///     if (end == 0) (c1$init, .., ck$init)
///     else {
///         let (c1$acc, .., ck$acc) =
///             spec_fold$gen$N(v, c1$init, .., ck$init, end - 1, <ctx..>);
///         (E1, .., Ek)
///     }
/// }
/// ```
/// where `E1..Ek` are the transformer values with the pre-state references
/// `old(c)` replaced by the bound accumulator components and the iteration
/// parameter by `v[end - 1]`. `elem_ty` is the element type of the folded
/// vector for the element form; for the general (index) form it is `None`,
/// the recursion takes no vector, and the iteration parameter — the index
/// binder, which the pre-composed transformer values reference — is
/// replaced by `end - 1` instead. Free variables of the transformer
/// material become context parameters, mirroring `SpecFunSpecializer` (the
/// generated binders all contain `$`, so they cannot capture user
/// variables). Occurrences with the same form, element and accumulator
/// types, and spec-equivalent transformer material share one recursion
/// through `folds_of_unifier`, so facts proven about one expansion apply
/// to spec-equivalent lambdas elsewhere.
fn generate_multi_capture_recursion(
    env: &mut GlobalEnv,
    target_fun: Option<QualifiedFunId>,
    loc: &Loc,
    elem_ty: Option<&Type>,
    accumulators: &[(Symbol, Type, Option<Type>)],
    capture_values: &[(Symbol, Exp)],
    iter_param: Symbol,
    folds_of_unifier: &mut Vec<FoldsOfRecursionEntry>,
) -> Option<SpecFunSpecialization> {
    let Some(module_id) = target_fun.map(|qid| qid.module_id) else {
        // The derivation preceding this call already requires a target
        // function.
        return None;
    };
    let acc_tys: Vec<Type> = accumulators.iter().map(|(_, ty, _)| ty.clone()).collect();
    let u64_ty = Type::new_prim(PrimitiveType::U64);
    let iter_param_ty = elem_ty.cloned().unwrap_or_else(|| u64_ty.clone());
    let mk_var = |env: &GlobalEnv, sym: Symbol, ty: &Type| -> Exp {
        ExpData::LocalVar(env.new_node(loc.clone(), ty.clone()), sym).into_exp()
    };
    // The matching key: the transformer material as
    // `|c1..ck, <iter>| (E1..Ek)`, with the pre-state references replaced
    // by the like-named parameters.
    let key = {
        let pre_state_map: BTreeMap<Symbol, Exp> = accumulators
            .iter()
            .map(|(sym, ty, _)| (*sym, mk_var(env, *sym, ty)))
            .collect();
        let items: Vec<Exp> = capture_values
            .iter()
            .map(|(_, value)| replace_capture_pre_states(value, &pre_state_map))
            .collect();
        let body = ExpData::Call(
            env.new_node(loc.clone(), Type::Tuple(acc_tys.clone())),
            Operation::Tuple,
            items,
        )
        .into_exp();
        let pat_vars: Vec<(Symbol, Type)> = accumulators
            .iter()
            .map(|(sym, ty, _)| (*sym, ty.clone()))
            .chain(iter::once((iter_param, iter_param_ty.clone())))
            .collect();
        let pats: Vec<Pattern> = pat_vars
            .iter()
            .map(|(sym, ty)| Pattern::Var(env.new_node(loc.clone(), ty.clone()), *sym))
            .collect();
        let pat_tys: Vec<Type> = pat_vars.into_iter().map(|(_, ty)| ty).collect();
        let fun_ty = Type::Fun(
            Box::new(Type::Tuple(pat_tys.clone())),
            Box::new(Type::Tuple(acc_tys.clone())),
            AbilitySet::EMPTY,
        );
        let pat = Pattern::Tuple(env.new_node(loc.clone(), Type::Tuple(pat_tys)), pats);
        ExpData::Lambda(
            env.new_node(loc.clone(), fun_ty),
            pat,
            body,
            LambdaCaptureKind::Default,
            None,
        )
        .into_exp()
    };
    if key.any(&mut |e| matches!(e, ExpData::Call(_, Operation::Old, _))) {
        env.diag(
            Severity::Bug,
            loc,
            "residual pre-state reference in derived fold transformer",
        );
        return None;
    }
    // Reuse a spec-equivalent recursion.
    if let Some(entry) = folds_of_unifier.iter().find(|entry| {
        entry.elem_ty.as_ref() == elem_ty
            && entry.acc_tys == acc_tys
            && entry.key.as_ref().is_spec_equivalent(env, key.as_ref())
    }) {
        return Some(entry.spec.clone());
    }
    // Context arguments: free variables of the transformer material (and
    // parameters of the enclosing function it uses), freshened against the
    // material's own binders, mirroring `SpecFunSpecializer`. Reference
    // types are stripped: specifications are value-level, and the
    // translation passes the referenced value at call sites.
    let enclosing_params = target_fun
        .map(|qid| env.get_function(qid).get_parameters())
        .unwrap_or_default();
    let mut ctx_args: Vec<CtxArg> = vec![];
    for (sym, ty) in key.free_vars_with_types(env) {
        if !ctx_args.iter().any(|c| c.temp.is_none() && c.sym == sym) {
            ctx_args.push(CtxArg {
                sym,
                param_sym: sym,
                temp: None,
                ty: ty.skip_reference().clone(),
                caller_ty: ty,
            });
        }
    }
    for (idx, ty) in key.used_temporaries_with_types(env) {
        if let Some(Parameter(sym, ..)) = enclosing_params.get(idx) {
            if !ctx_args.iter().any(|c| c.temp == Some(idx)) {
                ctx_args.push(CtxArg {
                    sym: *sym,
                    param_sym: *sym,
                    temp: Some(idx),
                    ty: ty.skip_reference().clone(),
                    caller_ty: ty,
                });
            }
        } else {
            env.diag(
                Severity::Bug,
                loc,
                "unresolved parameter reference in lambda argument",
            );
        }
    }
    if ctx_args
        .iter()
        .any(|ctx| ctx.caller_ty.is_mutable_reference())
    {
        if env.is_verify_mode() {
            env.diag(
                Severity::Warning,
                loc,
                &format!(
                    "cannot derive `folds_of` exactly for this lambda argument: \
                     a multi-capture fold reads through a mutable reference, \
                     which cannot yet be represented as a stable recursion \
                     parameter; weakening the enclosing loop invariant; see {}",
                    INLINE_HOF_WEAKENING_ISSUE
                ),
            );
        }
        return None;
    }
    let mut taken: BTreeSet<Symbol> = capture_values
        .iter()
        .flat_map(|(_, value)| binder_syms(value))
        .collect();
    let mut ctx_renames: BTreeMap<Symbol, Symbol> = BTreeMap::new();
    for ctx in ctx_args.iter_mut() {
        if taken.contains(&ctx.sym) {
            let base = ctx.sym.display(env.symbol_pool()).to_string();
            let mut count = 0;
            ctx.param_sym = loop {
                let candidate = env.symbol_pool().make(&format!("{}$ctx{}", base, count));
                if !taken.contains(&candidate) {
                    break candidate;
                }
                count += 1;
            };
            if ctx.temp.is_none() {
                ctx_renames.insert(ctx.sym, ctx.param_sym);
            }
        }
        taken.insert(ctx.param_sym);
    }

    // The generated recursion may stem from a generic enclosing context;
    // like `specialize`, the declaration is made parametric over the
    // context type parameters its material mentions, remapped to a compact
    // index space, with call sites passing the mentioned parameters (see
    // `compact_type_param_mapping`). The mention set is determined by the
    // element/accumulator types and the transformer key — the data the
    // recursion unifier compares.
    let mut used_params: BTreeSet<u16> = BTreeSet::new();
    if let Some(ty) = elem_ty {
        used_type_params_in_type(ty, &mut used_params);
    }
    for ty in &acc_tys {
        used_type_params_in_type(ty, &mut used_params);
    }
    used_type_params_in_exp(env, &key, &mut used_params);
    let (type_params, spec_type_args, type_param_remap) =
        compact_type_param_mapping(env, loc, &used_params);
    let remap_ty = |ty: Type| -> Type {
        if type_param_remap.is_empty() {
            ty
        } else {
            ty.instantiate(&type_param_remap)
        }
    };

    // Register the declaration; the body's recursive call is built from the
    // specialization record directly. The vector parameter exists only for
    // the element form.
    let pool = env.symbol_pool();
    let v_sym = pool.make("$v");
    let end_sym = pool.make("$end");
    let init_syms: Vec<Symbol> = accumulators
        .iter()
        .map(|(sym, ..)| pool.make(&format!("{}$init", sym.display(pool))))
        .collect();
    let acc_syms: Vec<Symbol> = accumulators
        .iter()
        .map(|(sym, ..)| pool.make(&format!("{}$acc", sym.display(pool))))
        .collect();
    let name = pool.make(&format!("spec_fold$gen${}", folds_of_unifier.len()));
    let vector_ty = elem_ty.map(|ty| Type::Vector(Box::new(ty.clone())));
    let result_type = Type::Tuple(acc_tys.clone());
    let mut params: Vec<Parameter> = vec![];
    if let Some(vector_ty) = &vector_ty {
        params.push(Parameter(v_sym, remap_ty(vector_ty.clone()), loc.clone()));
    }
    params.extend(
        init_syms
            .iter()
            .zip(&acc_tys)
            .map(|(sym, ty)| Parameter(*sym, remap_ty(ty.clone()), loc.clone())),
    );
    params.push(Parameter(end_sym, u64_ty.clone(), loc.clone()));
    params.extend(
        ctx_args
            .iter()
            .map(|c| Parameter(c.param_sym, remap_ty(c.ty.clone()), loc.clone())),
    );
    let new_qid = env.add_spec_function_def(module_id, SpecFunDecl {
        loc: loc.clone(),
        name,
        type_params,
        params,
        result_type: remap_ty(result_type.clone()),
        used_memory: BTreeSet::new(),
        old_memory: BTreeSet::new(),
        uninterpreted: false,
        is_move_fun: false,
        is_native: false,
        body: None,
        callees: BTreeSet::new(),
        is_recursive: RefCell::new(None),
        insts_using_generic_type_reflection: RefCell::new(BTreeMap::new()),
        spec: RefCell::new(Spec::default()),
        uses_old: false,
        frame_spec: None,
    });
    let specialization = SpecFunSpecialization {
        qid: new_qid,
        retained: vec![],
        ctx_args: ctx_args.clone(),
        result_type: result_type.clone(),
        type_args: spec_type_args,
        underivable_behavior: Rc::new(Cell::new(None)),
    };
    folds_of_unifier.push(FoldsOfRecursionEntry {
        elem_ty: elem_ty.cloned(),
        acc_tys: acc_tys.clone(),
        key,
        spec: specialization.clone(),
    });

    // Build the body.
    let base = ExpData::Call(
        env.new_node(loc.clone(), result_type.clone()),
        Operation::Tuple,
        init_syms
            .iter()
            .zip(&acc_tys)
            .map(|(sym, ty)| mk_var(env, *sym, ty))
            .collect(),
    )
    .into_exp();
    let zero = ExpData::Value(
        env.new_node(loc.clone(), u64_ty.clone()),
        Value::Number(BigInt::from(0)),
    )
    .into_exp();
    let cond = ExpData::Call(env.new_bool_node(loc), Operation::Eq, vec![
        mk_var(env, end_sym, &u64_ty),
        zero,
    ])
    .into_exp();
    let one = ExpData::Value(
        env.new_node(loc.clone(), u64_ty.clone()),
        Value::Number(BigInt::from(1)),
    )
    .into_exp();
    let end_minus_one = ExpData::Call(
        env.new_node(loc.clone(), u64_ty.clone()),
        Operation::Sub,
        vec![mk_var(env, end_sym, &u64_ty), one],
    )
    .into_exp();
    let mut rec_args = vec![];
    if let Some(vector_ty) = &vector_ty {
        rec_args.push(mk_var(env, v_sym, vector_ty));
    }
    rec_args.extend(
        init_syms
            .iter()
            .zip(&acc_tys)
            .map(|(sym, ty)| mk_var(env, *sym, ty)),
    );
    rec_args.push(end_minus_one.clone());
    let rec_call = specialization.make_call(
        env,
        loc.clone(),
        &MemoryRange::default(),
        rec_args,
        &ctx_renames,
    );
    // The iteration parameter at the step: `v[end - 1]` for the element
    // form, `end - 1` itself for the index form.
    let iter_at_end = match (elem_ty, &vector_ty) {
        (Some(elem_ty), Some(vector_ty)) => ExpData::Call(
            env.new_node(loc.clone(), elem_ty.clone()),
            Operation::Index,
            vec![mk_var(env, v_sym, vector_ty), end_minus_one],
        )
        .into_exp(),
        _ => end_minus_one,
    };
    let pre_state_map: BTreeMap<Symbol, Exp> = accumulators
        .iter()
        .zip(&acc_syms)
        .map(|((sym, ty, _), acc_sym)| (*sym, mk_var(env, *acc_sym, ty)))
        .collect();
    let mut elem_subst: BTreeMap<Symbol, Exp> = BTreeMap::new();
    elem_subst.insert(iter_param, iter_at_end);
    let step_items: Vec<Exp> = capture_values
        .iter()
        .map(|(_, value)| {
            let value = rename_free_vars(value, &ctx_renames);
            let value = replace_capture_pre_states(&value, &pre_state_map);
            substitute_free_locals(&value, &elem_subst)
        })
        .collect();
    let step_tuple = ExpData::Call(
        env.new_node(loc.clone(), result_type.clone()),
        Operation::Tuple,
        step_items,
    )
    .into_exp();
    let acc_pats: Vec<Pattern> = acc_syms
        .iter()
        .zip(&acc_tys)
        .map(|(sym, ty)| Pattern::Var(env.new_node(loc.clone(), ty.clone()), *sym))
        .collect();
    let let_block = ExpData::Block(
        env.new_node(loc.clone(), result_type.clone()),
        Pattern::Tuple(
            env.new_node(loc.clone(), Type::Tuple(acc_tys.clone())),
            acc_pats,
        ),
        Some(rec_call),
        step_tuple,
    )
    .into_exp();
    let body = ExpData::IfElse(
        env.new_node(loc.clone(), result_type.clone()),
        cond,
        base,
        let_block,
    )
    .into_exp();
    // Replace references to enclosing-function parameters by the context
    // parameters.
    let temp_map: BTreeMap<TempIndex, (Symbol, Type)> = ctx_args
        .iter()
        .filter_map(|c| c.temp.map(|idx| (idx, (c.param_sym, c.ty.clone()))))
        .collect();
    let body = if temp_map.is_empty() {
        body
    } else {
        let env_ref: &GlobalEnv = env;
        let mut replacer = |_id: NodeId, target: ExpRewriteTarget| match target {
            ExpRewriteTarget::Temporary(idx) => temp_map.get(&idx).map(|(sym, ty)| {
                ExpData::LocalVar(env_ref.new_node(loc.clone(), ty.clone()), *sym).into_exp()
            }),
            _ => None,
        };
        ExpRewriter::new(env_ref, &mut replacer).rewrite_exp(body)
    };
    // Remap the body — built in the enclosing context's type parameter
    // space — to the declaration's compact type parameters.
    let body = instantiate_exp_with_patterns(env, body, &type_param_remap);
    // Internal invariant: every free variable of the generated body is a
    // parameter.
    let param_set: BTreeSet<Symbol> = env
        .get_spec_fun(new_qid)
        .params
        .iter()
        .map(|Parameter(sym, ..)| *sym)
        .collect();
    if !body.free_vars().is_subset(&param_set) {
        env.diag(
            Severity::Bug,
            loc,
            "dangling variable reference in the body of a generated fold recursion",
        );
    }
    let callees = body.called_spec_funs(env);
    let new_decl = env.get_spec_fun_mut(new_qid);
    new_decl.body = Some(body);
    new_decl.callees = callees;
    Some(specialization)
}

/// Builds the loop-invariant conditions substituted for a resolved
/// `folds_of<f>(v, i)` occurrence:
/// ```text
/// (c1, .., ck) == spec_fold$N(v, c1$pre, .., ck$pre, i)
///   && forall j in 0..i:
///        !ABORT[c1..ck -> spec_fold$N(v, c1$pre, .., ck$pre, j), e -> v[j]]
/// ```
/// where `ABORT` is the disjunction of the lambda's abort conditions,
/// `c$pre` the captures' snapshots at expansion entry, and `spec_fold$N`
/// the specialized or generated fold recursion. For a general-form
/// occurrence `folds_of<f>(g, i)` the fold call takes no vector, and
/// iteration `j`'s arguments are the index lambda's components at `j`
/// instead of `v[j]`. A `&mut` capture's current value is the dereferenced
/// target. With more than one capture the recursion returns a tuple, which
/// the no-abort condition threads through a `let`. For a lambda without
/// captures only the prefix no-abort conjunct remains; without abort
/// conditions, only the equation.
fn build_folds_of_invariant(
    env: &GlobalEnv,
    loc: &Loc,
    resolution: &FoldsOfDirect,
    v_arg: &Exp,
    i_arg: &Exp,
) -> Exp {
    let mk_var = |sym: Symbol, ty: &Type| -> Exp {
        ExpData::LocalVar(env.new_node(loc.clone(), ty.clone()), sym).into_exp()
    };
    let fold_call = |end: Exp| -> Exp {
        let spec = resolution
            .spec
            .as_ref()
            .expect("fold specialization present with captures");
        let mut args = vec![];
        if matches!(resolution.form, FoldsOfForm::Element) {
            args.push(v_arg.clone());
        }
        args.extend(
            resolution
                .captures
                .iter()
                .map(|c| c.snapshot_value(env, loc)),
        );
        args.push(end);
        spec.make_call_with_context_values(
            env,
            loc.clone(),
            &MemoryRange::default(),
            args,
            &BTreeMap::new(),
            &resolution.anchored_ctx_values,
        )
    };
    let mut conjuncts = vec![];
    if !resolution.captures.is_empty() {
        // The equation: the captures' current values are the fold of the
        // first `i` elements from their entry values.
        let lhs = if let [capture] = resolution.captures.as_slice() {
            capture.current_value(env, loc)
        } else {
            let tuple_ty = Type::Tuple(resolution.captures.iter().map(|c| c.ty.clone()).collect());
            ExpData::Call(
                env.new_node(loc.clone(), tuple_ty),
                Operation::Tuple,
                resolution
                    .captures
                    .iter()
                    .map(|c| c.current_value(env, loc))
                    .collect(),
            )
            .into_exp()
        };
        let eq = ExpData::Call(env.new_bool_node(loc), Operation::Eq, vec![
            lhs,
            fold_call(i_arg.clone()),
        ])
        .into_exp();
        conjuncts.push(eq);
    }
    if !resolution.aborts.is_empty() {
        // The prefix no-abort condition, with the iteration's arguments per
        // form.
        let u64_ty = Type::new_prim(PrimitiveType::U64);
        let j_sym = env
            .symbol_pool()
            .make(&format!("$j_{}", env.new_global_id().as_usize()));
        let mut subst: BTreeMap<Symbol, Exp> = BTreeMap::new();
        match &resolution.form {
            FoldsOfForm::Element => {
                let elem_ty = match env.get_node_type(v_arg.node_id()).skip_reference() {
                    Type::Vector(elem) => elem.as_ref().clone(),
                    _ => {
                        env.diag(
                            Severity::Bug,
                            loc,
                            "invalid vector argument type of folds_of",
                        );
                        return env.new_bool_const(loc, true);
                    },
                };
                let elem_at_j =
                    ExpData::Call(env.new_node(loc.clone(), elem_ty), Operation::Index, vec![
                        v_arg.clone(),
                        mk_var(j_sym, &u64_ty),
                    ])
                    .into_exp();
                subst.insert(resolution.param_syms[0], elem_at_j);
            },
            FoldsOfForm::General {
                j_sym: g_j_sym,
                args,
            } => {
                let j_map: BTreeMap<Symbol, Exp> =
                    [(*g_j_sym, mk_var(j_sym, &u64_ty))].into_iter().collect();
                for (param, comp) in resolution.param_syms.iter().zip(args) {
                    subst.insert(*param, substitute_free_locals(comp, &j_map));
                }
            },
        }
        let mk_abort = |subst: &BTreeMap<Symbol, Exp>| -> Exp {
            env.new_bool_join(
                loc,
                Operation::Or,
                resolution
                    .aborts
                    .iter()
                    .map(|a| substitute_free_locals(a, subst))
                    .collect(),
                false,
            )
        };
        let no_abort_j = if resolution.captures.len() > 1 {
            // Thread the recursion's accumulator tuple through a `let`,
            // binding fresh components for the captures' values at the
            // start of iteration `j`.
            let it_syms: Vec<Symbol> = resolution
                .captures
                .iter()
                .map(|c| {
                    env.symbol_pool()
                        .make(&format!("{}$it", c.sym.display(env.symbol_pool())))
                })
                .collect();
            for (capture, it_sym) in resolution.captures.iter().zip(&it_syms) {
                subst.insert(capture.sym, mk_var(*it_sym, &capture.ty));
            }
            let not_abort = ExpData::Call(env.new_bool_node(loc), Operation::Not, vec![mk_abort(
                &subst,
            )])
            .into_exp();
            let it_pats: Vec<Pattern> = it_syms
                .iter()
                .zip(&resolution.captures)
                .map(|(sym, c)| Pattern::Var(env.new_node(loc.clone(), c.ty.clone()), *sym))
                .collect();
            let tuple_ty = Type::Tuple(resolution.captures.iter().map(|c| c.ty.clone()).collect());
            ExpData::Block(
                env.new_bool_node(loc),
                Pattern::Tuple(env.new_node(loc.clone(), tuple_ty), it_pats),
                Some(fold_call(mk_var(j_sym, &u64_ty))),
                not_abort,
            )
            .into_exp()
        } else {
            for capture in &resolution.captures {
                subst.insert(capture.sym, fold_call(mk_var(j_sym, &u64_ty)));
            }
            ExpData::Call(env.new_bool_node(loc), Operation::Not, vec![mk_abort(
                &subst,
            )])
            .into_exp()
        };
        let zero = ExpData::Value(
            env.new_node(loc.clone(), u64_ty.clone()),
            Value::Number(BigInt::from(0)),
        )
        .into_exp();
        let range = ExpData::Call(
            env.new_node(loc.clone(), Type::Primitive(PrimitiveType::Range)),
            Operation::Range,
            vec![zero, i_arg.clone()],
        )
        .into_exp();
        let j_pat = Pattern::Var(env.new_node(loc.clone(), u64_ty), j_sym);
        let forall = ExpData::Quant(
            env.new_bool_node(loc),
            QuantKind::Forall,
            vec![(j_pat, range)],
            vec![],
            None,
            no_abort_j,
        )
        .into_exp();
        conjuncts.push(forall);
    }
    env.new_bool_join(loc, Operation::And, conjuncts, true)
}

/// Builds the deferred substitution of a `folds_of` occurrence over a pure
/// forwarding lambda (see `FoldsOfDeferred`):
/// ```text
/// folds_of<f>(|j| (A1(v[j]), .., An(v[j])), i) @ anchor(label)
///   && forall j in 0..i: !PRELUDE_ABORT[params -> iteration args]
/// ```
/// where `f` is the enclosing inline function's forwarded parameter,
/// `A1..An` the application's argument values composed at iteration `j`'s
/// arguments (`v[j]` for an element-form occurrence, the original index
/// lambda's components for a general-form one), and `PRELUDE_ABORT` the
/// disjunction of the forwarder's own abort conditions. The anchor label
/// is carried in the occurrence's `MemoryRange` and matches the
/// `FoldsCaptureAnchor` marker at this expansion's entry.
fn build_folds_of_deferral(
    env: &GlobalEnv,
    loc: &Loc,
    deferred: &FoldsOfDeferred,
    i_arg: &Exp,
) -> Exp {
    let u64_ty = Type::new_prim(PrimitiveType::U64);
    let j_sym = deferred.j_sym;
    // The composed index lambda `|j| (A1, .., An)`.
    let composed = deferred.components.clone();
    let comp_tys: Vec<Type> = composed
        .iter()
        .map(|comp| env.get_node_type(comp.node_id()))
        .collect();
    let (g_body, g_body_ty) = if let [comp] = composed.as_slice() {
        (comp.clone(), comp_tys[0].clone())
    } else {
        let body_ty = Type::Tuple(comp_tys);
        (
            ExpData::Call(
                env.new_node(loc.clone(), body_ty.clone()),
                Operation::Tuple,
                composed,
            )
            .into_exp(),
            body_ty,
        )
    };
    let g_fun_ty = Type::Fun(
        Box::new(u64_ty.clone()),
        Box::new(g_body_ty),
        AbilitySet::EMPTY,
    );
    let g = ExpData::Lambda(
        env.new_node(loc.clone(), g_fun_ty),
        Pattern::Var(env.new_node(loc.clone(), u64_ty.clone()), j_sym),
        g_body,
        LambdaCaptureKind::Default,
        None,
    )
    .into_exp();
    // The rewritten anchored occurrence.
    let occurrence = ExpData::Call(
        env.new_bool_node(loc),
        Operation::Behavior(BehaviorKind::FoldsOf, MemoryRange {
            pre: Some(deferred.label),
            post: None,
        }),
        vec![deferred.target.clone(), g, i_arg.clone()],
    )
    .into_exp();
    let mut conjuncts = vec![occurrence];
    if !deferred.prelude_aborts.is_empty() {
        // The prefix no-abort condition of the forwarder's own aborts,
        // expressible right here (the deferral covers only the
        // application's abort behavior).
        let no_abort =
            ExpData::Call(env.new_bool_node(loc), Operation::Not, vec![env
                .new_bool_join(
                    loc,
                    Operation::Or,
                    deferred.prelude_aborts.clone(),
                    false,
                )])
            .into_exp();
        let zero = ExpData::Value(
            env.new_node(loc.clone(), u64_ty.clone()),
            Value::Number(BigInt::from(0)),
        )
        .into_exp();
        let range = ExpData::Call(
            env.new_node(loc.clone(), Type::Primitive(PrimitiveType::Range)),
            Operation::Range,
            vec![zero, i_arg.clone()],
        )
        .into_exp();
        let j_pat = Pattern::Var(env.new_node(loc.clone(), u64_ty), j_sym);
        conjuncts.push(
            ExpData::Quant(
                env.new_bool_node(loc),
                QuantKind::Forall,
                vec![(j_pat, range)],
                vec![],
                None,
                no_abort,
            )
            .into_exp(),
        );
    }
    env.new_bool_join(loc, Operation::And, conjuncts, true)
}

/// Substitutes free occurrences of local variables by the given
/// expressions, honoring shadowing. Used to instantiate the lambda-scope
/// abort conditions of a `folds_of` resolution at a concrete iteration.
fn substitute_free_locals(exp: &Exp, subst: &BTreeMap<Symbol, Exp>) -> Exp {
    struct Substituter<'a> {
        subst: &'a BTreeMap<Symbol, Exp>,
        shadowed: Vec<BTreeSet<Symbol>>,
    }
    impl ExpRewriterFunctions for Substituter<'_> {
        fn rewrite_enter_scope<'b>(
            &mut self,
            _id: NodeId,
            vars: impl Iterator<Item = &'b (NodeId, Symbol)>,
        ) {
            self.shadowed.push(vars.map(|(_, sym)| *sym).collect());
        }

        fn rewrite_exit_scope(&mut self, _id: NodeId) {
            self.shadowed.pop();
        }

        fn rewrite_local_var(&mut self, _id: NodeId, sym: Symbol) -> Option<Exp> {
            if self.shadowed.iter().any(|scope| scope.contains(&sym)) {
                None
            } else {
                self.subst.get(&sym).cloned()
            }
        }
    }
    Substituter {
        subst,
        shadowed: vec![],
    }
    .rewrite_exp(exp.clone())
}

/// Anchors `old(parameter)` at inline-expansion entry. Verifier temporaries
/// avoid Move `copy` and `drop` requirements; other `old(..)` forms retain
/// function-entry semantics.
fn anchor_param_old_at_expansion_entry(
    env: &GlobalEnv,
    body: Exp,
    parameters: &[Parameter],
    call_site_loc: &Loc,
) -> Exp {
    // Fast path: no `old(<parameter>)` occurrence in the body.
    if !body.any(&mut |e| {
        matches!(e, ExpData::Call(_, Operation::Old, args)
            if matches!(args.first().map(|a| a.as_ref()), Some(ExpData::Temporary(..))))
    }) {
        return body;
    }
    struct Anchorer<'a> {
        env: &'a GlobalEnv,
        parameters: &'a [Parameter],
        label: MemoryLabel,
        anchored: bool,
    }
    impl ExpRewriterFunctions for Anchorer<'_> {
        fn rewrite_call(&mut self, id: NodeId, oper: &Operation, args: &[Exp]) -> Option<Exp> {
            if !matches!(oper, Operation::Old) {
                return None;
            }
            let [arg] = args else {
                return None;
            };
            let ExpData::Temporary(_, idx) = arg.as_ref() else {
                return None;
            };
            let param = self.parameters.get(*idx)?;
            if matches!(param.1, Type::Fun(..)) {
                return None;
            }
            let env = self.env;
            let loc = env.get_node_loc(id);
            let value_ty = param.1.skip_reference().clone();
            // Anchor the binding; a forwarded argument may change before entry.
            let mut value =
                ExpData::LocalVar(env.new_node(loc.clone(), param.1.clone()), param.0).into_exp();
            if param.1.is_reference() {
                value = ExpData::Call(
                    env.new_node(loc.clone(), value_ty.clone()),
                    Operation::Deref,
                    vec![value],
                )
                .into_exp();
            }
            let old = ExpData::Call(
                env.new_node(loc.clone(), value_ty.clone()),
                Operation::Old,
                vec![value],
            )
            .into_exp();
            self.anchored = true;
            Some(
                ExpData::Call(
                    env.new_node(loc, value_ty),
                    Operation::WithStateAnchor(self.label),
                    vec![old],
                )
                .into_exp(),
            )
        }
    }
    let label = MemoryLabel::new(env.new_global_id().as_usize());
    let mut anchorer = Anchorer {
        env,
        parameters,
        label,
        anchored: false,
    };
    let result = anchorer.rewrite_exp(body);
    if anchorer.anchored {
        prepend_folds_anchor_marker(env, call_site_loc, result, label)
    } else {
        result
    }
}

/// Freshens verifier-state labels consistently for each expansion.
fn freshen_folds_anchor_labels(env: &GlobalEnv, body: Exp) -> Exp {
    let mut labels = BTreeSet::new();
    body.visit_pre_order(&mut |e| {
        match e {
            ExpData::Call(_, Operation::FoldsCaptureAnchor(label), _) => {
                labels.insert(*label);
            },
            ExpData::Call(_, Operation::Behavior(BehaviorKind::FoldsOf, range), _) => {
                labels.extend(range.pre);
                labels.extend(range.post);
            },
            _ => {},
        }
        true
    });
    if labels.is_empty() {
        return body;
    }
    struct Freshener {
        map: BTreeMap<MemoryLabel, MemoryLabel>,
    }
    impl Freshener {
        fn freshen(&self, label: MemoryLabel) -> MemoryLabel {
            self.map.get(&label).copied().unwrap_or(label)
        }
    }
    impl ExpRewriterFunctions for Freshener {
        fn rewrite_call(&mut self, id: NodeId, oper: &Operation, args: &[Exp]) -> Option<Exp> {
            let new_oper = match oper {
                Operation::FoldsCaptureAnchor(label) => {
                    Some(Operation::FoldsCaptureAnchor(self.freshen(*label)))
                },
                Operation::WithStateAnchor(label) if self.map.contains_key(label) => {
                    Some(Operation::WithStateAnchor(self.freshen(*label)))
                },
                Operation::Behavior(BehaviorKind::FoldsOf, range) if !range.is_default() => {
                    Some(Operation::Behavior(BehaviorKind::FoldsOf, MemoryRange {
                        pre: range.pre.map(|l| self.freshen(l)),
                        post: range.post.map(|l| self.freshen(l)),
                    }))
                },
                _ => None,
            };
            new_oper.map(|oper| ExpData::Call(id, oper, args.to_vec()).into_exp())
        }
    }
    Freshener {
        map: labels
            .into_iter()
            .map(|label| (label, MemoryLabel::new(env.new_global_id().as_usize())))
            .collect(),
    }
    .rewrite_exp(body)
}

/// Builds the marker used for fold captures and inline-parameter snapshots.
fn mk_folds_anchor_marker(env: &GlobalEnv, loc: &Loc, label: MemoryLabel) -> Exp {
    let marker_exp = ExpData::Call(
        env.new_bool_node(loc),
        Operation::FoldsCaptureAnchor(label),
        vec![],
    )
    .into_exp();
    ExpData::SpecBlock(env.new_node(loc.clone(), Type::unit()), Spec {
        conditions: vec![Condition {
            loc: loc.clone(),
            kind: ConditionKind::Assume,
            properties: Default::default(),
            exp: marker_exp,
            additional_exps: vec![],
        }],
        ..Spec::default()
    })
    .into_exp()
}

/// Marks the snapshot point of captures and bound inline parameters.
fn prepend_folds_anchor_marker(env: &GlobalEnv, loc: &Loc, body: Exp, label: MemoryLabel) -> Exp {
    let body_ty = env.get_node_type(body.node_id());
    ExpData::Sequence(env.new_node(loc.clone(), body_ty), vec![
        mk_folds_anchor_marker(env, loc, label),
        body,
    ])
    .into_exp()
}

/// Adds a derivation summary while asserting it against the executable result.
fn prepend_inline_call_summary(
    env: &GlobalEnv,
    loc: &Loc,
    body: Exp,
    result: Exp,
    aborts: Exp,
) -> Exp {
    let body_ty = env.get_node_type(body.node_id());
    let marker_exp = ExpData::Call(env.new_bool_node(loc), Operation::InlineCallSummary, vec![
        result.clone(),
        aborts,
    ])
    .into_exp();
    let marker = ExpData::SpecBlock(env.new_node(loc.clone(), Type::unit()), Spec {
        conditions: vec![Condition {
            loc: loc.clone(),
            kind: ConditionKind::Assume,
            properties: Default::default(),
            exp: marker_exp,
            additional_exps: vec![],
        }],
        ..Spec::default()
    })
    .into_exp();

    let result_sym = env.symbol_pool().make(&format!(
        "$inline_summary_result_{}",
        env.new_global_id().as_usize()
    ));
    let result_pattern = Pattern::Var(env.new_node(loc.clone(), body_ty.clone()), result_sym);
    let actual =
        ExpData::LocalVar(env.new_node(loc.clone(), body_ty.clone()), result_sym).into_exp();
    let eq_id = env.new_bool_node(loc);
    env.set_node_instantiation(eq_id, vec![body_ty.clone()]);
    let equality = ExpData::Call(eq_id, Operation::Eq, vec![actual, result]).into_exp();
    let check = ExpData::SpecBlock(env.new_node(loc.clone(), Type::unit()), Spec {
        conditions: vec![Condition {
            loc: loc.clone(),
            kind: ConditionKind::Assert,
            properties: Default::default(),
            exp: equality,
            additional_exps: vec![],
        }],
        ..Spec::default()
    })
    .into_exp();
    let returned =
        ExpData::LocalVar(env.new_node(loc.clone(), body_ty.clone()), result_sym).into_exp();
    let checked_body = ExpData::Block(
        env.new_node(loc.clone(), body_ty.clone()),
        result_pattern,
        Some(body),
        ExpData::Sequence(env.new_node(loc.clone(), body_ty.clone()), vec![
            check, returned,
        ])
        .into_exp(),
    )
    .into_exp();
    ExpData::Sequence(env.new_node(loc.clone(), body_ty), vec![
        marker,
        checked_body,
    ])
    .into_exp()
}

/// Returns the label of a `FoldsCaptureAnchor` statement.
fn folds_anchor_marker_label(exp: &Exp) -> Option<MemoryLabel> {
    let ExpData::SpecBlock(_, spec) = exp.as_ref() else {
        return None;
    };
    let [cond] = spec.conditions.as_slice() else {
        return None;
    };
    match cond.exp.as_ref() {
        ExpData::Call(_, Operation::FoldsCaptureAnchor(label), _) => Some(*label),
        _ => None,
    }
}

/// Removes unreferenced `FoldsCaptureAnchor` markers.
fn prune_folds_anchor_markers(body: Exp) -> Exp {
    let mut referenced: BTreeSet<MemoryLabel> = BTreeSet::new();
    body.visit_pre_order(&mut |e| {
        match e {
            ExpData::Call(_, Operation::Behavior(BehaviorKind::FoldsOf, range), _) => {
                referenced.extend(range.pre);
                referenced.extend(range.post);
            },
            ExpData::Call(_, Operation::WithStateAnchor(label), _) => {
                referenced.insert(*label);
            },
            _ => {},
        }
        true
    });
    struct MarkerPass {
        referenced: BTreeSet<MemoryLabel>,
    }
    impl ExpRewriterFunctions for MarkerPass {
        fn rewrite_exp(&mut self, exp: Exp) -> Exp {
            let exp = self.rewrite_exp_descent(exp);
            let ExpData::Sequence(id, exps) = exp.as_ref() else {
                return exp;
            };
            if !exps.iter().any(|e| folds_anchor_marker_label(e).is_some()) {
                return exp;
            }
            let result: Vec<Exp> = exps
                .iter()
                .filter(|e| {
                    folds_anchor_marker_label(e)
                        .is_none_or(|label| self.referenced.contains(&label))
                })
                .cloned()
                .collect();
            match result.as_slice() {
                [single] => single.clone(),
                _ => ExpData::Sequence(*id, result).into_exp(),
            }
        }
    }
    let mut pass = MarkerPass { referenced };
    pass.rewrite_exp(body)
}

// ======================================================================================
// Specialization of spec functions over lambda arguments

/// Collects the type parameter indices referenced by a type.
fn used_type_params_in_type(ty: &Type, acc: &mut BTreeSet<u16>) {
    ty.visit(&mut |t| {
        if let Type::TypeParameter(idx) = t {
            acc.insert(*idx);
        }
    });
}

/// Collects the type parameter indices referenced by an expression: the
/// types and instantiations of all nodes, including pattern nodes.
fn used_type_params_in_exp(env: &GlobalEnv, exp: &Exp, acc: &mut BTreeSet<u16>) {
    fn visit_pat(env: &GlobalEnv, pat: &Pattern, acc: &mut BTreeSet<u16>) {
        pat.visit_pre_post(&mut |_, p| {
            used_type_params_in_type(&env.get_node_type(p.node_id()), acc);
            if let Pattern::Struct(_, sid, ..) = p {
                for ty in &sid.inst {
                    used_type_params_in_type(ty, acc);
                }
            }
        });
    }
    exp.visit_pre_order(&mut |e| {
        used_type_params_in_type(&env.get_node_type(e.node_id()), acc);
        for ty in env.get_node_instantiation(e.node_id()) {
            used_type_params_in_type(&ty, acc);
        }
        match e {
            ExpData::Lambda(_, pat, ..)
            | ExpData::Block(_, pat, ..)
            | ExpData::Assign(_, pat, _) => visit_pat(env, pat, acc),
            ExpData::Match(_, _, arms) => {
                for arm in arms {
                    visit_pat(env, &arm.pattern, acc);
                }
            },
            ExpData::Quant(_, _, ranges, ..) => {
                for (pat, _) in ranges {
                    visit_pat(env, pat, acc);
                }
            },
            _ => {},
        }
        true
    });
}

/// The type parameter substitution derived from the type parameter indices
/// a specialization's material mentions (see `specialize`): the mentioned
/// indices in ascending order become the compact parameters `0..k` of the
/// specialized declaration. Returns the declared parameters, the type
/// arguments call sites pass (the mentioned parameters, in the enclosing
/// context's own index space), and the substitution mapping the enclosing
/// index space to the compact one (empty when nothing is mentioned).
fn compact_type_param_mapping(
    env: &GlobalEnv,
    loc: &Loc,
    used_params: &BTreeSet<u16>,
) -> (Vec<TypeParameter>, Vec<Type>, Vec<Type>) {
    let Some(max) = used_params.iter().next_back() else {
        return (vec![], vec![], vec![]);
    };
    let mut remap: Vec<Type> = (0..=*max).map(Type::TypeParameter).collect();
    let mut type_params = vec![];
    for (j, i) in used_params.iter().enumerate() {
        remap[*i as usize] = Type::TypeParameter(j as u16);
        type_params.push(TypeParameter(
            env.symbol_pool().make(&format!("T{}", j)),
            TypeParameterKind::default(),
            loc.clone(),
        ));
    }
    let type_args = used_params
        .iter()
        .map(|i| Type::TypeParameter(*i))
        .collect();
    (type_params, type_args, remap)
}

/// Instantiates type parameters in an expression, covering pattern node
/// types and struct pattern instantiations in addition to the expression
/// nodes `ExpRewriter::set_type_args` handles.
fn instantiate_exp_with_patterns(env: &GlobalEnv, exp: Exp, type_args: &[Type]) -> Exp {
    struct Instantiator<'a> {
        env: &'a GlobalEnv,
        type_args: &'a [Type],
    }
    impl Instantiator<'_> {
        fn instantiate_pattern_id(&self, id: NodeId) -> Option<NodeId> {
            ExpData::instantiate_node(self.env, id, self.type_args)
        }
    }
    impl ExpRewriterFunctions for Instantiator<'_> {
        fn rewrite_node_id(&mut self, id: NodeId) -> Option<NodeId> {
            ExpData::instantiate_node(self.env, id, self.type_args)
        }

        fn rewrite_pattern(&mut self, pat: &Pattern, _creating_scope: bool) -> Option<Pattern> {
            // Sub-patterns have already been rewritten when this is called;
            // only the pattern's own node (and struct instantiation) is
            // handled here.
            match pat {
                Pattern::Var(id, sym) => self
                    .instantiate_pattern_id(*id)
                    .map(|new_id| Pattern::Var(new_id, *sym)),
                Pattern::Wildcard(id) => self.instantiate_pattern_id(*id).map(Pattern::Wildcard),
                Pattern::Tuple(id, pats) => self
                    .instantiate_pattern_id(*id)
                    .map(|new_id| Pattern::Tuple(new_id, pats.clone())),
                Pattern::Struct(id, sid, variant, pats) => {
                    let new_id = self.instantiate_pattern_id(*id);
                    let new_inst = Type::instantiate_slice(&sid.inst, self.type_args);
                    if new_id.is_none() && new_inst == sid.inst {
                        None
                    } else {
                        let mut new_sid = sid.clone();
                        new_sid.inst = new_inst;
                        Some(Pattern::Struct(
                            new_id.unwrap_or(*id),
                            new_sid,
                            *variant,
                            pats.clone(),
                        ))
                    }
                },
                Pattern::LiteralValue(id, value) => self
                    .instantiate_pattern_id(*id)
                    .map(|new_id| Pattern::LiteralValue(new_id, value.clone())),
                Pattern::Range(id, lo, hi, inclusive) => self
                    .instantiate_pattern_id(*id)
                    .map(|new_id| Pattern::Range(new_id, lo.clone(), hi.clone(), *inclusive)),
                Pattern::Error(id) => self.instantiate_pattern_id(*id).map(Pattern::Error),
            }
        }
    }
    if type_args.is_empty() {
        return exp;
    }
    Instantiator { env, type_args }.rewrite_exp(exp)
}

/// Key of a spec function specialization within one context: the function,
/// its type instantiation, and the argument positions bound to lambdas,
/// each identified by the function parameter supplying the lambda — within
/// a context, the parameter symbol determines the lambda, so distinct
/// parameters at the same position map to distinct specializations. Literal
/// lambda arguments have no such symbol and resolve through the global
/// unifier instead.
type SpecFunSpecKey = (QualifiedId<SpecFunId>, Vec<Type>, Vec<(usize, Symbol)>);

/// A globally unified spec function specialization: within one inliner run,
/// requests for the same spec function, instantiation, and spec-equivalent
/// lambdas (see `ExpData::is_spec_equivalent`) at the same argument positions
/// share one specialized function. This makes occurrences from different
/// contexts definitionally equal — e.g. a `spec_fold(f, ..)` loop invariant
/// expanded with a lambda argument, and a lemma or caller spec restating that
/// lambda literally, resolve to the same specialization, which is what makes
/// facts about the expansion provable outside of it.
struct SpecFunUnifierEntry {
    target: QualifiedId<SpecFunId>,
    inst: Vec<Type>,
    bindings: Vec<(usize, Exp)>,
    /// The shared specialization, or `None` if specializing this key failed
    /// (an error has been reported). Spec-equivalent requests from any
    /// context then fail without a new attempt or a repeated error.
    spec: Option<SpecFunSpecialization>,
}

/// A context argument of a specialized spec function: a free variable of the
/// substituted lambda material, lifted into a parameter. `sym` is the source
/// symbol, naming the caller-side value at call sites. `param_sym` names the
/// parameter inside the specialized function; it equals `sym` unless that
/// would collide with a retained parameter, a binder in the spec function's
/// body, or another context parameter, in which case it is freshened. `temp`
/// is set if the variable is a parameter of the enclosing function referenced
/// as a temporary.
#[derive(Clone)]
struct CtxArg {
    sym: Symbol,
    param_sym: Symbol,
    temp: Option<TempIndex>,
    /// Type of the caller-side expression. This can be a reference even
    /// though the generated spec parameter in `ty` is value-level.
    caller_ty: Type,
    ty: Type,
}

#[derive(Clone)]
struct SpecFunSpecialization {
    /// The specialized function.
    qid: QualifiedId<SpecFunId>,
    /// The original argument positions which are retained.
    retained: Vec<usize>,
    /// Context arguments appended after the retained ones.
    ctx_args: Vec<CtxArg>,
    /// The instantiated result type, in the enclosing context's type
    /// parameter space (call sites are written in that space).
    result_type: Type,
    /// The type arguments call sites pass: the enclosing context's type
    /// parameters mentioned by the specialization's material, in ascending
    /// index order — matching the specialized declaration's own compact
    /// type parameters (see `compact_type_param_mapping`). Empty for a
    /// specialization over fully concrete material.
    type_args: Vec<Type>,
    underivable_behavior: Rc<Cell<Option<BehaviorKind>>>,
}

impl SpecFunSpecialization {
    fn underivable_behavior(&self, env: &GlobalEnv) -> Option<BehaviorKind> {
        self.underivable_behavior.get().or_else(|| {
            env.get_spec_fun(self.qid)
                .body
                .as_ref()
                .and_then(|body| underivable_concrete_behavior(env, body))
        })
    }

    /// Constructs the redirected call to the specialized function: the given
    /// retained arguments, followed by the materialized context arguments.
    /// Context arguments backed by parameters of the enclosing function are
    /// materialized as temporaries (inside a specialized body, the temporary
    /// replacement pass turns them into the enclosing context parameters);
    /// the others reference the like-named local, translated through
    /// `ctx_renames` — the freshening map of the enclosing specialized body —
    /// when the call is emitted into one.
    fn make_call(
        &self,
        env: &GlobalEnv,
        loc: Loc,
        range: &MemoryRange,
        retained_args: Vec<Exp>,
        ctx_renames: &BTreeMap<Symbol, Symbol>,
    ) -> Exp {
        self.make_call_with_context_values(
            env,
            loc,
            range,
            retained_args,
            ctx_renames,
            &BTreeMap::new(),
        )
    }

    fn make_call_with_context_values(
        &self,
        env: &GlobalEnv,
        loc: Loc,
        range: &MemoryRange,
        mut retained_args: Vec<Exp>,
        ctx_renames: &BTreeMap<Symbol, Symbol>,
        ctx_values: &BTreeMap<Symbol, Exp>,
    ) -> Exp {
        for ctx in &self.ctx_args {
            if let Some(value) = ctx_values.get(&ctx.sym) {
                retained_args.push(value.clone());
                continue;
            }
            let node = env.new_node(loc.clone(), ctx.caller_ty.clone());
            retained_args.push(match ctx.temp {
                Some(idx) => ExpData::Temporary(node, idx).into_exp(),
                None => {
                    let sym = ctx_renames.get(&ctx.sym).copied().unwrap_or(ctx.sym);
                    ExpData::LocalVar(node, sym).into_exp()
                },
            });
        }
        let node = env.new_node(loc, self.result_type.clone());
        if !self.type_args.is_empty() {
            env.set_node_instantiation(node, self.type_args.clone());
        }
        ExpData::Call(
            node,
            Operation::SpecFunction(self.qid.module_id, self.qid.id, range.clone()),
            retained_args,
        )
        .into_exp()
    }
}

/// Specializes spec functions which are called with lambda-bound function
/// parameters as arguments. A spec function taking a function value (e.g. a
/// recursive `spec fun spec_fold(f: |A, &T| A, ..)` used in the loop invariant
/// of an inline `fold`) cannot be called with a lambda after expansion, since
/// lambdas are beta-reduced and never become function values. Instead, a
/// monomorphic copy is generated in which the function parameter is
/// eliminated: behavioral predicates over it are substituted by the lambda's
/// spec (see `substitute_bp_by_lambda_spec`), applications are beta-reduced,
/// and calls to spec functions passing it along (including recursive calls)
/// are redirected to their specializations. Free variables of the lambda
/// material become additional parameters of the copy, named by the same
/// symbols and supplied at every (redirected) call site.
struct SpecFunSpecializer<'env, 'unifier> {
    env: &'env mut GlobalEnv,
    /// The function into which the expansion happens.
    target_fun: Option<QualifiedFunId>,
    /// Context used when behavioral predicates in the specialized body are
    /// resolved. Fold-transformer failures are reported by `folds_of`.
    bp_context: BpContext,
    /// Parameters of the enclosing function, for resolving temporaries in
    /// lambda material.
    enclosing_params: Vec<Parameter>,
    cache: BTreeMap<SpecFunSpecKey, SpecFunSpecialization>,
    /// The global unifier of the inliner run; specializations are shared
    /// across contexts through it.
    unifier: &'unifier mut Vec<SpecFunUnifierEntry>,
}

impl<'env, 'unifier> SpecFunSpecializer<'env, 'unifier> {
    fn new(
        env: &'env mut GlobalEnv,
        target_fun: Option<QualifiedFunId>,
        unifier: &'unifier mut Vec<SpecFunUnifierEntry>,
    ) -> Self {
        let enclosing_params = target_fun
            .map(|qid| env.get_function(qid).get_parameters())
            .unwrap_or_default();
        Self {
            env,
            target_fun,
            bp_context: BpContext::SpecFunBody,
            enclosing_params,
            cache: BTreeMap::new(),
            unifier,
        }
    }

    /// Scans `body` for calls to spec functions with lambda-bound arguments
    /// and generates the needed specializations.
    fn run(
        env: &'env mut GlobalEnv,
        body: &Exp,
        type_args: &[Type],
        parameters: &[Parameter],
        lambda_param_map: &BTreeMap<Symbol, &Exp>,
        target_fun: Option<QualifiedFunId>,
        unifier: &'unifier mut Vec<SpecFunUnifierEntry>,
    ) -> BTreeMap<SpecFunSpecKey, SpecFunSpecialization> {
        if lambda_param_map.is_empty() {
            return BTreeMap::new();
        }
        let mut requests = vec![];
        body.visit_pre_order(&mut |e| {
            if let ExpData::Call(id, Operation::SpecFunction(mid, sid, _), args) = e {
                let bindings = collect_lambda_bindings(args, parameters, lambda_param_map);
                if !bindings.is_empty() {
                    let inst = env.get_node_instantiation(*id);
                    let inst = if type_args.is_empty() {
                        inst
                    } else {
                        Type::instantiate_vec(inst, type_args)
                    };
                    requests.push((env.get_node_loc(*id), mid.qualified(*sid), inst, bindings));
                }
            }
            true
        });
        if requests.is_empty() {
            return BTreeMap::new();
        }
        let mut specializer = SpecFunSpecializer::new(env, target_fun, unifier);
        for (loc, qid, inst, bindings) in requests {
            let key = (
                qid,
                inst.clone(),
                bindings.iter().map(|(pos, sym, _)| (*pos, *sym)).collect(),
            );
            let bindings = bindings
                .into_iter()
                .map(|(pos, _, lambda)| (pos, lambda))
                .collect();
            specializer.specialize(&loc, qid, inst, bindings, Some(key));
        }
        specializer.cache
    }

    /// Ensures a specialization of `qid` instantiated with `inst` exists for
    /// the parameters at the `bindings` positions bound to the given lambdas,
    /// and returns it. `cache_key` memoizes the resolution per context, for
    /// lambdas identified by a function parameter; literal lambdas pass
    /// `None` and resolve through the global unifier. On failure, an error
    /// is reported, no usable cache or unifier entry remains, and `None` is
    /// returned. This includes failures while rewriting the body (e.g. the
    /// body forwards the eliminated parameter to a function which cannot be
    /// specialized): the specialization, which is registered before the body
    /// rewrite to serve recursive calls, is then invalidated, so all callers
    /// take the leave-intact failure path.
    fn specialize(
        &mut self,
        loc: &Loc,
        qid: QualifiedId<SpecFunId>,
        inst: Vec<Type>,
        bindings: Vec<(usize, Exp)>,
        cache_key: Option<SpecFunSpecKey>,
    ) -> Option<SpecFunSpecialization> {
        if let Some(key) = &cache_key {
            if let Some(spec) = self.cache.get(key) {
                return Some(spec.clone());
            }
        }
        // Consult the global unifier: reuse a spec-equivalent specialization
        // from another context, making both contexts refer to the same
        // specialized function.
        let env: &GlobalEnv = self.env;
        if let Some(entry) = self.unifier.iter().find(|e| {
            e.target == qid
                && e.inst == inst
                && e.bindings.len() == bindings.len()
                && e.bindings
                    .iter()
                    .zip(&bindings)
                    .all(|((pos1, lambda1), (pos2, lambda2))| {
                        pos1 == pos2 && lambda1.as_ref().is_spec_equivalent(env, lambda2.as_ref())
                    })
        }) {
            let Some(spec) = entry.spec.clone() else {
                // Specializing this key failed before; the error has already
                // been reported.
                return None;
            };
            if let Some(key) = cache_key {
                self.cache.insert(key, spec.clone());
            }
            return Some(spec);
        }
        let decl = self.env.get_spec_fun(qid).clone();
        let Some(orig_body) = decl.body.clone() else {
            spec_error(
                self.env,
                loc,
                &format!(
                    "cannot pass a lambda to native or uninterpreted spec function `{}`",
                    decl.name.display(self.env.symbol_pool())
                ),
            );
            return None;
        };
        // Compute the context arguments: free variables of the lambdas (and
        // parameters of the enclosing function they use), lifted into
        // parameters named by the same symbols. Reference types are
        // stripped: specifications are value-level, and the translation
        // passes the referenced value at call sites.
        let mut ctx_args: Vec<CtxArg> = vec![];
        for (_, lambda) in &bindings {
            for (sym, ty) in lambda.free_vars_with_types(self.env) {
                if !ctx_args.iter().any(|c| c.temp.is_none() && c.sym == sym) {
                    ctx_args.push(CtxArg {
                        sym,
                        param_sym: sym,
                        temp: None,
                        ty: ty.skip_reference().clone(),
                        caller_ty: ty,
                    });
                }
            }
            for (idx, ty) in lambda.used_temporaries_with_types(self.env) {
                if let Some(Parameter(sym, ..)) = self.enclosing_params.get(idx) {
                    if !ctx_args.iter().any(|c| c.temp == Some(idx)) {
                        ctx_args.push(CtxArg {
                            sym: *sym,
                            param_sym: *sym,
                            temp: Some(idx),
                            ty: ty.skip_reference().clone(),
                            caller_ty: ty,
                        });
                    }
                } else {
                    self.env.diag(
                        Severity::Bug,
                        loc,
                        "unresolved parameter reference in lambda argument",
                    );
                }
            }
        }
        let bound: BTreeSet<usize> = bindings.iter().map(|(pos, _)| *pos).collect();
        let retained: Vec<usize> = (0..decl.params.len())
            .filter(|pos| !bound.contains(pos))
            .collect();
        // Freshen context parameter symbols which would collide with a
        // retained parameter, a binder in the spec function's body, or
        // another context parameter: the specialized body must distinguish
        // the spliced lambda material's captures from like-named parameters
        // and let/quantifier bindings around the substitution sites.
        let mut taken: BTreeSet<Symbol> = retained
            .iter()
            .map(|pos| decl.params[*pos].0)
            .chain(binder_syms(&orig_body))
            .collect();
        let mut ctx_renames: BTreeMap<Symbol, Symbol> = BTreeMap::new();
        for ctx in ctx_args.iter_mut() {
            if taken.contains(&ctx.sym) {
                let base = ctx.sym.display(self.env.symbol_pool()).to_string();
                let mut count = 0;
                ctx.param_sym = loop {
                    let candidate = self
                        .env
                        .symbol_pool()
                        .make(&format!("{}$ctx{}", base, count));
                    if !taken.contains(&candidate) {
                        break candidate;
                    }
                    count += 1;
                };
                if ctx.temp.is_none() {
                    ctx_renames.insert(ctx.sym, ctx.param_sym);
                }
            }
            taken.insert(ctx.param_sym);
        }
        // The specialization may stem from a generic enclosing context (a
        // generic function expanding the inline HOF, a generic lemma
        // restating its lambda): the instantiation and the lambda material
        // then reference the context's type parameters. The specialized
        // declaration is made parametric over exactly the mentioned
        // parameters, remapped to a compact index space; call sites pass
        // the mentioned parameters as type arguments. The mention set is
        // fully determined by `inst` and the lambda material — the same
        // data the global unifier compares — so spec-equivalent requests
        // from different generic contexts share one parametric
        // specialization and instantiate it with their own parameters.
        let mut used_params: BTreeSet<u16> = BTreeSet::new();
        for ty in &inst {
            used_type_params_in_type(ty, &mut used_params);
        }
        for (_, lambda) in &bindings {
            used_type_params_in_exp(self.env, lambda, &mut used_params);
        }
        let (type_params, spec_type_args, type_param_remap) =
            compact_type_param_mapping(self.env, loc, &used_params);
        let remap_ty = |ty: Type| -> Type {
            if type_param_remap.is_empty() {
                ty
            } else {
                ty.instantiate(&type_param_remap)
            }
        };
        let mut params: Vec<Parameter> = retained
            .iter()
            .map(|pos| {
                let Parameter(sym, ty, ploc) = &decl.params[*pos];
                Parameter(*sym, remap_ty(ty.instantiate(&inst)), ploc.clone())
            })
            .collect();
        params.extend(
            ctx_args
                .iter()
                .map(|c| Parameter(c.param_sym, remap_ty(c.ty.clone()), loc.clone())),
        );
        let result_type = decl.result_type.instantiate(&inst);
        // Specializations are added to the module of the function into which
        // the expansion happens, if known.
        let module_id = self
            .target_fun
            .map(|f| f.module_id)
            .unwrap_or(qid.module_id);
        let unifier_index = self.unifier.len();
        let name = self.env.symbol_pool().make(&format!(
            "{}$lambda${}",
            decl.name.display(self.env.symbol_pool()),
            unifier_index
        ));
        // Register the declaration before rewriting the body, so recursive
        // calls can be redirected through the cache.
        let new_qid = self.env.add_spec_function_def(module_id, SpecFunDecl {
            loc: decl.loc.clone(),
            name,
            type_params,
            params,
            result_type: remap_ty(result_type.clone()),
            used_memory: BTreeSet::new(),
            old_memory: BTreeSet::new(),
            uninterpreted: false,
            is_move_fun: decl.is_move_fun,
            is_native: false,
            body: None,
            callees: BTreeSet::new(),
            is_recursive: RefCell::new(None),
            insts_using_generic_type_reflection: RefCell::new(BTreeMap::new()),
            spec: RefCell::new(Spec::default()),
            uses_old: false,
            frame_spec: None,
        });
        let specialization = SpecFunSpecialization {
            qid: new_qid,
            retained,
            ctx_args: ctx_args.clone(),
            result_type,
            type_args: spec_type_args,
            underivable_behavior: Rc::new(Cell::new(None)),
        };
        self.unifier.push(SpecFunUnifierEntry {
            target: qid,
            inst: inst.clone(),
            bindings: bindings.clone(),
            spec: Some(specialization.clone()),
        });
        if let Some(key) = &cache_key {
            self.cache.insert(key.clone(), specialization.clone());
        }

        // Instantiate and rewrite the body.
        let inst_body = instantiate_exp_with_patterns(self.env, orig_body, &inst);
        // The lambdas are spliced into the specialized body with their free
        // variables renamed to the (possibly freshened) context parameters.
        let eliminated: BTreeMap<Symbol, Exp> = bindings
            .iter()
            .map(|(pos, lambda)| (decl.params[*pos].0, rename_free_vars(lambda, &ctx_renames)))
            .collect();
        let eliminated_original: BTreeMap<Symbol, Exp> = bindings
            .iter()
            .map(|(pos, lambda)| (decl.params[*pos].0, lambda.clone()))
            .collect();
        let mut body_rewriter = SpecFunBodyRewriter {
            specializer: self,
            eliminated: &eliminated,
            eliminated_original: &eliminated_original,
            ctx_renames: &ctx_renames,
            shadowed: vec![],
            failed: false,
        };
        let new_body = body_rewriter.rewrite_exp(inst_body);
        if body_rewriter.failed {
            // The body contains a use of an eliminated function parameter
            // which could not be resolved (an error has been reported):
            // installing it would leave a dangling reference to a parameter
            // the specialization no longer has. Poison the unifier entry so
            // all spec-equivalent requests take the failure path, drop the
            // cache entry, and leave the already registered declaration
            // behind as an unused uninterpreted function (declarations
            // cannot be removed, since nested specializations may have
            // registered later ones).
            self.unifier[unifier_index].spec = None;
            if let Some(key) = &cache_key {
                self.cache.remove(key);
            }
            self.env.get_spec_fun_mut(new_qid).uninterpreted = true;
            return None;
        }
        // Replace references to enclosing-function parameters (temporaries in
        // the spliced lambda material) by the context parameters.
        let temp_map: BTreeMap<TempIndex, (Symbol, Type)> = ctx_args
            .iter()
            .filter_map(|c| c.temp.map(|idx| (idx, (c.param_sym, c.ty.clone()))))
            .collect();
        let new_body = if temp_map.is_empty() {
            new_body
        } else {
            let env: &GlobalEnv = self.env;
            let mut replacer = |_id: NodeId, target: ExpRewriteTarget| match target {
                ExpRewriteTarget::Temporary(idx) => temp_map.get(&idx).map(|(sym, ty)| {
                    ExpData::LocalVar(env.new_node(loc.clone(), ty.clone()), *sym).into_exp()
                }),
                _ => None,
            };
            ExpRewriter::new(env, &mut replacer).rewrite_exp(new_body)
        };
        // Remap the body — built in the enclosing context's type parameter
        // space — to the declaration's compact type parameters. Calls to
        // (nested or recursive) specializations inside the body carry their
        // type arguments in context space and are remapped alongside.
        let new_body = instantiate_exp_with_patterns(self.env, new_body, &type_param_remap);
        specialization
            .underivable_behavior
            .set(underivable_concrete_behavior(self.env, &new_body));
        // Internal invariant: every free variable of the specialized body is
        // a parameter of the specialization; in particular, no reference to
        // an eliminated function parameter survives.
        let param_syms: BTreeSet<Symbol> = self
            .env
            .get_spec_fun(new_qid)
            .params
            .iter()
            .map(|Parameter(sym, ..)| *sym)
            .collect();
        if !new_body.free_vars().is_subset(&param_syms) {
            self.env.diag(
                Severity::Bug,
                loc,
                "dangling variable reference in the body of a specialized spec function",
            );
        }
        let callees = new_body.called_spec_funs(self.env);
        let new_decl = self.env.get_spec_fun_mut(new_qid);
        new_decl.body = Some(new_body);
        new_decl.callees = callees;
        Some(specialization)
    }
}

/// Rewrites the body of a spec function specialization, resolving all uses of
/// the eliminated function-typed parameters against their lambdas.
struct SpecFunBodyRewriter<'a, 'b, 'env, 'unifier> {
    specializer: &'a mut SpecFunSpecializer<'env, 'unifier>,
    /// The eliminated parameters and their lambdas, with free variables
    /// renamed to the context parameters; splices into the specialized body
    /// (behavioral predicate substitution, beta reduction) use these.
    eliminated: &'b BTreeMap<Symbol, Exp>,
    /// The eliminated parameters and their lambdas in caller scope, as
    /// passed in. Nested specialization requests use these, so their
    /// identity in the cache and the global unifier is independent of this
    /// specialization's freshening.
    eliminated_original: &'b BTreeMap<Symbol, Exp>,
    /// The freshening map of the specialization being built, translating
    /// context argument references of redirected calls.
    ctx_renames: &'b BTreeMap<Symbol, Symbol>,
    shadowed: Vec<BTreeSet<Symbol>>,
    /// Set when a use of an eliminated parameter could not be resolved (an
    /// error has been reported): the body then retains a dangling reference,
    /// and `specialize` invalidates the specialization as a whole.
    failed: bool,
}

impl SpecFunBodyRewriter<'_, '_, '_, '_> {
    fn is_shadowed(&self, sym: &Symbol) -> bool {
        self.shadowed.iter().any(|scope| scope.contains(sym))
    }

    /// Resolves an expression to the renamed lambda of an eliminated
    /// parameter, for splicing.
    fn resolve(&self, exp: &Exp) -> Option<Exp> {
        self.resolve_in(exp, self.eliminated)
            .map(|(_, lambda)| lambda)
    }

    /// Resolves an expression to the caller-scope lambda of an eliminated
    /// parameter and that parameter's symbol, for nested specialization.
    fn resolve_original(&self, exp: &Exp) -> Option<(Symbol, Exp)> {
        self.resolve_in(exp, self.eliminated_original)
    }

    fn resolve_in(&self, exp: &Exp, map: &BTreeMap<Symbol, Exp>) -> Option<(Symbol, Exp)> {
        if let ExpData::LocalVar(_, sym) = exp.as_ref() {
            if !self.is_shadowed(sym) {
                return map.get(sym).map(|lambda| (*sym, lambda.clone()));
            }
        }
        None
    }
}

impl ExpRewriterFunctions for SpecFunBodyRewriter<'_, '_, '_, '_> {
    fn rewrite_exp(&mut self, exp: Exp) -> Exp {
        match exp.as_ref() {
            ExpData::Call(id, Operation::Behavior(kind, range), args) => {
                if let Some(lambda) = args.first().and_then(|target| self.resolve(target)) {
                    let bp_args: Vec<Exp> = args[1..]
                        .iter()
                        .map(|arg| self.rewrite_exp(arg.clone()))
                        .collect();
                    let result = substitute_bp_by_lambda_spec(
                        self.specializer.env,
                        self.specializer.target_fun,
                        None,
                        self.specializer.bp_context,
                        false,
                        *id,
                        *kind,
                        range,
                        &lambda,
                        bp_args,
                        None,
                    );
                    if matches!(
                        result.as_ref(),
                        ExpData::Call(_, Operation::Behavior(..), args)
                            if args.first().is_some_and(
                                |target| matches!(target.as_ref(), ExpData::Lambda(..))
                            )
                    ) {
                        self.failed = true;
                    }
                    return result;
                }
            },
            ExpData::Call(id, Operation::SpecFunction(mid, sid, range), args) => {
                let bindings: Vec<(usize, Symbol, Exp)> = args
                    .iter()
                    .enumerate()
                    .filter_map(|(pos, arg)| {
                        self.resolve_original(arg)
                            .map(|(sym, lambda)| (pos, sym, lambda))
                    })
                    .collect();
                if !bindings.is_empty() {
                    let loc = self.specializer.env.get_node_loc(*id);
                    let inst = self.specializer.env.get_node_instantiation(*id);
                    let qid = mid.qualified(*sid);
                    let key = (
                        qid,
                        inst.clone(),
                        bindings.iter().map(|(pos, sym, _)| (*pos, *sym)).collect(),
                    );
                    let bindings = bindings
                        .into_iter()
                        .map(|(pos, _, lambda)| (pos, lambda))
                        .collect();
                    let Some(spec) =
                        self.specializer
                            .specialize(&loc, qid, inst, bindings, Some(key))
                    else {
                        // Error already reported by `specialize`. The call
                        // cannot be redirected and still references the
                        // eliminated parameter, so the enclosing
                        // specialization is invalid as a whole.
                        self.failed = true;
                        return exp;
                    };
                    let retained_args: Vec<Exp> = spec
                        .retained
                        .iter()
                        .map(|pos| self.rewrite_exp(args[*pos].clone()))
                        .collect();
                    // Inside a specialized body, context values are available
                    // as the enclosing context parameters.
                    return spec.make_call(
                        self.specializer.env,
                        loc,
                        range,
                        retained_args,
                        self.ctx_renames,
                    );
                }
            },
            ExpData::Invoke(id, target, args) => {
                if let Some(lambda) = self.resolve(target) {
                    // Spec function bodies are specifications: function values
                    // cannot be applied there. Only enforced in verify mode;
                    // in regular compilation, beta-reduce silently as before.
                    if self.specializer.env.is_verify_mode() {
                        self.failed = true;
                        self.specializer.env.error(
                            &self.specializer.env.get_node_loc(*id),
                            "a function value cannot be applied in a specification; \
                             use a behavioral predicate instead (e.g. `result_of<f>(..)` \
                             for the value of `f(..)`)",
                        );
                        return ExpData::Invalid(*id).into_exp();
                    }
                    if let ExpData::Lambda(_, pat, lambda_body, _, _) = lambda.as_ref() {
                        let loc = self.specializer.env.get_node_loc(*id);
                        let new_args: Vec<Exp> = args
                            .iter()
                            .map(|arg| self.rewrite_exp(arg.clone()))
                            .collect();
                        let tuple_pat = make_lambda_pattern_a_tuple(self.specializer.env, pat);
                        return InlinedRewriter::construct_inlined_call_expression(
                            self.specializer.env,
                            &loc,
                            lambda_body.clone(),
                            tuple_pat,
                            new_args,
                        );
                    }
                }
            },
            _ => {},
        }
        self.rewrite_exp_descent(exp)
    }

    fn rewrite_enter_scope<'c>(
        &mut self,
        _id: NodeId,
        vars: impl Iterator<Item = &'c (NodeId, Symbol)>,
    ) {
        self.shadowed.push(vars.map(|(_, sym)| *sym).collect());
    }

    fn rewrite_exit_scope(&mut self, _id: NodeId) {
        self.shadowed.pop();
    }

    fn rewrite_local_var(&mut self, id: NodeId, sym: Symbol) -> Option<Exp> {
        if !self.is_shadowed(&sym) && self.eliminated.contains_key(&sym) {
            self.failed = true;
            spec_error(
                self.specializer.env,
                &self.specializer.env.get_node_loc(id),
                "a function-typed parameter of a spec function can only be applied, \
                 used as a behavioral predicate target, or passed to another spec \
                 function, when specialized over a lambda",
            );
        }
        None
    }
}

/// Collects the symbols bound by any binder (let, lambda, quantifier range,
/// match arm) within the expression.
fn binder_syms(exp: &Exp) -> BTreeSet<Symbol> {
    let mut bound = BTreeSet::new();
    exp.visit_pre_order(&mut |e| {
        let mut add = |pat: &Pattern| bound.extend(pat.vars().into_iter().map(|(_, sym)| sym));
        match e {
            ExpData::Block(_, pat, ..) | ExpData::Lambda(_, pat, ..) => add(pat),
            ExpData::Quant(_, _, ranges, ..) => ranges.iter().for_each(|(pat, _)| add(pat)),
            ExpData::Match(_, _, arms) => arms.iter().for_each(|arm| add(&arm.pattern)),
            _ => {},
        }
        true
    });
    bound
}

/// Renames free occurrences of local variables in `exp` (including
/// assignment targets) according to `renames`, honoring shadowing.
fn rename_free_vars(exp: &Exp, renames: &BTreeMap<Symbol, Symbol>) -> Exp {
    if renames.is_empty() {
        return exp.clone();
    }
    struct Renamer<'a> {
        renames: &'a BTreeMap<Symbol, Symbol>,
        shadowed: Vec<BTreeSet<Symbol>>,
    }
    impl Renamer<'_> {
        fn rename(&self, sym: Symbol) -> Option<Symbol> {
            if self.shadowed.iter().any(|scope| scope.contains(&sym)) {
                None
            } else {
                self.renames.get(&sym).copied()
            }
        }
    }
    impl ExpRewriterFunctions for Renamer<'_> {
        fn rewrite_enter_scope<'b>(
            &mut self,
            _id: NodeId,
            vars: impl Iterator<Item = &'b (NodeId, Symbol)>,
        ) {
            self.shadowed.push(vars.map(|(_, sym)| *sym).collect());
        }

        fn rewrite_exit_scope(&mut self, _id: NodeId) {
            self.shadowed.pop();
        }

        fn rewrite_local_var(&mut self, id: NodeId, sym: Symbol) -> Option<Exp> {
            self.rename(sym)
                .map(|new_sym| ExpData::LocalVar(id, new_sym).into_exp())
        }

        fn rewrite_pattern(&mut self, pat: &Pattern, creating_scope: bool) -> Option<Pattern> {
            // Assignment patterns reference existing variables and are
            // subject to the renaming; binding patterns introduce fresh
            // variables and are not.
            if creating_scope {
                return None;
            }
            if let Pattern::Var(id, sym) = pat {
                self.rename(*sym).map(|new_sym| Pattern::Var(*id, new_sym))
            } else {
                None
            }
        }
    }
    Renamer {
        renames,
        shadowed: vec![],
    }
    .rewrite_exp(exp.clone())
}

/// Returns the parameter symbol an expression refers to, either directly as
/// a local variable or as a temporary resolved against the formal parameters.
fn param_sym(exp: &ExpData, formal_params: &[Parameter]) -> Option<Symbol> {
    match exp {
        ExpData::LocalVar(_, sym) => Some(*sym),
        ExpData::Temporary(_, idx) if *idx < formal_params.len() => Some(formal_params[*idx].0),
        _ => None,
    }
}

/// Returns for each argument position which is a function parameter bound to
/// a lambda, the position, the parameter symbol, and the lambda.
fn collect_lambda_bindings(
    args: &[Exp],
    formal_params: &[Parameter],
    lambda_param_map: &BTreeMap<Symbol, &Exp>,
) -> Vec<(usize, Symbol, Exp)> {
    args.iter()
        .enumerate()
        .filter_map(|(pos, arg)| {
            let sym = param_sym(arg.as_ref(), formal_params)?;
            lambda_param_map
                .get(&sym)
                .map(|lambda| (pos, sym, (*lambda).clone()))
        })
        .collect()
}

impl ExpRewriterFunctions for InlinedRewriter<'_, '_> {
    /// Override default implementation to flag an error on an disallowed Return,
    /// as well as Break and Continue expressions outside of loops.
    fn rewrite_exp(&mut self, exp: Exp) -> Exp {
        // Inline behavioral predicates over lambda arguments, and redirect
        // spec function calls with lambda arguments to their specializations,
        // before descent: descending into the target would report a use of a
        // function-typed parameter as a value, and the lambda's spec material
        // spliced by the substitution is caller scope which must not be
        // rewritten.
        if let Some(repl) = self.try_inline_behavior_predicate(&exp) {
            return repl;
        }
        if let Some(repl) = self.try_specialize_spec_fun_call(&exp) {
            return repl;
        }
        // In specifications, function values cannot be applied; their behavior
        // is accessed via behavioral predicates. This is only enforced in
        // verify mode: in regular compilation, specs are not translated and
        // the application is silently beta-reduced as before. (Under
        // `LIFT_INLINE_FUNS`, applications are instead rewritten to derived
        // spec functions.)
        if self.in_spec > 0 && !self.rewrite_invoke_for_spec && self.env.is_verify_mode() {
            if let ExpData::Invoke(id, target, _) = exp.as_ref() {
                if self.resolve_lambda_target(target).is_some() {
                    self.env.error(
                        &self.env.get_node_loc(*id),
                        "a function value cannot be applied in a specification; \
                         use a behavioral predicate instead (e.g. `result_of<f>(..)` \
                         for the value of `f(..)`)",
                    );
                    return ExpData::Invalid(*id).into_exp();
                }
            }
        }
        // Rewrite in-body spec blocks via the standard spec descent. The kind
        // of the condition currently being rewritten is tracked through
        // `rewrite_enter_condition`, so behavioral predicate substitution
        // knows whether it targets a loop invariant; it is reset when the
        // spec block is done.
        if let ExpData::SpecBlock(..) = exp.as_ref() {
            let saved = self.current_condition_kind.take();
            self.in_spec += 1;
            let result = self.rewrite_exp_descent(exp);
            self.in_spec -= 1;
            self.current_condition_kind = saved;
            return result;
        }

        // Disallow Return and free LoopCont("continue" and "break") expressions in an inlined function.
        // Record if this is a Loop, as well as tracking loop nesting depth in self.in_loop.
        let this_is_loop = match exp.as_ref() {
            ExpData::Return(node_id, _) => {
                let node_loc = self.env.get_node_loc(*node_id);
                self.env.error(
                    &node_loc,
                    "Return not currently supported in inline functions",
                );
                false
            },
            ExpData::Loop(..) => {
                self.in_loop += 1;
                true
            },
            ExpData::LoopCont(node_id, _, is_continue) if self.in_loop == 0 => {
                let node_loc = self.env.get_node_loc(*node_id);
                self.env.error(
                    &node_loc,
                    &format!(
                        "{} outside of a loop not currently supported in inline functions",
                        if *is_continue { "Continue" } else { "Break" },
                    ),
                );
                false
            },
            _ => false,
        };

        if let ExpData::SpecBlock(_, _) = exp.as_ref() {
            self.in_spec += 1;
        } else if self.in_spec > 0 {
            self.in_spec += 1;
        }

        // Proceed with default behavior in any case.
        let result = self.rewrite_exp_descent(exp);

        if self.in_spec > 0 {
            self.in_spec -= 1;
        }

        // Exit loop if we matched it.
        if this_is_loop {
            self.in_loop -= 1;
        };

        result
    }

    /// Tracks the kind of the spec condition being rewritten, consumed by
    /// `try_inline_behavior_predicate` to detect loop invariants.
    fn rewrite_enter_condition(&mut self, _target: &SpecBlockTarget, cond: &Condition) {
        self.current_condition_kind = Some(cond.kind.clone());
        self.current_condition_has_folds_of = cond.kind == ConditionKind::LoopInvariant
            && cond.exp.any(&mut |exp| {
                matches!(
                    exp,
                    ExpData::Call(_, Operation::Behavior(BehaviorKind::FoldsOf, _), _)
                )
            });
    }

    fn rewrite_condition(
        &mut self,
        _target: &SpecBlockTarget,
        cond: &Condition,
    ) -> Option<Condition> {
        if !self.env.is_verify_mode() && matches!(cond.kind, ConditionKind::LoopInvariant) {
            return Some(Condition {
                exp: self.env.new_bool_const(&cond.loc, true),
                ..cond.clone()
            });
        }
        let mut result = cond.clone();
        let mut changed = false;
        if cond.kind == ConditionKind::LoopInvariant && self.current_condition_has_folds_of {
            result.properties.insert(
                self.env.symbol_pool().make(FOLDS_OF_INVARIANT_MARKER),
                move_model::ast::PropertyValue::Value(Value::Bool(true)),
            );
            changed = true;
        }
        let (exp, unresolved) = weaken_unresolved_conjuncts(self.env, &cond.exp);
        if unresolved {
            self.unresolved_behavior_in_spec |= cond.kind == ConditionKind::LoopInvariant;
            result.exp = exp;
            changed = true;
        }
        changed.then_some(result)
    }

    fn rewrite_spec(&mut self, _target: &SpecBlockTarget, spec: &Spec) -> Option<Spec> {
        let marker = self.env.symbol_pool().make(FOLDS_OF_INVARIANT_MARKER);
        let weaken_behavioral_invariants = self.unresolved_behavior_in_spec;
        self.unresolved_behavior_in_spec = false;
        let mut result = spec.clone();
        let mut changed = false;
        for cond in &mut result.conditions {
            let is_folds_of = cond.properties.remove(&marker).is_some();
            changed |= is_folds_of;
            if weaken_behavioral_invariants && is_folds_of {
                let loc = self.env.get_node_loc(cond.exp.node_id());
                cond.exp = self.env.new_bool_const(&loc, true);
            }
        }
        changed.then_some(result)
    }

    /// Record that the provided symbols have local definitions, so renaming should be done.
    /// Note that incoming vars are from a Pattern *after* renaming, so these are shadowed symbols.
    fn rewrite_enter_scope<'a>(
        &mut self,
        _id: NodeId,
        vars: impl Iterator<Item = &'a (NodeId, Symbol)>,
    ) {
        self.shadow_stack
            .enter_scope_after_renaming(vars.map(|(_, sym)| sym));
    }

    /// On exiting a scope defining some symbols shadowing lambda free vars, record that we have
    /// exited the scope so any occurrences of those free vars should be left alone (if there are
    /// not further shadowing scopes further out).
    fn rewrite_exit_scope(&mut self, _id: NodeId) {
        self.shadow_stack.exit_scope();
    }

    /// Instantiates `self.type_args` on a node in an inlined function
    /// Also updates the `Loc` for the node to indicate the inlined
    /// call site.
    fn rewrite_node_id(&mut self, id: NodeId) -> Option<NodeId> {
        let loc = self.env.get_node_loc(id);
        let new_loc = loc.inlined_from(self.call_site_loc);
        let result = ExpData::instantiate_node_new_loc(self.env, id, self.type_args, &new_loc);
        if let Some(new_id) = result
            && self
                .env
                .get_extension::<UnresolvedBehaviorNodes>()
                .is_some_and(|nodes| nodes.nodes.borrow().contains(&id))
        {
            mark_unresolved_behavior(self.env, new_id);
        }
        result
    }

    /// Replaces symbol uses that are shadowed with the shadow symbol.
    fn rewrite_local_var(&mut self, id: NodeId, sym: Symbol) -> Option<Exp> {
        self.shadow_stack
            .get_shadow_symbol(sym, false)
            .map(|new_sym| ExpData::LocalVar(id, new_sym).into())
    }

    /// Replaces symbol uses that are shadowed with the shadow symbol.
    fn rewrite_temporary(&mut self, id: NodeId, idx: TempIndex) -> Option<Exp> {
        let loc = self.env.get_node_loc(id);
        if idx < self.inlined_formal_params.len() {
            let param = &self.inlined_formal_params[idx];
            let sym = param.0;
            if self.lambda_param_map.contains_key(&sym) {
                // lambda parameter `sym` is used as a temp apart from a call
                // which is currently not supported
                let msg = format!("parameter `{}` with function type cannot be used as a local variable in an inline function",
                   sym.display(self.env.symbol_pool()));
                let call_details = vec![(loc.clone(), "being used here".to_string())];
                self.env
                    .diag_with_labels(Severity::Error, &param.2, &msg, call_details);
            }
            let param_type = &param.1;
            let instantiated_param_type = param_type.instantiate(self.type_args);
            let new_node_id = self.env.new_node(loc, instantiated_param_type);
            if let Some(new_sym) = self.shadow_stack.get_shadow_symbol(sym, false) {
                Some(ExpData::LocalVar(new_node_id, new_sym).into())
            } else {
                Some(ExpData::LocalVar(new_node_id, sym).into())
            }
        } else {
            self.env.diag(
                Severity::Bug,
                &loc,
                &format!(
                    "Temporary with invalid index `{}` during inlining \
                     of function with `{}` parameters",
                    idx,
                    self.inlined_formal_params.len()
                ),
            );
            None
        }
    }

    /// Handle calls to lambda parameters within the inlined function.  Lambda bodies are not
    /// rewritten at all, but ``InlinedRewriter::construct_inlined_call_expression` is used to
    /// convert the body, formal parameters, and actual arguments into a let expression which
    /// can be used in place of the call.
    fn rewrite_invoke(&mut self, id: NodeId, target: &Exp, args: &[Exp]) -> Option<Exp> {
        // Rewrite invoke to lambda expression into call to the corresponding spec function or move function
        // do it in the spec context
        if self.rewrite_invoke_for_spec {
            let rewrite_invoke_into_fun = |para_pos, call_spec_fun: bool| -> Option<Exp> {
                if let (Some((spec_fun_id, fn_id)), Some(closure)) = (
                    self.function_value_spec_map.get(para_pos),
                    self.function_value_map.get(para_pos),
                ) {
                    let spec_fun_decl: &SpecFunDecl = self.env.get_spec_fun(*spec_fun_id);
                    let fun_env = self.env.get_function(*fn_id);
                    assert!(fun_env.get_parameters().len() == spec_fun_decl.params.len());
                    if let ExpData::Call(_, Operation::Closure(_, _, mask), captured) =
                        closure.as_ref()
                    {
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
                        Some(
                            if !call_spec_fun {
                                ExpData::Call(
                                    id,
                                    Operation::MoveFunction(fn_id.module_id, fn_id.id),
                                    new_args.clone(),
                                )
                            } else {
                                ExpData::Call(
                                    id,
                                    Operation::SpecFunction(
                                        spec_fun_id.module_id,
                                        spec_fun_id.id,
                                        MemoryRange::default(),
                                    ),
                                    new_args.clone(),
                                )
                            }
                            .into_exp(),
                        )
                    } else {
                        None
                    }
                } else {
                    None
                }
            };
            if let ExpData::LocalVar(_, sym) = target.as_ref() {
                if let Some(para_pos) = self.sym_param_map.get(sym) {
                    return rewrite_invoke_into_fun(para_pos, self.in_spec > 0);
                }
            } else if let ExpData::Temporary(_, para_pos) = target.as_ref() {
                return rewrite_invoke_into_fun(para_pos, self.in_spec > 0);
            }
            return None;
        }
        let optional_lambda_target: Option<&Exp> = match target.as_ref() {
            ExpData::LocalVar(_, symbol) => self.lambda_param_map.get(symbol).copied(),
            ExpData::Temporary(_, idx) => {
                if *idx < self.inlined_formal_params.len() {
                    let param = &self.inlined_formal_params[*idx];
                    let sym = param.0;
                    self.lambda_param_map.get(&sym).copied()
                } else {
                    None
                }
            },
            // FUTURE TODO: uncomment this for more functionality
            // ExpData::Lambda(..) => Some(Target),
            _ => None,
        };
        let call_loc = self.env.get_node_loc(id);
        // The anchor label of this application, when it is the unique
        // application of the parameter.
        let anchor = param_sym(target.as_ref(), &self.inlined_formal_params)
            .and_then(|sym| self.application_anchors.get(&sym).copied());
        if let Some(lambda_target) = optional_lambda_target {
            if let ExpData::Lambda(_, pat, body, _, _) = lambda_target.as_ref() {
                let args_vec: Vec<Exp> = args.to_vec();
                let body = if let Some(label) = anchor {
                    // Bind the anchor state between the parameter binding and
                    // the lambda body, so substituted two-state conditions
                    // refer to the state in which the lambda starts executing
                    // — after the invocation's arguments, which may
                    // themselves have global state effects, are evaluated.
                    let marker_exp = ExpData::Call(
                        self.env.new_bool_node(&call_loc),
                        Operation::SaveStateAnchor(label),
                        vec![],
                    )
                    .into_exp();
                    let marker = ExpData::SpecBlock(
                        self.env.new_node(call_loc.clone(), Type::unit()),
                        Spec {
                            conditions: vec![Condition {
                                loc: call_loc.clone(),
                                kind: ConditionKind::Assume,
                                properties: Default::default(),
                                exp: marker_exp,
                                additional_exps: vec![],
                            }],
                            ..Spec::default()
                        },
                    )
                    .into_exp();
                    let body_ty = self.env.get_node_type(body.as_ref().node_id());
                    ExpData::Sequence(self.env.new_node(call_loc.clone(), body_ty), vec![
                        marker,
                        body.clone(),
                    ])
                    .into_exp()
                } else {
                    body.clone()
                };
                Some(InlinedRewriter::construct_inlined_call_expression(
                    self.env,
                    &call_loc,
                    body,
                    make_lambda_pattern_a_tuple(self.env, pat),
                    args_vec,
                ))
            } else {
                self.env.diag(
                    Severity::Bug,
                    &call_loc,
                    "Invalid call target: problem dereferencing target expression",
                );
                None
            }
        } else {
            // This is an error, but it is flagged elsewhere.
            None
        }
    }

    fn rewrite_pattern(&mut self, pat: &Pattern, entering_scope: bool) -> Option<Pattern> {
        // Rewrite type instantiation in pattern node id
        let old_id = pat.node_id();
        let new_id_opt = ExpData::instantiate_node(self.env, old_id, self.type_args);
        let new_id = new_id_opt.unwrap_or(old_id);
        match pat {
            Pattern::Var(_, sym) => self
                .shadow_stack
                .get_shadow_symbol(*sym, entering_scope)
                .map(|new_sym| Pattern::Var(new_id, new_sym))
                .or_else(|| new_id_opt.map(|id| Pattern::Var(id, *sym))),
            Pattern::Tuple(_, pattern_vec) => Some(Pattern::Tuple(new_id, pattern_vec.clone())),
            Pattern::Struct(_, struct_id, variant, pattern_vec) => {
                let new_struct_id = struct_id.clone().instantiate(self.type_args);
                Some(Pattern::Struct(
                    new_id,
                    new_struct_id,
                    *variant,
                    pattern_vec.clone(),
                ))
            },
            Pattern::Wildcard(_) => Some(Pattern::Wildcard(new_id)),
            Pattern::LiteralValue(_, val) => Some(Pattern::LiteralValue(new_id, val.clone())),
            Pattern::Range(_, lo, hi, inc) => {
                Some(Pattern::Range(new_id, lo.clone(), hi.clone(), *inc))
            },
            Pattern::Error(_) => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use move_model::model::Loc;

    /// Helpers for building the derived-spec shapes `derive_forwarded_application`
    /// decomposes.
    fn fwd_test_env() -> GlobalEnv {
        GlobalEnv::new()
    }

    fn bool_exp(env: &GlobalEnv, value: bool) -> Exp {
        env.new_bool_const(&Loc::default(), value)
    }

    fn temp(env: &GlobalEnv, idx: usize) -> Exp {
        let fun_ty = Type::Fun(
            Box::new(Type::new_prim(PrimitiveType::U64)),
            Box::new(Type::unit()),
            AbilitySet::EMPTY,
        );
        ExpData::Temporary(env.new_node(Loc::default(), fun_ty), idx).into_exp()
    }

    fn num(env: &GlobalEnv, value: u64) -> Exp {
        ExpData::Value(
            env.new_node(Loc::default(), Type::new_prim(PrimitiveType::U64)),
            Value::Number(BigInt::from(value)),
        )
        .into_exp()
    }

    fn aborts_of_bp(env: &GlobalEnv, target: &Exp, args: Vec<Exp>) -> Exp {
        let mut bp_args = vec![target.clone()];
        bp_args.extend(args);
        ExpData::Call(
            env.new_bool_node(&Loc::default()),
            Operation::Behavior(BehaviorKind::AbortsOf, MemoryRange::default()),
            bp_args,
        )
        .into_exp()
    }

    fn derived_with(
        target: &Exp,
        args: Vec<Exp>,
        guard: Option<Exp>,
        aborts: Vec<Exp>,
        modifies: Option<Vec<Exp>>,
    ) -> DerivedSpec {
        DerivedSpec {
            aborts,
            modifies,
            deferred_applications: vec![(target.clone(), args, guard)],
            ..DerivedSpec::default()
        }
    }

    /// The forwarder decomposition (D6): a single unconditional deferred
    /// application with an empty exact footprint decomposes; the
    /// application's own `aborts_of` disjunct is covered by the deferral,
    /// other disjuncts are prelude material.
    #[test]
    fn forwarded_application_decomposes() {
        let env = fwd_test_env();
        let target = temp(&env, 1);
        let prelude_abort = bool_exp(&env, false);
        let derived = derived_with(
            &target,
            vec![num(&env, 7)],
            None,
            vec![
                prelude_abort.clone(),
                aborts_of_bp(&env, &target, vec![num(&env, 7)]),
            ],
            Some(vec![]),
        );
        let fwd = derive_forwarded_application(&derived).expect("decomposes");
        assert!(same_param_ref(&fwd.target, &target));
        assert_eq!(fwd.args.len(), 1);
        assert_eq!(fwd.prelude_aborts.len(), 1);
        assert!(fwd.prelude_aborts[0].as_ref() == prelude_abort.as_ref());
    }

    /// A conditional application, a non-empty footprint, or an abort
    /// disjunct mixing prelude material with a behavioral predicate over
    /// the parameter reject the decomposition.
    #[test]
    fn forwarded_application_rejections() {
        let env = fwd_test_env();
        let target = temp(&env, 1);
        // Conditional application.
        let derived = derived_with(
            &target,
            vec![],
            Some(bool_exp(&env, true)),
            vec![],
            Some(vec![]),
        );
        assert!(derive_forwarded_application(&derived).is_none());
        // Unknown footprint.
        let derived = derived_with(&target, vec![], None, vec![], None);
        assert!(derive_forwarded_application(&derived).is_none());
        // A disjunct containing (but not being) a predicate over the
        // parameter cannot be attributed by the partition.
        let mixed = ExpData::Call(env.new_bool_node(&Loc::default()), Operation::And, vec![
            bool_exp(&env, true),
            aborts_of_bp(&env, &target, vec![]),
        ])
        .into_exp();
        let derived = derived_with(&target, vec![], None, vec![mixed], Some(vec![]));
        assert!(derive_forwarded_application(&derived).is_none());
        // Two applications.
        let mut derived = derived_with(&target, vec![], None, vec![], Some(vec![]));
        derived
            .deferred_applications
            .push((temp(&env, 2), vec![], None));
        assert!(derive_forwarded_application(&derived).is_none());
    }

    #[test]
    fn test_cycle() {
        let graph = BTreeMap::from([
            (1, BTreeSet::from([2])),
            (2, BTreeSet::from([3])),
            (3, BTreeSet::from([4])),
            (4, BTreeSet::from([5, 6])),
            (5, BTreeSet::from([3])),
            (6, BTreeSet::new()),
        ]);
        let cycle = vec![3, 4, 5];
        assert!(check_for_cycles(&graph) == BTreeSet::from([cycle]));
    }

    #[test]
    fn test_no_cycle() {
        let graph = BTreeMap::from([
            (1, BTreeSet::from([2, 3])),
            (2, BTreeSet::from([4])),
            (3, BTreeSet::from([4])),
            (4, BTreeSet::from([5, 6])),
            (5, BTreeSet::from([7])),
            (6, BTreeSet::from([7])),
            (7, BTreeSet::new()),
        ]);
        assert!(check_for_cycles(&graph) == BTreeSet::new());
    }

    #[test]
    fn test_postorder() {
        let entries = vec![1, 2, 3, 4, 5, 7];
        let call_graph = BTreeMap::from([
            (1, BTreeSet::from([2, 3])),
            (2, BTreeSet::from([4])),
            (3, BTreeSet::from([4])),
            (4, BTreeSet::from([5, 6])),
            (5, BTreeSet::from([7])),
            (6, BTreeSet::new()),
            (7, BTreeSet::from([8])),
            (9, BTreeSet::new()),
        ]);
        let result = postorder(&entries, &call_graph);
        assert!(
            result == vec![8, 7, 5, 6, 4, 3, 2, 1]
                || result == vec![8, 7, 6, 5, 4, 3, 2, 1]
                || result == vec![8, 6, 7, 5, 4, 3, 2, 1]
                || result == vec![6, 8, 7, 5, 4, 3, 2, 1]
        );
    }
}
