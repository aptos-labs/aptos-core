// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

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
use crate::env_pipeline::{
    rewrite_target::{RewriteState, RewriteTarget, RewriteTargets, RewritingScope},
    spec_rewriter::run_spec_rewriter_inline,
};
use codespan_reporting::diagnostic::Severity;
use itertools::Itertools;
use log::trace;
use move_model::{
    ast::{Exp, ExpData, Operation, Pattern, Spec, SpecBlockTarget, SpecFunDecl, TempIndex},
    exp_rewriter::ExpRewriterFunctions,
    metadata::LanguageVersion,
    model::{FunId, GlobalEnv, Loc, NodeId, Parameter, QualifiedId, SpecFunId},
    symbol::Symbol,
    ty::{ReferenceKind, Type},
};
use std::{
    collections::{BTreeMap, BTreeSet},
    fmt::Debug,
    iter,
    iter::{zip, IntoIterator, Iterator},
    vec::Vec,
};

type QualifiedFunId = QualifiedId<FunId>;
type CallSiteLocations = BTreeMap<(RewriteTarget, QualifiedFunId), BTreeSet<NodeId>>;

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
    // Get non-inline function roots for running inlining.
    // Also generate an error for any target inline functions lacking a body to inline.
    let mut targets = RewriteTargets::create(env, scope);
    filter_targets(env, &mut targets);
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
        if let Ok(targets_needing_inlining) =
            targets_needing_inlining_in_order(env, &call_graph, inline_function_call_site_locations)
        {
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
                if func.is_inline() && func.get_def().is_some() {
                    // Only delete functions with a body.
                    inline_funs.insert(id);
                }
            }
        }
        env.filter_functions(|fun_id: &QualifiedFunId| !inline_funs.contains(fun_id));
    }
}

/// Filter out inline functions from targets since we only process them when they are
/// called from other functions. While we're iterating, produce an error
/// on every inline function lacking a body to inline.
fn filter_targets(env: &GlobalEnv, targets: &mut RewriteTargets) {
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
                false
            } else {
                true
            }
        } else {
            true
        }
    });
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
        .map(|(node, set)| (*node, iter::repeat(vec![*node]).take(set.len()).collect()))
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

impl<'env, 'inliner> OuterInlinerRewriter<'env, 'inliner> {
    fn new(inliner: &'inliner mut Inliner<'env>, current_target: Option<QualifiedFunId>) -> Self {
        Self {
            inliner,
            current_fun_target_opt: current_target,
        }
    }
}

