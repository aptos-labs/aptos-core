// Copyright © Velor Foundation
// SPDX-License-Identifier: Apache-2.0

//! AST Simplifier
//!
//! Simplify the AST before conversion to bytecode.
//! - flow-insensitive constant propagation and folding
//! - simple expression simplification
//!
//! Preconditions:
//! - have previously run `check_for_unused_vars_and_params` pass to warn about unused vars,
//!   which may be eliminated here.
//!
//! More details:
//! - Do flow-insensitive constant propagation:
//!   - identify "possibly modified" symbols which may have more than one value,
//!     or may have their value moved (possibly leaving none)
//!   - for safe symbols whose value is a constant value, propagate
//!     the value to the use site to enable simplifying code:
//!     - inline a constant value
//! - Implement ExpRewriterFunctions to allow bottom-up replacement
//!   of some complex expressions by "simpler" ones:
//!   - Constant folding of operations with constant parameters which
//!     do not abort (have arguments in range).
//!   - TODO(#12472) Flag In the future, we could warn about expressions
//!     which are guaranteed to abort, but it's nontrivial so deferred.
//!   - Eliminate unused expressions (with a warning)
//!   - Eliminate used variables whose uses are all eliminated by
//!     other simplification optimizations (e.g., constant folding)
//!   - Eliminate unused value expressions which are side-effect-free.
//!   - Unwrap trivial compound expressions:
//!     - a Sequence of 1 expression
//!     - a Block with no variable binding
//!   - Simple call rewriting: (one example)
//!     - eliminate cast to same type as parameter
//!
//! - Optionally do some simplifications that may eliminate dead
//!   code and hide some warnings:
//!     - eliminate side-effect-free expressions with ignored value
//!       in a `Sequence` instruction.
//!     - eliminate unused variable assignments in a `let` statement,
//!       and unassigned values expressions from `let` RHS which are
//!       side-effect-free.
//!     - use constant folding on if predicates to eliminate dead
//!       then or else branches (currently disabled by local constant,
//!       as it may eliminate some useful code diagnostics).
//!     - warn about dead code in the cases above where a statement
//!       whose value is not used is removed.

use codespan_reporting::diagnostic::Severity;
use itertools::Itertools;
use log::{debug, log_enabled, trace, Level};
use move_core_types::ability::Ability;
use move_model::{
    ast::{Exp, ExpData, Operation, Pattern, Value, VisitorPosition},
    constant_folder::ConstantFolder,
    exp_rewriter::ExpRewriterFunctions,
    model::{FunctionEnv, GlobalEnv, NodeId, Parameter},
    symbol::Symbol,
    ty::{ReferenceKind, Type, TypeDisplayContext},
};
use std::{
    collections::{BTreeMap, BTreeSet},
    fmt::Debug,
    iter::{IntoIterator, Iterator},
    vec::Vec,
};

const DEBUG: bool = false;

/// Run the AST simplification pass on all target functions in the `env`.
/// Optionally do some aggressive simplifications that may eliminate code.
pub fn run_simplifier(env: &mut GlobalEnv, eliminate_code: bool) {
    let mut new_definitions = Vec::new(); // Avoid borrowing issues for env.
    for module in env.get_modules() {
        if module.is_target() {
            for func_env in module.get_functions() {
                if let Some(def) = func_env.get_def() {
                    let mut rewriter = SimplifierRewriter::new(&func_env, eliminate_code);
                    let rewritten = rewriter.rewrite_function_body(def.clone());
                    trace!(
                        "After rewrite_function_body, function body is `{}`",
                        rewritten.display(env)
                    );

                    if !ExpData::ptr_eq(&rewritten, def) {
                        new_definitions.push((func_env.get_qualified_id(), rewritten));
                    }
                }
            }
        }
    }
    // Actually do the writing of new definitions.
    for (qfid, def) in new_definitions.into_iter() {
        env.set_function_def(qfid, def);
        if DEBUG {
            debug!(
                "After simplifier, function is `{}`",
                env.dump_fun(&env.get_function(qfid))
            );
        }
    }
}

/// ScopedMap<K, V> provides a simple sort of
/// `BTreeMap<K, V>` which can be checkpointed
/// and restored, as when descending function scopes.
/// - Operations `new()`, `clear()`, `insert(K, V)`,
///   `remove(K)`, `get(&K)`, and `contains_key(&K)`
///    act like operations on `BTreeMap`.
/// - `enter_scope()` checkpoints the current map state
///   on a stack of scopes.
/// - `exit_scope()` restores map to the corresponding
///   previous state.
#[derive(Debug)]
struct ScopedMap<K, V> {
    // The implementation uses a stack of maps, with
    // `get` operation checking maps in order, stopping
    // when a value is found.
    //
    // The maps use `Option<V>` as the value so that
    // `remove(K)` can hide values saved in outer scopes
    // by setting the current scope value to `None`.
    maps: Vec<BTreeMap<K, Option<V>>>,
}

