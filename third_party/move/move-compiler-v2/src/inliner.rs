// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

// Overview of approach:
// - We visit function calling inline functions reachable from compilation targets in a bottom-up
//   fashion, storing rewritten functions in a map to simplify further processing.
//   - Change to the program happens at the end.
//
// - struct `Inliner`
//   - holds the map recording function bodies which are rewritten due to inlining so that we don't
//     need to modify the program until the end.
//   - `try_inlining_in` function is the entry point for each function needing inlining.
//
// - struct `OuterInlinerRewriter` uses trait `ExpRewriterFunctions` to rewrite each call in the
//   target.
//   - `rewrite_call` recognizes inline functions and rewrites them using
//     `InlinedRewriter::inline_call`
//
// - struct `InlinedRewriter` uses trait `ExpRewriterFunctions` to rewrite the inlined function
//   body.
//   - `rewrite_exp` disallows Return expressions
//
//   - `rewrite_enter_scope` and `rewrite_exit_scope` record lambda free vars which are shadowed by
//      a local scope, so that uses can be renamed to shadow vars appropriately.
//
//   - `rewrite_node_id` instantiates type_args on every node in the inlined function
//
//   - `rewrite_local_var` replaces symbol uses that are shadowed with the shadow symbol.
//   - `rewrite_temporary` replaces references to function parameters with parameter symbols, also
//      shadowing as needed.
//
//   - `rewrite_invoke` handles calls to lambda parameters within the inlined function
//      - Note that lambda bodies are not rewritten during inlining, but are kept intact
//
//   - `rewrite_pattern` replaces syms in a pattern with shadow symbols as necessary
//
// - struct `InlinedRewriter` also has various methods to support rewriting a call
//   - `inline_call` is the entry point for rewriting a call to an inline function.
//
//   - `create_shadow_symbols` creates a shadow `Symbol` for each lambda free variable to use if a
//     local variable conflicts with that variable.  It uses `SymbolPool::shadow` to create shadow
//     symbols which print out the same but don't conflict.
//
//   - `get_shadow_symbol` checks whether a `Symbol` conflicts with lambda free variables, and if
//     so, returns the shadow symbol to use instead..
//
//   - `parameter_list_to_pattern` is a helper to convert a list of `Parameter` into a `Pattern`,
//      suitable for use in the body.
//
//   - `construct_inlined_call_expression` is a helper to build the expression corresponding to
//      { let params=actuals; body } used for both lambda inlining and inline function inlining.
//
//   - rewrite_pattern_vector is a helper for `rewrite_pattern`
//

// TODO:
// - add a InlinedCall() node that represents an inlined call, so that an inlined Return() has a
//   place to go. [later]
//   - OR: if an inlined fn has a return, then error out.
// - add a simplifier that simplifies certain code constructs, e.g.
//   - InlinedCall() with no nested Return() can be flattened.
//   - others?
// - do we need to insert FreezeRef in actual args if formal param is &T but arg is &mod T ?
// - do we need to do anything about abilities?
// - if lambda parameters also may be referred to by temporaries, then `rewrite_invoke` might need
//   to call yet another `ExpRewriterFunctions` implementation to do that.

use crate::options::Options;
use codespan_reporting::diagnostic::Severity;
use itertools::chain;
use move_model::{
    ast::{Exp, ExpData, Operation, Pattern, TempIndex},
    exp_rewriter::ExpRewriterFunctions,
    model::{FunId, GlobalEnv, Loc, NodeId, Parameter, QualifiedId, TypeParameter},
    symbol::Symbol,
    ty::{ReferenceKind, Type},
};
use std::{
    collections::{BTreeMap, BTreeSet},
    fmt::Debug,
    iter::{zip, IntoIterator, Iterator},
    ops::Deref,
    vec::Vec,
};

type QualifiedFunId = QualifiedId<FunId>;
type CallSiteLocations = BTreeMap<(QualifiedFunId, QualifiedFunId), BTreeSet<NodeId>>;

// ======================================================================================
// Entry