impl<'env, 'inliner> ExpRewriterFunctions for OuterInlinerRewriter<'env, 'inliner> {
    /// recognize call to inline function and rewrite it using `InlinedRewriter::inline_call`
    fn rewrite_call(&mut self, call_id: NodeId, oper: &Operation, args: &[Exp]) -> Option<Exp> {
        if let Operation::MoveFunction(module_id, fun_id) = oper {
            let qfid = module_id.qualified(*fun_id);
            let func_env = self.inliner.env.get_function(qfid);
            if func_env.is_inline() {
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
                    let rewritten = InlinedRewriter::inline_call(
                        self.inliner.env,
                        call_id,
                        &func_loc,
                        &expr,
                        type_args,
                        parameters,
                        args,
                        self.inliner.lift_inline_funs,
                        self.current_fun_target_opt,
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
    /// Return all lambda parameters that are used as temps
    lambda_used_as_temp: BTreeSet<Symbol>,
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

        if lift_inline_funs && target_qualified_fun_id_opt.is_some() {
            let mut lifted_lambda_funs: BTreeMap<usize, move_model::model::FunctionData> =
                BTreeMap::new();
            let options = LambdaLiftingOptions {
                include_inline_functions: true,
            };
            let fun_env = env.get_function(target_qualified_fun_id_opt.unwrap());
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
                // Only one lift function should be generated.
                assert_eq!(lifter.lifted_len(), 1);
                let func_data = lifter.get_lifted_at(0).unwrap().generate_function_data(env);
                sym_param_map.insert(para.1 .0, para.0);
                function_value_map.insert(para.0, closure_exp.clone());
                lifted_lambda_funs.insert(para.0, func_data);
            }
            function_value_spec_map = run_spec_rewriter_inline(
                env,
                target_qualified_fun_id_opt.unwrap().module_id,
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
            lambda_used_as_temp: BTreeSet::new(),
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
    ) -> Exp {
        let body = body.clone();
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
        let mut regular_params = regular_params
            .into_iter()
            .map(|(_, para)| para)
            .collect_vec();

        // Find free variables in lambda expr.  Perhaps we could minimize changes if we tracked each
        // lambda arg individually in the inlined method and only rewrite the context of each
        // inlined lambda, but that seems quite difficult.  Instead, just group all the free vars
        // together and shadow them all.
        let all_lambda_free_vars: BTreeSet<_> = lambda_args_matched
            .iter()
            .flat_map(|(_, exp)| exp.free_vars().into_iter())
            .collect();

        // While we're looking at the lambdas, check for Return in their bodies.
        for (_, lambda_body) in lambda_args_matched.iter() {
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
        );

        // For now, just copy the actuals.  If FreezeRef is needed, we'll do it in
        // construct_inlined_call_expression.
        let mut rewritten_actuals: Vec<Exp> = regular_actuals.into_iter().cloned().collect();

        // Enter the scope defined by the params.
        rewriter.shadowing_enter_scope(regular_params_overlapping_free_vars);

        // Rewrite body types, shadowed vars, replace invoked lambda params, etc.
        let rewritten_body = rewriter.rewrite_exp(body.clone());

        // For each lambda parameter, if it is also used as a temp in the body of inline function,
        // add it to the list of regular parameters and actuals.
        for ((_, param), exp) in lambda_args_matched {
            if rewriter.lambda_used_as_temp.contains(&param.0) {
                regular_params.push(param);
                rewritten_actuals.push(exp.clone());
            }
        }
        // Turn list of parameters into a pattern.  Also rewrite types as needed.
        // Shadow param vars as if we are in a let.
        let params_pattern =
            rewriter.parameter_list_to_pattern(env, func_loc, &call_site_loc, regular_params);

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
                if env.language_version().is_at_least(LanguageVersion::V2_1)
                    && env.symbol_pool().string(*sym).as_ref() == "_"
                {
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

    /// Convert any non-`Tuple` pattern `pat` into a a singleton `Pattern::Tuple` if needed,
    /// for convenience in matching it to a `Tuple` of expressions.
    fn make_lambda_pattern_a_tuple(&mut self, pat: &Pattern) -> Pattern {
        if !matches!(pat, Pattern::Tuple(..)) {
            let id = pat.node_id();
            let new_id = self.env.new_node(
                self.env.get_node_loc(id),
                Type::Tuple(vec![self.env.get_node_type(id)]),
            );
            Pattern::Tuple(new_id, vec![pat.clone()])
        } else {
            pat.clone()
        }
    }
}

impl<'env, 'rewriter> ExpRewriterFunctions for InlinedRewriter<'env, 'rewriter> {
    /// Override default implementation to flag an error on an disallowed Return,
    /// as well as Break and Continue expressions outside of loops.
    fn rewrite_exp(&mut self, exp: Exp) -> Exp {
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
        ExpData::instantiate_node_new_loc(self.env, id, self.type_args, &new_loc)
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
                // lambda parameter `sym` is used as a tempapart from a call
                self.lambda_used_as_temp.insert(sym);
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
                                        None,
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
        if let Some(lambda_target) = optional_lambda_target {
            if let ExpData::Lambda(_, pat, body, _, _) = lambda_target.as_ref() {
                let args_vec: Vec<Exp> = args.to_vec();
                Some(InlinedRewriter::construct_inlined_call_expression(
                    self.env,
                    &call_loc,
                    body.clone(),
                    self.make_lambda_pattern_a_tuple(pat),
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
            Pattern::Wildcard(_) => None,
            Pattern::Error(_) => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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