impl<K, V> ScopedMap<K, V>
where
    K: Ord + Copy,
    V: Clone,
{
    pub fn new() -> Self {
        Self {
            maps: vec![BTreeMap::new()],
        }
    }

    pub fn clear(&mut self) {
        self.maps.clear();
        self.maps.push(BTreeMap::new());
    }

    pub fn enter_scope(&mut self) {
        self.maps.push(BTreeMap::new());
    }

    // Restore `values` to what they were before the corresponding
    // `enter_scope` call.
    pub fn exit_scope(&mut self) {
        self.maps.pop().expect("Bug: imbalanced enter/exit");
    }

    // Set a `value` for `key`, valid until the current scope is
    // exited.
    pub fn insert(&mut self, key: K, value: V) {
        let mut top_map = self.maps.pop().expect("imbalanced enter/exit");
        top_map.insert(key, Some(value));
        self.maps.push(top_map);
    }

    #[allow(unused)]
    // Remove any value for `key` for the current scope.
    pub fn remove(&mut self, key: K) {
        let mut top_map = self.maps.pop().expect("imbalanced enter/exit");
        top_map.insert(key, None);
        self.maps.push(top_map);
    }

    pub fn get(&self, key: &K) -> Option<&V> {
        for scope in self.maps.iter().rev() {
            if let Some(value) = scope.get(key) {
                return value.as_ref();
            }
        }
        None
    }

    #[allow(unused)]
    pub fn contains_key(&self, key: &K) -> bool {
        let x = self.get(key);
        x.is_some()
    }
}