// Run inlining on current program's AST.  For each function which is target of the compilation,
// visit that function body and inline any calls to functions marked as "inline".
pub fn run_inlining(env: &mut GlobalEnv) {
    // Get non-inline function roots for running inlining.
    // While we're iterating, generate an error for any target inline functions lacking a body to
    // inline.
    let mut todo = get_targets(env);

    if !todo.is_empty() {
        // Recursively find callees of each target with a function body.
        let mut function_callees: BTreeMap<_, BTreeSet<_>> = BTreeMap::new();

        // Record for each pair of (caller, callee) functions, all the call site locations (for error
        // messages).
        let mut inline_function_call_site_locations: CallSiteLocations = CallSiteLocations::new();

        // Update function_callees and inline_function_call_site_locations for all reachable calls.
        let mut visited_functions = BTreeSet::new();
        while let Some(id) = todo.pop_first() {
            if visited_functions.insert(id) {
                if let Some(def) = env.get_function(id).get_def().deref() {
                    let callees_with_sites = def.called_funs_with_callsites();
                    for (callee, sites) in callees_with_sites {
                        todo.insert(callee);
                        function_callees.entry(id).or_default().insert(callee);
                        if env.get_function(callee).is_inline() {
                            inline_function_call_site_locations.insert((id, callee), sites);
                        }
                    }
                }
            }
        }

        // Get a list of all reachable functions calling inline functions, in bottom-up order.
        if let Ok(functions_needing_inlining) = functions_needing_inlining_in_order(
            env,
            &function_callees,
            inline_function_call_site_locations,
        ) {
            // We inline functions bottom-up, so that any inline function which itself has calls to
            // inline functions has already had its stuff inlined.
            let mut inliner = Inliner::new(env);
            for fid in functions_needing_inlining.iter() {
                inliner.try_inlining_in(*fid);
            }

            // Now that all inlining finished, actually update function bodies in env.
            for (fun_id, funexpr_after_inlining) in inliner.funexprs_after_inlining {
                if let Some(changed_funexpr) = funexpr_after_inlining {
                    let oldexp = env.get_function(fun_id);
                    // Only record changed cuntion bodies for functions which are targets.
                    if oldexp.module_env.is_target() {
                        let mut old_def = oldexp.get_mut_def();
                        *old_def = Some(changed_funexpr);
                    }
                }
            }
        }
    }

    // Delete all inline functions with bodies from the program rep, even if none were inlined,
    // since (1) they are no longer needed, and (2) they may have code constructs that codegen can't
    // deal with.
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
    env.filter_functions(|fun_id: &QualifiedFunId| inline_funs.contains(fun_id));
}

/// Helper functions for inlining driver

// Get all target functions which are not themselves inline functions.
// While we're iterating, produce an error on every target inline function lacking a body to
// inline.
fn get_targets(env: &mut GlobalEnv) -> BTreeSet<QualifiedFunId> {
    let mut targets = BTreeSet::new();
    for module in env.get_modules() {
        if module.is_target() {
            for func in module.get_functions() {
                let id = func.get_qualified_id();
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
                    } else {
                        eprintln!("Found inline function {:#?}", func.get_name_str());
                    }
                } else {
                    targets.insert(id);
                }
            }
        }
    }
    targets
}

/// Return a list of all functions calling inline functions, in bottom-up order,
/// so that any inline function will be processed before any function calling it.
fn functions_needing_inlining_in_order(
    env: &GlobalEnv,
    function_callees: &BTreeMap<QualifiedFunId, BTreeSet<QualifiedFunId>>,
    inline_function_call_site_locations: CallSiteLocations,
) -> Result<Vec<QualifiedFunId>, ()> {
    // Restrict attention to inline functions calling inline functions.
    let inline_functions_with_callees: BTreeMap<QualifiedFunId, BTreeSet<QualifiedFunId>> =
        function_callees
            .iter()
            .filter(|&(fnid, _)| env.get_function(*fnid).is_inline())
            .map(|(fnid, callees)| {
                (
                    *fnid,
                    callees
                        .iter()
                        .filter(|&caller_fnid| env.get_function(*caller_fnid).is_inline())
                        .cloned()
                        .collect(),
                )
            })
            .collect();

    // Calculate the list of inline functions which call at least one other inline function.
    let inline_functions_calling_others: Vec<QualifiedFunId> = inline_functions_with_callees
        .iter()
        .filter(|(_, callees)| !callees.is_empty())
        .map(|(caller_fnid, _)| caller_fnid)
        .cloned()
        .collect();

    // Check for cycles
    let cycles = check_for_cycles(&inline_functions_with_callees);
    if !cycles.is_empty() {
        for cycle in cycles {
            let start_fnid = cycle.first().unwrap();
            let func_env = env.get_function(*start_fnid);
            let path_string: String = cycle
                .iter()
                .map(|fnid| env.get_function(*fnid).get_name_str())
                .collect::<Vec<String>>()
                .join("` -> `");
            let mut call_details: Vec<_> = cycle
                .iter()
                .zip(cycle.iter().skip(1).chain([*start_fnid].iter()))
                .flat_map(|(f, g)| {
                    let sites_ids = inline_function_call_site_locations.get(&(*f, *g)).unwrap();
                    let f_str = env.get_function(*f).get_name_str();
                    let g_str = env.get_function(*g).get_name_str();
                    let msg = format!("call from `{}` to `{}`", f_str, g_str);
                    sites_ids
                        .iter()
                        .map(move |node_id| (env.get_node_loc(*node_id), msg.clone()))
                })
                .collect();
            let msg = format!(
                "recursion during function inlining not allowed: `{}` -> `{}`",
                path_string,
                func_env.get_name_str()
            );
            let loc = call_details.first_mut().unwrap().0.clone();
            env.diag_with_labels(Severity::Error, &loc, &msg, call_details);
        }
        return Err(());
    }

    // Compute post-order of inline_functions which call others.
    let po_inline_functions = postorder(
        &inline_functions_calling_others,
        &inline_functions_with_callees,
    );

    // Identify subset of non-inline functions which call inline functions.  Order doesn't matter here.
    let non_inline_functions_needing_inlining: Vec<QualifiedFunId> = function_callees
        .iter()
        .filter(|(fnid, callees)| {
            !env.get_function(**fnid).is_inline()
                && callees
                    .iter()
                    .any(|fnid2| env.get_function(*fnid2).is_inline())
        })
        .map(|(fnid, _callees)| fnid)
        .cloned()
        .collect();

    let result: Vec<QualifiedFunId> =
        chain(po_inline_functions, non_inline_functions_needing_inlining).collect();
    Ok(result)
}