// Finds local vars that may be modified, and shouldn't be treated as constant.
//
// Vars are identified by symbol name and by the scope in which they are defined.
//  Scope is either
// - `None`: procedure parameter scope (uses are usually a temporary but may not be, notably in
//    the case of `Assign`)
// - `Some(NodeId)`: the let which creates the variable scope.
//
// Note that as expression simplification occurs, the `NodeId` of the original `Let` expression
// may change/disappear, but not until the scope is exited.  So the "possibly modified" property
// shouldn't be trusted after that.
fn find_possibly_modified_vars(
    env: &GlobalEnv,
    params: &[Parameter],
    exp: &ExpData,
) -> BTreeSet<(Symbol, Option<NodeId>)> {
    // Maps each symbol to the nearest enclosing `let` (identified by `NodeId`) binding it
    let mut visiting_binding: ScopedMap<Symbol, NodeId> = ScopedMap::new();
    // Identifies modified `LocalVar`s by `Symbol` and the `let` (identified by `Some(NodeId)`) binding it
    // Also includes possibly modified parameters and free variables paired with `None`.
    let mut possibly_modified_vars: BTreeSet<(Symbol, Option<NodeId>)> = BTreeSet::new();
    // Reverse map from `Symbol` to `idx` to facilitate checking of `Assign` validity.
    let param_map: BTreeMap<_, _> = params
        .iter()
        .enumerate()
        .map(|(idx, p)| (p.0, idx))
        .collect();

    // Track when we are in a modifying scope.
    let mut modifying = false;
    // Use a stack to keep the modification status properly scoped.
    let mut modifying_stack = Vec::new();

    exp.visit_positions(&mut |pos, e| {
        use ExpData::*;
        match e {
            Invalid(_) | Value(..) | LoopCont(..) => {
                // Nothing happens inside these expressions, so don't bother `modifying` state.
            },
            LocalVar(id, sym) => {
                let current_binding_id_opt = visiting_binding.get(sym);
                if let Some(current_binding_id) = current_binding_id_opt {
                    if modifying {
                        trace!(
                            "Var `{}` in binding `{}` is possibly modified at node `{}`",
                            sym.display(env.symbol_pool()),
                            current_binding_id.as_usize(),
                            id.as_usize(),
                        );
                        possibly_modified_vars.insert((*sym, current_binding_id_opt.copied()));
                    }
                } else {
                    match param_map.get(sym) {
                        None => {
                            trace!(
                                "Var `{}` used at node `{}` as a `LocalVar` is free and not a parameter",
                                sym.display(env.symbol_pool()),
                                id.as_usize(),
                            );
                        },
                        Some(idx) => {
                            trace!(
                                "Temp `{}` = Var `{}` is used at node `{}` as a `LocalVar`",
                                *idx,
                                sym.display(env.symbol_pool()),
                                id.as_usize(),
                            );
                        }
                    }
                    if modifying {
                        trace!(
                            "LocalVar `{}` with no binding is possibly modified at node `{}`",
                            sym.display(env.symbol_pool()),
                            id.as_usize(),
                        );
                        possibly_modified_vars.insert((*sym, None));
                    }
                }
            },
            Temporary(id, idx) => {
                if let Some(sym) = params.get(*idx).map(|p| p.0) {
                    if modifying {
                        trace!(
                            "Temp `{}` = Var `{}` is possibly modified at node `{}`",
                            *idx,
                            sym.display(env.symbol_pool()),
                            id.as_usize(),
                        );
                        if let Some(current_binding_id) = visiting_binding.get(&sym) {
                            let loc = env.get_node_loc(*id);
                            let loc2 = env.get_node_loc(*current_binding_id);
                            env.diag_with_labels(
                                Severity::Bug,
                                &loc,
                                &format!(
                                    "Temp `{}` = Var `{}` is used inside aliasing let at node `{}`",
                                    *idx,
                                    sym.display(env.symbol_pool()),
                                    id.as_usize()
                                ),
                                vec![(loc2, "Aliasing let here".to_string())],
                            );
                        }
                        possibly_modified_vars.insert((sym, None));
                    };
                } else {
                    let loc = env.get_node_loc(*id);
                    env.diag(
                        Severity::Bug,
                        &loc,
                        &format!("Use of temporary `{}` with no corresponding parameter", idx),
                    )
                }
            },
            Call(_, op, _explist) => {
                trace!(
                    "In find_possibly_modified_vars, looking at Call expr `{}`",
                    e.display_verbose(env)
                );
                match op {
                    // NOTE: we don't consider values in globals, so no need to
                    // consider BorrowGlobal(ReferenceKind::Mutable)} here.
                    //
                    // A top-level value in a Borrow(Mutable) is treated as if
                    // it is mutated, though this is a tiny bit conservative
                    // (possibly the value could be discarded).
                    Operation::Borrow(ReferenceKind::Mutable) |
                    // A top-level argument to a `Move` operation (always a variable)
                    // is modified.
                    //
                    Operation::Move => {
                        match pos {
                            VisitorPosition::Pre => {
                                // Add all mentioned vars to modified vars.
                                modifying_stack.push(modifying);
                                modifying = true;
                                trace!("Entering Borrow/MoveTo/From");
                            },
                            VisitorPosition::Post => {
                                // stop adding vars
                                modifying = modifying_stack.pop().expect("unbalanced visit 1");
                                trace!("Exiting Borrow/MoveTo/From");
                            },
                            _ => {},
                        }
                    },
                    Operation::Select(..) => {
                        // Variable appearing in Select argument may be borrowed if it occurs in
                        // a Borrow parameter, so leave modifying state alone.  Note that other
                        // modification contexts (e.g., MoveFrom) cannot have a `Select` in their
                        // subexpressions.
                    },
                    _ => {
                        // Other operations don't modify argument variables, so turn off `modifying`
                        // inside.
                        match pos {
                            VisitorPosition::Pre => {
                                modifying_stack.push(modifying);
                                modifying = false;
                            },
                            VisitorPosition::Post => {
                                modifying = modifying_stack.pop().expect("unbalanced visit 4");
                            },
                            _ => {},
                        }
                    },
                };
            },
            Invoke(..) | Return(..) | Quant(..) | Loop(..) | Mutate(..) | SpecBlock(..) => {
                // We don't modify top-level variables here.
                match pos {
                    VisitorPosition::Pre => {
                        modifying_stack.push(modifying);
                        modifying = false;
                    },
                    VisitorPosition::Post => {
                        modifying = modifying_stack.pop().expect("unbalanced visit 5");
                    },
                    _ => {},
                }
            },
            Lambda(node_id, pat, _, _, _) => {
                // Define a new scope for bound vars, and turn off `modifying` within.
                match pos {
                    VisitorPosition::Pre => {
                        trace!("Entering lambda {}", node_id.as_usize());
                        visiting_binding.enter_scope();
                        for (_, sym) in pat.vars() {
                            visiting_binding.insert(sym, *node_id);
                        }
                        modifying_stack.push(modifying);
                        modifying = false;
                    },
                    VisitorPosition::Post => {
                        // remove a scope
                        visiting_binding.exit_scope();
                        trace!("Exiting lambda {}", node_id.as_usize());
                        modifying = modifying_stack.pop().expect("unbalanced visit 6");
                    },
                    _ => {},
                };
            },
            Block(node_id, pat, _, _) => {
                // Define a new scope for bound vars, and turn off `modifying` within.
                match pos {
                    VisitorPosition::Pre => {
                        modifying_stack.push(modifying);
                        modifying = false;
                        trace!(
                            "Entering block -- evaluating binding RHS {}",
                            node_id.as_usize()
                        );
                    },
                    VisitorPosition::BeforeBody => {
                        trace!("Entering block scope {}", node_id.as_usize());
                        visiting_binding.enter_scope();
                        for (_, sym) in pat.vars() {
                            visiting_binding.insert(sym, *node_id);
                        }
                    },
                    VisitorPosition::Post => {
                        // remove a scope
                        visiting_binding.exit_scope();
                        trace!("Exiting block scope {}", node_id.as_usize());
                        modifying = modifying_stack.pop().expect("unbalanced visit 7");
                    },
                    _ => {},
                };
            },
            Match(node_id, _, arms) => {
                match pos {
                    VisitorPosition::Pre => {
                        modifying_stack.push(modifying);
                        modifying = false;
                    }
                    VisitorPosition::BeforeMatchBody(idx) => {
                        let arm = &arms[idx];
                        visiting_binding.enter_scope();
                        for (_, sym) in arm.pattern.vars() {
                            visiting_binding.insert(sym, *node_id);
                        }
                    }
                    VisitorPosition::AfterMatchBody(_) => {
                        visiting_binding.exit_scope();
                    }
                    VisitorPosition::Post => {
                        modifying = modifying_stack.pop().expect("unbalanced visit 8");
                    }
                    _ => {}
                }
            }
            IfElse(..) | Sequence(..) => {
                match pos {
                    VisitorPosition::Pre => {
                        modifying_stack.push(modifying);
                        modifying = false;
                    },
                    VisitorPosition::Post => {
                        modifying = modifying_stack.pop().expect("unbalanced visit 8");
                    },
                    _ => {},
                };
            },
            Assign(id, pat, _) => {
                match pos {
                    VisitorPosition::Pre => {
                        // add vars in pat to modified vars, then turn off modifying for the RHS
                        for (_pat_var_id, sym) in pat.vars() {
                            let current_binding_id_opt = visiting_binding.get(&sym);
                            if let Some(current_binding_id) = current_binding_id_opt {
                                trace!(
                                    "Var {} in binding {} is assigned in node {} so is modified",
                                    sym.display(env.symbol_pool()),
                                    current_binding_id.as_usize(),
                                    id.as_usize()
                                );
                            } else {
                                match param_map.get(&sym) {
                                    Some(idx) => {
                                        trace!(
                                            "Var `{}` assigned at node `{}` is Temp `{}`",
                                            sym.display(env.symbol_pool()),
                                            id.as_usize(),
                                            *idx,
                                        );
                                    },
                                    None => {
                                        let loc = env.get_node_loc(*id);
                                        env.diag(
                                            Severity::Bug,
                                            &loc,
                                            &format!(
                                                "Free symbol {} in assignment",
                                                sym.display(env.symbol_pool())
                                            ),
                                        );
                                    },
                                }
                            }
                            possibly_modified_vars.insert((sym, current_binding_id_opt.copied()));
                        }
                        // RHS is not modifying, turn it off.
                        modifying_stack.push(modifying);
                        modifying = false;
                    },
                    VisitorPosition::Post => {
                        modifying = modifying_stack.pop().expect("unbalanced visit 9");
                    },
                    _ => {},
                };
            },
        };
        true
    });
    possibly_modified_vars
}

/// A function-specific simplifier rewriter.
struct SimplifierRewriter<'env> {
    pub func_env: &'env FunctionEnv<'env>,

    pub constant_folder: ConstantFolder<'env>,

    // Guard whether entire subexpressions are eliminated (possibly hiding some warnings).
    pub eliminate_code: bool,

    // Tracks which definition (`Let` or `Lambda` statement `NodeId`) is visible during visit to
    // find modified local vars.  A use of a symbol which is missing must be a `Parameter`.  This is
    // used only to determine if a symbol is in `possibly_modified_variables`.
    visiting_binding: ScopedMap<Symbol, NodeId>,

    // Possibly modified variables are identified by `Symbol` and `Let` or `Lambda` statement `NodeId`,
    // except enclosing function parameters, which have no `NodeId` so get `None`.
    possibly_modified_variables: BTreeSet<(Symbol, Option<NodeId>)>,

    // Tracks constant values from scope.
    values: ScopedMap<Symbol, SimpleValue>,

    // During expression rewriting, tracks whether we are evaluating a mutable borrow argument.
    in_mut_borrow: bool,

    // Records values from outer scope of a borrow.
    in_mut_borrow_stack: Vec<bool>,
}

// Representation to record a known value of a variable to
// allow simplification.  Currently we only track constant values
// and definitely uninitialized values (from `let` with no binding).
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
enum SimpleValue {
    Value(Value),
    Uninitialized,
}

impl<'env> SimplifierRewriter<'env> {
    fn new(func_env: &'env FunctionEnv, eliminate_code: bool) -> Self {
        let constant_folder = ConstantFolder::new(func_env.module_env.env, false);
        Self {
            func_env,
            constant_folder,
            eliminate_code,
            visiting_binding: ScopedMap::new(),
            possibly_modified_variables: BTreeSet::new(),
            values: ScopedMap::new(),
            in_mut_borrow: false,
            in_mut_borrow_stack: Vec::new(),
        }
    }

    fn env(&self) -> &GlobalEnv {
        self.func_env.module_env.env
    }

    fn debug_dump_variables(&self) {
        trace!(
            "Possibly modified variables are ({:#?})",
            self.possibly_modified_variables
                .iter()
                .map(|(sym, opt_node)| format!(
                    "{}@{}",
                    sym.display(self.env().symbol_pool()),
                    if let Some(node) = opt_node {
                        node.as_usize().to_string()
                    } else {
                        "None".to_string()
                    }
                ))
                .join(", ")
        )
    }

    /// Process a function.
    pub fn rewrite_function_body(&mut self, exp: Exp) -> Exp {
        self.possibly_modified_variables = find_possibly_modified_vars(
            self.env(),
            self.func_env.get_parameters_ref(),
            exp.as_ref(),
        );
        self.visiting_binding.clear();
        self.values.clear();
        self.debug_dump_variables();
        // Enter Function scope (a specialized `rewrite_enter_scope()` call)
        self.values.enter_scope();

        for param in self.func_env.get_parameters_ref().iter() {
            let sym = param.0;
            self.values.remove(sym);
        }
        // Now rewrite the body
        self.rewrite_exp(exp)
    }