// Calculate a bottom-up traversal for entries, given the provided callee map.
fn postorder<T: Ord + Copy + Debug>(
    entries: &Vec<T>,
    callee_map: &BTreeMap<T, BTreeSet<T>>,
) -> Vec<T> {
    let mut stack = Vec::new();
    let mut visited = BTreeSet::new();
    let mut grey = BTreeSet::new();
    let mut postorder_num_to_node = Vec::new();
    let mut node_to_postorder_num = BTreeMap::new();

    for entry in entries {
        if !visited.contains(&entry) {
            visited.insert(entry);
            stack.push(entry);
            while let Some(curr) = stack.pop() {
                if grey.contains(&curr) {
                    let curr_num = postorder_num_to_node.len();
                    postorder_num_to_node.push(*curr);
                    node_to_postorder_num.insert(curr, curr_num);
                } else {
                    grey.insert(curr);
                    stack.push(curr);
                    if let Some(children) = callee_map.get(curr) {
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

// Check for cycles in a callee_map.
// If there is a cycle, return one cyclical path.
fn check_for_cycles<T: Ord + Copy + Debug>(
    callee_map: &BTreeMap<T, BTreeSet<T>>,
) -> BTreeSet<Vec<T>> {
    let mut cycles: BTreeSet<Vec<T>> = BTreeSet::new();
    let mut reachable_from_map: BTreeMap<T, BTreeSet<Vec<T>>> = callee_map
        .iter()
        .map(|(node, set)| (*node, set.iter().map(|_node2| [*node].to_vec()).collect()))
        .collect();

    let mut changed = true;
    let mut new_paths: BTreeSet<Vec<T>> = BTreeSet::new();
    while changed {
        changed = false;
        for (start_node, path_set) in reachable_from_map.iter_mut() {
            for path in path_set.iter() {
                let path_last = path.last().unwrap();
                if let Some(succ_set) = callee_map.get(path_last) {
                    if succ_set.contains(start_node) {
                        // found a cycle, return it.
                        // TODO: maybe find all cycles?
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
    env: &'env GlobalEnv,
    debug: bool,
    /// Functions already processed all get an entry here, with a new function body after inline
    /// calls are substituted here.  Functions which are unchanged (no calls to inline functions)
    /// bind to None.
    funexprs_after_inlining: BTreeMap<QualifiedFunId, Option<Exp>>,
}

impl<'env> Inliner<'env> {
    fn new(env: &'env GlobalEnv) -> Self {
        let funexprs_after_inlining = BTreeMap::new();
        let debug = env
            .get_extension::<Options>()
            .expect("Options is available")
            .debug;
        Self {
            env,
            debug,
            funexprs_after_inlining,
        }
    }

    /// If `self.funxprs_after_inlining` doesn't already have an entry for provided `func_id`, then
    /// scan the function body for inlining opportunities.  Add an entry to
    /// `self.funexprs_after_inlining`, mapping to `None` if there are no inlining opportunities,
    /// or to the function body after inlining.  Return the set of non-inline functions called from
    /// the resulting function.
    ///
    /// If `self.funxprs_after_inlining` already has an entry, then returns the empty set.
    fn try_inlining_in(&mut self, func_id: QualifiedFunId) {
        assert!(!self.funexprs_after_inlining.contains_key(&func_id));
        let func_env = self.env.get_function(func_id);

        if self.debug {
            eprintln!(
                "try_inlining_in `{}`:\n{}",
                func_env.get_full_name_str(),
                self.env.dump_fun(&func_env)
            );
        }
        let optional_def_ref = func_env.get_def();
        if let Some(def) = &*optional_def_ref {
            let mut rewriter = OuterInlinerRewriter::new(self.env, self);

            let rewritten = rewriter.rewrite_exp(def.clone());
            let changed = !ExpData::ptr_eq(&rewritten, def);
            if changed {
                self.funexprs_after_inlining
                    .insert(func_id, Some(rewritten));
            } else {
                self.funexprs_after_inlining.insert(func_id, None);
            }
        } else {
            // Ignore missing body.  Error is flagged elsewhere.
        }
    }
}

/// Rewriter for processing functions which may have inline function calls within them.
/// The only thing it rewrites are calls to inline functions; we use the ExpRewriterFunctions
/// trait to find such calls and reconstruct the outer function to include them after rewriting.
struct OuterInlinerRewriter<'env, 'inliner> {
    env: &'env GlobalEnv,
    /// Functions already processed all get an entry here, with a new function body after inline
    /// calls are substituted here.
    inliner: &'inliner mut Inliner<'env>,
}

impl<'env, 'inliner> OuterInlinerRewriter<'env, 'inliner> {
    fn new(env: &'env GlobalEnv, inliner: &'inliner mut Inliner<'env>) -> Self {
        Self { env, inliner }
    }
}

impl<'env, 'inliner> ExpRewriterFunctions for OuterInlinerRewriter<'env, 'inliner> {
    fn rewrite_call(&mut self, call_id: NodeId, oper: &Operation, args: &[Exp]) -> Option<Exp> {
        if let Operation::MoveFunction(module_id, fun_id) = oper {
            let fid = module_id.qualified(*fun_id);
            let func_env = self.env.get_function(fid);
            if func_env.is_inline() {
                // inline the function call
                let type_parameters = func_env.get_type_parameters();
                let type_args = self.env.get_node_instantiation(call_id);
                let parameters = func_env.get_parameters();
                let result_type = func_env.get_result_type();
                let func_loc = func_env.get_loc();
                if let Some(Some(expr)) = self.inliner.funexprs_after_inlining.get(&fid) {
                    // inline here
                    if self.inliner.debug {
                        eprintln!(
                            "inlining (inlined) function `{}` with args `{}`",
                            self.env.dump_fun(&func_env),
                            args.iter()
                                .map(|exp| format!("{}", exp.as_ref().display(self.env)))
                                .collect::<Vec<_>>()
                                .join(","),
                        );
                    }
                    let rewritten = InlinedRewriter::inline_call(
                        self.env,
                        call_id,
                        &func_loc,
                        expr,
                        type_parameters,
                        type_args,
                        parameters,
                        args,
                        result_type,
                        self.inliner.debug,
                    );
                    if self.inliner.debug {
                        eprintln!(
                            "After (inlined) inlining, expr is `{}`",
                            rewritten.display(self.env)
                        );
                    }
                    Some(rewritten)
                } else {
                    let func_env_def = func_env.get_def();
                    let func_env_def_deref = func_env_def.deref();
                    if let Some(expr) = &func_env_def_deref {
                        // inline here
                        if self.inliner.debug {
                            eprintln!(
                                "inlining function `{}` with args `{}`",
                                self.env.dump_fun(&func_env),
                                args.iter()
                                    .map(|exp| format!("{}", exp.as_ref().display(self.env)))
                                    .collect::<Vec<_>>()
                                    .join(","),
                            );
                        }
                        let rewritten = InlinedRewriter::inline_call(
                            self.env,
                            call_id,
                            &func_loc,
                            expr,
                            type_parameters,
                            type_args,
                            parameters,
                            args,
                            result_type,
                            self.inliner.debug,
                        );
                        if self.inliner.debug {
                            eprintln!(
                                "After inlining, expr is `{}`",
                                rewritten.as_ref().display(self.env)
                            );
                        }
                        Some(rewritten)
                    } else {
                        // Ignore missing body.  Error is flagged elsewhere.
                        None
                    }
                }
            } else {
                None
            }
        } else {
            None
        }
    }
}

/// For a given set of "free" variables, the ShadowStack tracks which variables are
/// still directly visible, and which variables have been hidden by local variable
/// declarations with the same symbol.  In the latter case, the ShadowStack provides
/// a "shadow" symbol which can be used in place of the original.
///
/// Operations are
///       pub fn new<'a, T>(env: &GlobalEnv, free_vars: T) -> Self
///       pub fn get_shadow_symbol(&mut self, sym: Symbol, entering_scope: bool) -> Option<Symbol> {
///       pub fn enter_scope<T>(&mut self, entering_vars: T)
///       pub fn enter_scope_after_renaming<'a>(
///       pub fn exit_scope(&mut self) {

struct ShadowStack {
    /// unique shadow var for each "lambda free var", a free variable from any lambda parameter.
    shadow_symbols: BTreeMap<Symbol, Symbol>,

    /// inverse of shadow_symbols for more efficient scoping
    shadow_symbols_inverse: BTreeMap<Symbol, Symbol>,

    /// subset of lambda free vars shadowed at each scope
    scoped_shadowed_vars: Vec<Vec<Symbol>>,

    /// maps each of "lambda free var" to 0 initially, incremented as scopes are entered
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

    // Proactively create a shadow symbol for every free variable, storing them in a map.
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

    // If a var is a free variable which is currently shadowed, then gets the shadow variable,
    // else None.
    //
    // If entering_scope, then the free variable is rewritten even if we're not yet in a scope,
    // since we are about to enter one.
    pub fn get_shadow_symbol(&mut self, sym: Symbol, entering_scope: bool) -> Option<Symbol> {
        if self
            .scoped_shadowed_count
            .get(&sym)
            .map(|count| if entering_scope { *count + 1 } else { *count })
            .unwrap_or(0)
            > 0
        {
            let new_sym = self
                .shadow_symbols
                .get(&sym)
                .expect("Inconsistency in free-var handling in inlining");
            Some(*new_sym)
        } else {
            None
        }
    }

    // Record that the provided symbols have local definitions, so should be shadowed.
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
                .expect("shadow counter for free var") += 1;
        }
        self.scoped_shadowed_vars.push(entering_free_vars);
    }

    // Record that the provided symbols have local definitions, so should be shadowed.
    // In this case, shadowed variables have already been renamed, so they must be mapped back.
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

    // Unshadow the set of symbols from the most recent scope which has been entered and not exited
    // yet.
    pub fn exit_scope(&mut self) {
        let exiting_free_vars = self
            .scoped_shadowed_vars
            .pop()
            .expect("Scope misalignment in inlining");
        for free_var in exiting_free_vars {
            *self
                .scoped_shadowed_count
                .get_mut(&free_var)
                .expect("failed invariant in ShadowStack implementation") -= 1;
        }
    }
}

/// Rewriter for transforming an inlined function call body into an expression to simply evaluate.
/// This involves just replacing variables and instantiating Lambda calls.
struct InlinedRewriter<'env, 'rewriter> {
    env: &'env GlobalEnv,
    type_args: &'rewriter Vec<Type>,
    lambda_param_map: BTreeMap<Symbol, &'rewriter Exp>,
    inlined_formal_params: Vec<Parameter>,

    // Shadow stack tracks whether free variables are hidden by local variable declarations.
    shadow_stack: ShadowStack,

    debug: bool,
}

impl<'env, 'rewriter> InlinedRewriter<'env, 'rewriter> {
    fn new(
        env: &'env GlobalEnv,
        type_args: &'rewriter Vec<Type>,
        inlined_formal_params: Vec<Parameter>,
        lambda_param_map: BTreeMap<Symbol, &'rewriter Exp>,
        lambda_free_vars: BTreeSet<Symbol>,
        debug: bool,
    ) -> Self {
        let shadow_stack = ShadowStack::new(env, &lambda_free_vars);
        Self {
            env,
            type_args,
            lambda_param_map,
            inlined_formal_params,
            shadow_stack,
            debug,
        }
    }

    /// Any free var
    fn shadowing_enter_scope(&mut self, entering_vars: Vec<Symbol>) {
        self.shadow_stack.enter_scope(entering_vars);
    }

    fn inline_call(
        env: &'env GlobalEnv,
        call_node_id: NodeId,
        func_loc: &Loc,
        body: &Exp,
        _type_parameters: Vec<TypeParameter>,
        type_args: Vec<Type>,
        parameters: Vec<Parameter>,
        args: &[Exp],
        result_type: Type,
        debug: bool,
    ) -> Exp {
        let args_matched: Vec<(&Parameter, &Exp)> = zip(&parameters, args).collect();
        let (lambda_args_matched, regular_args_matched): (
            Vec<(&Parameter, &Exp)>,
            Vec<(&Parameter, &Exp)>,
        ) = args_matched
            .iter()
            .partition(|(_, arg)| matches!(arg.as_ref(), ExpData::Lambda(..)));
        let non_lambda_function_args = regular_args_matched.iter().filter_map(|(param, exp)| {
            if let Type::Fun(_, _) = param.1 {
                Some((param, exp))
            } else {
                None
            }
        });

        for (param, exp) in non_lambda_function_args {
            env.error(
                &env.get_node_loc(exp.as_ref().node_id()),
                "Inline function-typed parameter currently must be a literal lambda expression",
            );
            if debug {
                eprintln!(
                    "bad exp is {:?}, param is {:?}, sym is `{}` type is `{:?}`",
                    exp,
                    param,
                    param.0.display(env.symbol_pool()),
                    param.1
                );
            }
        }

        // let type_param_map: BTreeMap<&TypeParameter, &Type> =
        //     zip(&type_parameters, &type_args).collect();
        let lambda_param_map: BTreeMap<Symbol, &Exp> = lambda_args_matched
            .iter()
            .map(|(param, exp)| (param.0, *exp))
            .collect();

        if debug {
            eprintln!("lambda_param_map is `{:#?}`", &lambda_param_map);
        }

        let (regular_params, regular_actuals): (Vec<&Parameter>, Vec<&Exp>) =
            regular_args_matched.into_iter().unzip();

        if debug {
            eprintln!("regular_parms are `{:#?}`", &regular_params);
            eprintln!("regular_actuals are `{:#?}`", &regular_actuals);
        }

        // Find free variables in lambda expr.  Perhaps we could minimize changes if we tracked
        // each lambda arg individually in the inlined method and only rewrite the context of each
        // inlined lambda, but that seems quite difficult.  Instead, just group all the free vars together
        // and shadow them all.
        let all_lambda_free_vars: BTreeSet<_> = lambda_args_matched
            .iter()
            .flat_map(|(_, exp)| exp.get_free_local_vars().into_iter())
            .collect();

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

        if debug {
            eprintln!("lambda_free_vars are `{:#?}`", &all_lambda_free_vars);
        }

        // rewrite body with type_args, lambda params, and var renames to keep lambda free vars free.
        let mut rewriter = InlinedRewriter::new(
            env,
            &type_args,
            parameters.clone(),
            lambda_param_map,
            all_lambda_free_vars,
            debug,
        );

        // Rewrite types in result type.
        let rewritten_result_type = result_type.instantiate(&type_args);

        // For now, just copy the actuals.  If FreezeRef is needed, we'll do it in
        // construct_inlined_call_expression.
        let rewritten_actuals: Vec<Exp> = regular_actuals.into_iter().cloned().collect();

        // Turn list of parameters into a pattern.  Also rewrite types as needed.
        // Shadow param vars as if we are in a let.
        let params_pattern = rewriter.parameter_list_to_pattern(env, func_loc, regular_params);

        // Enter the scope defined by the params.
        rewriter.shadowing_enter_scope(regular_params_overlapping_free_vars);

        // Rewrite body types, shadowed vars, replace invoked lambda params, etc.
        if debug {
            eprintln!("rewriting body `{:#?}`", &body);
        }
        let rewritten_body = rewriter.rewrite_exp(body.clone());
        if debug {
            eprintln!("rewritten body is `{:#?}`", &rewritten_body);
        }

        let call_loc = env.get_node_loc(call_node_id);

        InlinedRewriter::construct_inlined_call_expression(
            env,
            &call_loc,
            func_loc,
            rewritten_body,
            params_pattern,
            rewritten_actuals,
            rewritten_result_type,
        )

        // TODO
        // 6. Specs?
    }

    // Convert a list of Parameters into a Pattern.
    // Check for conflits between lambda_free_vars and symbols in Parameters,
    // replacing them by shadow symbols.
    // Also remap types according to type_param_map as needed.
    fn parameter_list_to_pattern(
        &mut self,
        env: &'env GlobalEnv,
        function_loc: &Loc,
        parameters: Vec<&Parameter>,
    ) -> Pattern {
        let tuple_args: Vec<Pattern> = parameters
            .iter()
            .map(|param| {
                let Parameter(sym, ty) = *param;
                // TODO: ideally, each Parameter has its own loc.  For now, we use the function location.
                // body should have types rewritten, other inlining complete, lambdas inlined, etc.
                let id = env.new_node(function_loc.clone(), ty.instantiate(self.type_args));
                if let Some(new_sym) = self.shadow_stack.get_shadow_symbol(*sym, true) {
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
        let id = env.new_node(function_loc.clone(), tuple_type);
        Pattern::Tuple(id, tuple_args)
    }

    // Build an expression corresponding to an inlined function (either lambda or inline function),
    // essentially equivalent to { let pattern=args; body }.
    //
    // body should already have types rewritten, other inlining complete, lambdas inlined, etc.
    // all types in args, body, parameters should also be rewritten (type params instantiated) as necessary.
    // parameters and args should be only non-lambda regular ordinary values (not types).
    fn construct_inlined_call_expression(
        env: &'env GlobalEnv,
        invocation_loc: &Loc,
        _function_loc: &Loc,
        body: Exp,
        pattern: Pattern,
        args: Vec<Exp>,
        _result_type: Type,
    ) -> Exp {
        // Process Body
        let body_node_id = body.as_ref().node_id();
        let body_type = env.get_node_type(body_node_id);
        let body_loc = env.get_node_loc(body_node_id);

        let new_body_id = env.new_node(body_loc, body_type.clone());

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
                            let new_exp_vec: Vec<Exp> = [exp.clone()].to_vec();
                            (
                                Exp::from(ExpData::Call(new_node, Operation::Freeze, new_exp_vec)),
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
                .unwrap_or_else(|| invocation_loc.clone());

            let new_args_id = env.new_node(args_loc, args_type);
            let new_args_expr =
                ExpData::Call(new_args_id, Operation::Tuple, rewritten_args).into_exp();
            Some(new_args_expr)
        };

        let new_body = ExpData::Block(new_body_id, pattern, optional_new_args_expr, body);
        new_body.into_exp()
    }

    // Helper for construct_inlined_call_expression.
    //
    // If `pattern-type` is a tuple of same length as `arg_vec`, and types differ just in mutability of the
    // reference type, where the param is immutable and the arg is mutable, returns `Some(vec)`
    // where such corresponding elements are true, indicating that a `FreezeRef` could be inserted
    // to gain type compatibility.
    //
    // If there are no such parameters, returns None.
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

    // Helper for check_pattern_args_types_need_freezeref
    //
    // If any corresponding elements of `param_vec` and `arg_vec` differ just in mutability of the
    // reference type, where the param is immutable and the arg is mutable, returns `Some(vec)`
    // where such corresponding elements are true, indicating that a `FreezeRef` could be inserted
    // to gain type compatibility.
    //
    // If there are no such parameters, returns None.
    fn check_params_args_types_vectors_need_freezeref(
        params_types: &Vec<Type>,
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
                    } else if let Type::Reference(kind1, box_t1) = t1 {
                        if let Type::Reference(kind2, box_t2) = t2 {
                            *box_t1 == *box_t2
                                && *kind1 == ReferenceKind::Immutable
                                && *kind2 == ReferenceKind::Mutable
                        } else {
                            false
                        }
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

    // Helper function for `rewrite_pattern` in trait `ExpRewriterFunctions` below.
    // If any subpattern gets simplified, replace the whole thing.
    fn rewrite_pattern_vector(
        &mut self,
        pat_vec: &[Pattern],
        entering_scope: bool,
    ) -> Option<Vec<Pattern>> {
        let rewritten_part: Vec<_> = pat_vec
            .iter()
            .map(|pat| self.rewrite_pattern(pat, entering_scope))
            .collect();
        if rewritten_part.iter().any(|opt_pat| opt_pat.is_some()) {
            // if any subpattern was simplified, then rebuild the vector
            // with a combination of original and new patterns.
            let rewritten_vec: Vec<_> = pat_vec
                .iter()
                .zip(rewritten_part)
                .map(|(org_pat, opt_new_pat)| opt_new_pat.unwrap_or(org_pat.clone()))
                .collect();
            Some(rewritten_vec)
        } else {
            None
        }
    }

    // Convert a single-variable pattern into a tuple if needed.
    fn make_lambda_pattern_a_tuple(&mut self, pat: &Pattern) -> Pattern {
        if let Pattern::Var(id, _) = pat {
            let new_id = self.env.new_node(
                self.env.get_node_loc(*id),
                Type::Tuple([self.env.get_node_type(*id)].to_vec()),
            );
            Pattern::Tuple(new_id, [pat.clone()].to_vec())
        } else {
            pat.clone()
        }
    }
}

impl<'env, 'rewriter> ExpRewriterFunctions for InlinedRewriter<'env, 'rewriter> {
    // Override default implementation to flag an error on an inlined Return expressions.
    fn rewrite_exp(&mut self, exp: Exp) -> Exp {
        // Disallow Return expression in an inlined function.
        if let ExpData::Return(id, _val) = exp.as_ref() {
            self.env.error(
                &self.env.get_node_loc(*id),
                "Return not currently supported in inline functions",
            );
        }
        // Proceed with default behavior in any case.
        self.rewrite_exp_descent(exp)
    }

    // Record that the provided symbols have local definitions, so renaming should be done.
    // Note that incoming vars are from a Pattern *after* renaming, so these are shadowed symbols.
    fn rewrite_enter_scope<'a>(&mut self, vars: impl Iterator<Item = &'a (NodeId, Symbol)>) {
        self.shadow_stack
            .enter_scope_after_renaming(vars.map(|(_, sym)| sym));
    }

    // On exiting a scope defining some symbols shadowing lambda free vars, record that we have
    // exited the scope so any occurrences of those free vars should be left alone (if there are
    // not further shadowing scopes furter out).
    fn rewrite_exit_scope(&mut self) {
        self.shadow_stack.exit_scope();
    }

    fn rewrite_node_id(&mut self, id: NodeId) -> Option<NodeId> {
        ExpData::instantiate_node(self.env, id, self.type_args)
    }

    fn rewrite_local_var(&mut self, id: NodeId, sym: Symbol) -> Option<Exp> {
        let result = self
            .shadow_stack
            .get_shadow_symbol(sym, false)
            .map(|new_sym| ExpData::LocalVar(id, new_sym).into());
        if self.debug {
            eprintln!("rewriting local var {:#?} into {:#?}", sym, result,);
        };
        result
    }

    fn rewrite_temporary(&mut self, id: NodeId, idx: TempIndex) -> Option<Exp> {
        let loc = self.env.get_node_loc(id);
        if idx < self.inlined_formal_params.len() {
            let param = &self.inlined_formal_params[idx];
            let sym = param.0;
            let param_type = &param.1;
            let new_node_id = self.env.new_node(loc, param_type.clone());
            if let Some(new_sym) = self.shadow_stack.get_shadow_symbol(sym, false) {
                Some(ExpData::LocalVar(new_node_id, new_sym).into())
            } else {
                Some(ExpData::LocalVar(new_node_id, sym).into())
            }
        } else {
            self.env.error(
                &loc,
                &format!("Temporary with invalid index `{}` during inlining of function with `{}` parameters",
                        idx, self.inlined_formal_params.len()),
            );
            None
        }
    }

    fn rewrite_invoke(&mut self, id: NodeId, target: &Exp, args: &[Exp]) -> Option<Exp> {
        let mut target_id = None;
        let optional_lambda_target: Option<&Exp> = match target.as_ref() {
            ExpData::LocalVar(node_id, symbol) => {
                target_id = Some(node_id);
                self.lambda_param_map.get(symbol).copied()
            },
            ExpData::Temporary(node_id, idx) => {
                if *idx < self.inlined_formal_params.len() {
                    target_id = Some(node_id);
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
            if let ExpData::Lambda(lambda_id, pat, body) = lambda_target.as_ref() {
                let lambda_loc = self.env.get_node_loc(*lambda_id);
                let body_node_id = body.as_ref().node_id();
                let body_type = self.env.get_node_type(body_node_id);
                let args_vec: Vec<Exp> = args.to_vec();
                Some(InlinedRewriter::construct_inlined_call_expression(
                    self.env,
                    &call_loc,
                    &lambda_loc,
                    body.clone(),
                    self.make_lambda_pattern_a_tuple(pat),
                    args_vec,
                    body_type,
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
            let target_loc = target_id
                .map(|id| self.env.get_node_loc(*id))
                .unwrap_or(call_loc);
            self.env.error(
                &target_loc,
                "Invalid call target: currently indirect call must be a parameter to an inline function called with an argument which is a literal lambda expression",
            );
            None
        }
    }

    fn rewrite_pattern(&mut self, pat: &Pattern, entering_scope: bool) -> Option<Pattern> {
        let result = match pat {
            Pattern::Var(node_id, sym) => self
                .shadow_stack
                .get_shadow_symbol(*sym, entering_scope)
                .map(|new_sym| Pattern::Var(*node_id, new_sym)),
            Pattern::Tuple(node_id, pattern_vec) => self
                .rewrite_pattern_vector(pattern_vec, entering_scope)
                .map(|rewritten_vec| Pattern::Tuple(*node_id, rewritten_vec)),
            Pattern::Struct(node_id, struct_id, pattern_vec) => self
                .rewrite_pattern_vector(pattern_vec, entering_scope)
                .map(|rewritten_vec| Pattern::Struct(*node_id, struct_id.clone(), rewritten_vec)),
            Pattern::Wildcard(_) => None,
            Pattern::Error(_) => None,
        };
        if self.debug {
            eprintln!("rewriting pattern {:#?} into {:#?}", pat, result);
        };
        result
    }
}