    /// If symbol `sym` has a recorded value that is currently visible, then
    /// build an expression to produce that value.
    fn rewrite_to_recorded_value(&mut self, id: NodeId, sym: &Symbol) -> Option<Exp> {
        if let Some(simple_value) = self.values.get(sym) {
            match simple_value {
                SimpleValue::Value(val) => Some(ExpData::Value(id, val.clone()).into_exp()),
                SimpleValue::Uninitialized => {
                    trace!(
                        "Var {} was uninitialized",
                        sym.display(self.env().symbol_pool()),
                    );
                    None
                },
            }
        } else {
            trace!(
                "Found no value for var {} ",
                sym.display(self.env().symbol_pool()),
            );
            None
        }
    }

    /// If `exp` can be represented as a `SimpleValue`, then return it.
    fn exp_to_simple_value(&mut self, exp: Option<Exp>) -> Option<SimpleValue> {
        // `exp` should have already been simplified so we only need to check
        // for a constant value expression here.
        if let Some(exp) = exp {
            match exp.as_ref() {
                ExpData::Value(_, val) => Some(SimpleValue::Value(val.clone())),
                _ => None,
            }
        } else {
            None
        }
    }

    /// Expand a `Value::Tuple` value expression to a call to `Tuple`
    /// Note that a `Value::Vector` value is left alone.
    fn expand_tuple(&mut self, exp: Exp) -> Exp {
        if let ExpData::Value(id, Value::Tuple(x)) = exp.as_ref() {
            if x.is_empty() {
                ExpData::Call(*id, Operation::Tuple, Vec::new()).into_exp()
            } else {
                let loc = self.env().get_node_loc(*id);
                self.env().diag(
                    Severity::Bug,
                    &loc,
                    &format!(
                        "Illegal use of non-empty Tuple constant of length {}",
                        x.len()
                    ),
                );
                exp
            }
        } else {
            exp
        }
    }

    /// Try to turn a call to cast(x:T1,:T1) -> x
    fn try_collapse_cast(&mut self, id: NodeId, arg0: &Exp) -> Option<Exp> {
        let arg0_type = self.env().get_node_type(arg0.node_id());
        let result_type = self.env().get_node_type(id);
        if arg0_type == result_type {
            Some(arg0.clone())
        } else {
            None
        }
    }
}

impl ExpRewriterFunctions for SimplifierRewriter<'_> {
    fn rewrite_exp(&mut self, exp: Exp) -> Exp {
        let old_id = exp.as_ref().node_id().as_usize();
        trace!(
            "Before rewrite, expr {} is `{}`",
            old_id,
            exp.display_verbose(self.env())
        );
        // Top-level vars in an argument to `Borrow` (possibly with a `Select` as well)
        // are borrowed directly, while if they occur in any other subexpression they will
        // be interpreted as a copy to a temp value which is borrowed instead of the var.
        //
        // That is ok for immutable borrows, but mutable ones may lead to modifications of
        // a temp instead of the desired variable.  Thus we track Mutable Borrows here.
        enum BorrowEffect {
            Borrowable,
            IsMutableBorrow,
            TransparentToBorrow,
            NotBorrowable,
        }
        use BorrowEffect::*;
        let borrow_effect = {
            if exp.is_directly_borrowable() {
                Borrowable
            } else if let ExpData::Call(_, op, _explist) = exp.as_ref() {
                match op {
                    Operation::Borrow(ReferenceKind::Mutable) => IsMutableBorrow,
                    // Leave in_mut_borrow alone
                    Operation::Select(..) => TransparentToBorrow,
                    // Other Call operations escape from the borrow
                    _ => NotBorrowable,
                }
            } else {
                NotBorrowable
            }
        };
        match &borrow_effect {
            TransparentToBorrow => {
                // no effect on `in_mut_borrow`. Depends on context.
            },
            Borrowable => {
                // no effect on `in_mut_borrow`, safe to rewrite ***if safe***,
                // since such rewrites are only safe if not in a Mutable borrow.
            },
            IsMutableBorrow => {
                // Turn on `in_mut_borrow`
                self.in_mut_borrow_stack.push(self.in_mut_borrow);
                self.in_mut_borrow = true;
            },
            NotBorrowable => {
                self.in_mut_borrow_stack.push(self.in_mut_borrow);
                self.in_mut_borrow = false;
            },
        };
        let rexp = self.rewrite_exp_descent(exp);
        let new_id = rexp.as_ref().node_id().as_usize();
        trace!(
            "After rewrite, expr {} is now {}: `{}`",
            old_id,
            new_id,
            rexp.display_verbose(self.env())
        );
        // Exit from local `in_mut_borrow` state, and if the enclosing scope was `in_mut_borrow`, then make
        // sure that anything transformed into a var or `Select` from another expression type is
        // wrapped by a `Sequence` so it will be treated as a temporary value to be borrowed.
        let protected_rexp = match &borrow_effect {
            TransparentToBorrow | Borrowable => {
                // It was already borrowable, don't need to check for unwrap.
                rexp // No effect.
            },
            IsMutableBorrow => {
                // Exit this `in_mut_borrow` scope
                self.in_mut_borrow = self
                    .in_mut_borrow_stack
                    .pop()
                    .expect("Imbalanced in_mut_borrow stack.");
                rexp
            },
            NotBorrowable => {
                // Exit `in_mut_borrow=false` scope
                self.in_mut_borrow = self
                    .in_mut_borrow_stack
                    .pop()
                    .expect("Imbalanced in_mut_borrow stack.");
                // If `in_mut_borrow` was true before, then we have to be careful
                // to make sure that we didn't unwrap a directly borrowable item.
                // For example, a sequence with 1 expression which is a `LocalVar`
                // will get unwrapped into just the `LocalVar` expression, which
                // is directly borrowable and can change behavior. To avoid that,
                // we check for such a case and wrap it in a `Sequence`.
                //
                // (This can happen when transforming other expressions than
                // `Sequence`, so it's easier to just undo them all here than
                // try to be clever when rewriting every kind of expression.)
                if self.in_mut_borrow {
                    // This expression is at top-level in a Borrow, and was not a Variable or Select.
                    // If we turned it into one, then wrap it in a `Sequence` to generate a temp value
                    // to be borrowed.
                    if rexp.is_directly_borrowable() {
                        use ExpData::*;
                        match rexp.as_ref() {
                            LocalVar(id, ..)
                            | Temporary(id, ..)
                            | Call(id, Operation::Select(..), _) => {
                                let cloned_id = self.env().clone_node(*id);
                                Sequence(cloned_id, vec![rexp]).into_exp()
                            },
                            _ => {
                                // Nothing to do.
                                rexp
                            },
                        }
                    } else {
                        rexp
                    }
                } else {
                    rexp
                }
            },
        };
        let protected_rexp_id = protected_rexp.as_ref().node_id().as_usize();
        trace!(
            "After rewrite2, {} is now protected_rexp {}: `{}`",
            new_id,
            protected_rexp_id,
            protected_rexp.display_verbose(self.env())
        );
        protected_rexp
    }

    fn rewrite_enter_scope<'a>(
        &mut self,
        _id: NodeId,
        vars: impl Iterator<Item = &'a (NodeId, Symbol)>,
    ) {
        self.visiting_binding.enter_scope();
        self.values.enter_scope();
        for (_, sym) in vars {
            self.values.remove(*sym);
        }
    }

    fn rewrite_exit_scope(&mut self, _id: NodeId) {
        self.visiting_binding.exit_scope();
        self.values.exit_scope();
    }

    fn rewrite_local_var(&mut self, id: NodeId, sym: Symbol) -> Option<Exp> {
        // Note that we could but don't need to check `in_mut_borrow` since if we have a value for a
        // variable here then the variable can't appear in a borrow position of a Mutable borrow
        // (see `find_possibly_modified_vars`).
        let result = self.rewrite_to_recorded_value(id, &sym);
        if log_enabled!(Level::Trace) {
            if let Some(exp) = &result {
                let in_scope = self.visiting_binding.get(&sym);
                let value = self.values.get(&sym);
                trace!(
                    "Replacing symbol {} at node {} with {}; in_scope={:?}, value={:?}",
                    sym.display(self.env().symbol_pool()),
                    id.as_usize(),
                    exp.display(self.env()),
                    in_scope.map(|n| n.as_usize()),
                    value
                );
                assert!(!self.in_mut_borrow);
            }
        }
        result
    }

    fn rewrite_call(&mut self, id: NodeId, oper: &Operation, args: &[Exp]) -> Option<Exp> {
        self.constant_folder
            .rewrite_call(id, oper, args)
            .map(|exp| self.expand_tuple(exp))
            .or_else(|| {
                // Not completely a constant.
                if *oper == Operation::Cast && args.len() == 1 {
                    self.try_collapse_cast(id, &args[0])
                } else {
                    // TODO(later): match some more interesting expressions.
                    // e.g., ((x + c1) + c2) -> (x + (c1 + c2))
                    None
                }
            })
    }

    fn rewrite_enter_block_scope(
        &mut self,
        id: NodeId,
        pat: &Pattern,
        binding: &Option<Exp>,
    ) -> Option<Pattern> {
        let mut new_binding = Vec::new();
        if let Some(exp) = binding {
            for (var, opt_new_binding_exp) in pat.vars_and_exprs(exp) {
                if self.possibly_modified_variables.contains(&(var, Some(id))) {
                    // Ignore RHS, mark this variable as possibly modified.
                    new_binding.push((var, None));
                } else {
                    // Try to evaluate opt_new_binding_exp as a constant/var.
                    // If unrepresentable as a Value, returns None.
                    new_binding.push((var, self.exp_to_simple_value(opt_new_binding_exp)));
                }
            }
        } else {
            // Body with no bindings, values are Uninitialized.
            for (_, var) in pat.vars() {
                if self.possibly_modified_variables.contains(&(var, Some(id))) {
                    // Ignore RHS, mark this variable as possibly modified.
                    new_binding.push((var, None));
                } else {
                    new_binding.push((var, Some(SimpleValue::Uninitialized)))
                }
            }
        }
        // Newly bound vars block any prior values
        self.rewrite_enter_scope(id, pat.vars().iter());
        // Add bindings to the scoped value map.
        for (var, opt_value) in new_binding.into_iter() {
            // Note that binding was already rewritten (but outside this scope).
            if let Some(value) = opt_value {
                self.values.insert(var, value);
            } else {
                self.values.remove(var)
            }
        }
        // Rename local variables in the pattern.
        None
    }

    /// Note that `rewrite_block` is called *after* `rewrite_exit_scope`.
    fn rewrite_block(
        &mut self,
        id: NodeId,
        pat: &Pattern,
        opt_binding: &Option<Exp>,
        body: &Exp,
    ) -> Option<Exp> {
        if let Some(exp) = opt_binding {
            let pat_id = pat.node_id();
            let exp_id = exp.node_id();
            let pat_type = self.env().get_node_type(pat_id);
            let exp_type = self.env().get_node_type(exp_id);
            let type_display_context = TypeDisplayContext::new(self.env());
            trace!(
                "Starting rewrite_block(id={}, pat={}, opt_binding={}, body={}, pat_type={}, exp_type={}, {})",
                id.as_usize(),
                pat.to_string(self.func_env),
                exp.display_verbose(self.env()),
                body.display_verbose(self.env()),
                pat_type.display(&type_display_context),
                exp_type.display(&type_display_context),
                if pat_type == exp_type { "MATCHES" } else { "NO MATCH" },
            );
        } else {
            trace!(
                "Starting rewrite_block(id={}, pat={}, opt_binding={}, body={})",
                id.as_usize(),
                pat.to_string(self.func_env),
                "None",
                body.display_verbose(self.env())
            );
        }

        // Simplify binding:
        //   A few ideas for simplification which are implemented below:
        //     (1) if no binding, then simplify to just the body.
        //     (2) if all pattern vars are unused in body and binding is OK to remove, again return body.
        //     (3) if some pattern vars are unused in the body, turn them into wildcards.

        let pat_vars = pat.vars();
        // (1) if no binding, then simplify to just the body
        if opt_binding.is_none() && pat_vars.is_empty() {
            trace!(
                "No binding, dropping all but body for rewrite_block(id={})",
                id.as_usize()
            );
            return Some(body.clone());
        }
        let bound_vars = pat.vars();

        // (2) If all pattern vars are unused in body and binding is OK to remove
        // (is side-effect free and has no potential impact on Move semantics) , again return
        // body.  But to avoid introducing a drop of a struct value that might not have "drop",
        // also check that opt_binding type has drop.
        let free_vars = body.free_vars();
        let unused_bound_vars: BTreeSet<_> = bound_vars
            .iter()
            .filter_map(|(id, sym)| {
                let ty = self.env().get_node_type(*id);
                if !free_vars.contains(sym) {
                    trace!(
                        "Sym {} is not in free_vars",
                        sym.display(self.env().symbol_pool())
                    );
                    if matches!(ty, Type::Tuple(_)) {
                        // Tuple type for variable is not valid, but won't be flagged until bytecode
                        // generation; leave it in place so diagnostic can be generated.
                        None
                    } else {
                        Some(sym)
                    }
                } else {
                    None
                }
            })
            .cloned()
            .collect();
        let binding_can_be_dropped = pat.has_no_struct()
            && if let Some(binding) = opt_binding {
                let node_id = binding.node_id();
                let opt_type = self.env().get_node_type_opt(node_id);
                if let Some(ty) = opt_type {
                    let ability_set = self
                        .env()
                        .type_abilities(&ty, self.func_env.get_type_parameters_ref());
                    // Don't drop a function-valued expression so we don't lose errors.
                    !ty.has_function() && ability_set.has_ability(Ability::Drop)
                } else {
                    // We're missing type info, be conservative
                    false
                }
            } else {
                true
            };
        let can_eliminate_bindings = binding_can_be_dropped
            && bound_vars.len() == unused_bound_vars.len()
            && if let Some(binding) = opt_binding {
                binding.is_ok_to_remove_from_code()
            } else {
                true
            };
        if can_eliminate_bindings {
            trace!(
                "No used vars, dropping all but body for rewrite_block(id={})",
                id.as_usize()
            );
            return Some(body.clone());
        }

        // The following is disabled for now until we figure out whether there is a fix
        // for #12475.  If that is fixed, then we can safely rewrite unused variable
        // definitions to wildcards.
        //
        // // (3) If some pattern vars are unused in the body, turn them into wildcards.
        // let new_pat = if !unused_bound_vars.is_empty() {
        //     Some(pat.clone().remove_vars(&unused_bound_vars))
        // } else {
        //     None
        // };

        // // Ideas not yet implemented:
        // //     (4) simplify the pattern: if subpat is wildcard and subexpr is side-effect-free,
        // //         can remove it and corresponding subexpr.
        // //     (5) simplify the pattern: if subpat is wildcard, corresponding subexpr can be
        // //         simplified to not produce a value
        // //     (6) if body is also a block and its binding has no references to our bound vars,
        // //         then merge patterns and blocks
        // //     (7) if pattern is a singleton `Tuple` and binding is a `Tuple`, turn it into let x = val.

        // if let Some(pat) = new_pat {
        //     let exp = ExpData::Block(id, pat, opt_binding.clone(), body.clone()).into_exp();
        //     trace!(
        //         "Dropping some vars  for rewrite_block(id={}), result = {}",
        //         id.as_usize(),
        //         exp.display_verbose(self.env()),
        //     );
        //     Some(exp)
        // } else {
        //     None
        // }

        None
    }

    fn rewrite_if_else(&mut self, _id: NodeId, cond: &Exp, then: &Exp, else_: &Exp) -> Option<Exp> {
        if self.eliminate_code {
            if let Some((truth_value, branch_name, result, eliminated_id)) = match cond.as_ref() {
                ExpData::Value(_, Value::Bool(true)) => {
                    if else_.as_ref().is_ok_to_remove_from_code() {
                        Some((true, "else", then.clone(), else_.node_id()))
                    } else {
                        None
                    }
                },
                ExpData::Value(_, Value::Bool(false)) => {
                    if then.as_ref().is_ok_to_remove_from_code() {
                        Some((false, "then", else_.clone(), then.node_id()))
                    } else {
                        None
                    }
                },
                _ => None,
            } {
                let loc = self.env().get_node_loc(eliminated_id);
                let cond_loc = self.env().get_node_loc(cond.node_id());
                self.env().diag_with_labels(
                    Severity::Warning,
                    &cond_loc,
                    &format!(
                        "If condition is always {}, so {} branch code eliminated as dead code",
                        truth_value, branch_name,
                    ),
                    vec![
                        (
                            cond_loc.clone(),
                            format!("condition is always {}", truth_value).to_string(),
                        ),
                        (
                            loc,
                            format!("{} branch eliminated", branch_name).to_string(),
                        ),
                    ],
                );
                Some(result)
            } else {
                None
            }
        } else {
            None
        }
    }

    fn rewrite_sequence(&mut self, id: NodeId, seq: &[Exp]) -> Option<Exp> {
        if self.eliminate_code && seq.len() > 1 {
            // Check which elements are side-effect-free
            let mut siter = seq.iter();
            let last_expr_opt = siter.next_back(); // first remove last element from siter
            let side_effecting_elts_refs = siter
                .filter(|exp|
                        if exp.as_ref().is_ok_to_remove_from_code() {
                            let loc = self.env().get_node_loc(exp.node_id());
                            self.env().diag(
                                Severity::Warning,
                                &loc,
                                "Expression value unused and side-effect free, so eliminated as dead code"
                            );
                            false
                        } else {
                            true
                        })
                .collect_vec();
            if side_effecting_elts_refs.len() + 1 < seq.len() {
                // We can remove some exprs; clone just the others.
                let new_vec = side_effecting_elts_refs
                    .into_iter()
                    .chain(last_expr_opt)
                    .cloned()
                    .collect_vec();
                if new_vec.len() == 1 {
                    // Unwrap a lone sequence item.
                    new_vec.first().cloned()
                } else {
                    Some(ExpData::Sequence(id, new_vec).into_exp())
                }
            } else {
                None
            }
        } else if seq.len() == 1 {
            // Unwrap a lone sequence item.
            seq.first().cloned()
        } else {
            None
        }
    }
}

#[test]
fn test_scoped_map() {
    let mut testmaps = Vec::new();
    let k = 6;

    // Initialize a set of maps to write to the scoped map.
    for i in 0..k {
        let mut testmap: BTreeMap<usize, usize> = BTreeMap::new();
        for j in 0..(k * 5) {
            if (j % (i + 2)) != 0 {
                testmap.insert(j, j + i);
            }
        }
        testmaps.push(testmap);
    }

    let mut smap: ScopedMap<usize, usize> = ScopedMap::new();

    // Scope 0
    for (key, value) in &testmaps[0] {
        smap.insert(*key, *value);
    }
    // check what we wrote to the smap
    for j in 0..(k * 5) {
        if (j % 2) != 0 {
            let i = 0;
            assert!(smap.get(&j) == Some(&(j + i)));
        } else {
            assert!(smap.get(&j).is_none());
        }
    }

    // Entering scope 1 .. k-1
    for i in 1..k {
        smap.enter_scope();

        let lastmap = &testmaps[i - 1];
        let testmap = &testmaps[i];
        for key in lastmap.keys() {
            if !testmap.contains_key(key) {
                smap.remove(*key)
            }
        }
        for (key, value) in testmap {
            smap.insert(*key, *value);
        }

        // check that our inserts and removes yielded what we thought
        for j in 0..(k * 5) {
            if (j % (i + 2)) != 0 {
                assert!(smap.get(&j) == Some(&(j + i)));
            } else {
                assert!(smap.get(&j).is_none());
            }
        }
    }

    // Exiting scopes k-1. .. 1
    for i in (1..k).rev() {
        // check that the scope at each level is what we had before
        for j in 0..(k * 5) {
            if (j % (i + 2)) != 0 {
                assert!(smap.get(&j) == Some(&(j + i)));
            } else {
                assert!(smap.get(&j).is_none());
            }
        }
        smap.exit_scope();
    }
    // scope 0
    for j in 0..(k * 5) {
        if (j % 2) != 0 {
            let i = 0;
            assert!(smap.get(&j) == Some(&(j + i)));
        } else {
            assert!(smap.get(&j).is_none());
        }
    }
}
