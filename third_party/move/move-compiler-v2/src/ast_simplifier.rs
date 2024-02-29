// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

/// AST Simplifier
///
/// Simplify the AST before conversion to bytecode
use codespan_reporting::diagnostic::Severity;
use itertools::Itertools;
use log::{debug, log_enabled, trace, Level};
use move_binary_format::file_format::Ability;
use move_model::{
    ast::{Exp, ExpData, Operation, Pattern, Value, VisitorPosition},
    constant_folder::ConstantFolder,
    exp_rewriter::ExpRewriterFunctions,
    model::{GlobalEnv, NodeId, Parameter, TypeParameter},
    symbol::Symbol,
    ty::{ReferenceKind, Type, TypeDisplayContext},
};
use std::{
    collections::{BTreeMap, BTreeSet},
    fmt::Debug,
    iter::{zip, IntoIterator, Iterator},
    vec::Vec,
};

static DISABLE_IFELSE_SIMPLIFICATION: bool = true;

pub fn run_simplifier(env: &mut GlobalEnv, eliminate_code: bool) {
    let mut rewriter = SimplifierRewriter::new(env, eliminate_code);
    let mut new_definitions = Vec::new(); // Avoid borrowing issues for env.
    for module in env.get_modules() {
        if module.is_target() {
            for func in module.get_functions() {
                if let Some(def) = func.get_def() {
                    let params = &func.get_parameters();
                    let type_params = &func.get_type_parameters();
                    let rewritten =
                        rewriter.rewrite_function_body(params, type_params, def.clone());
                    trace!(
                        "After rewrite_function_body, function body is `{}`",
                        rewritten.display(env)
                    );

                    if !ExpData::ptr_eq(&rewritten, def) {
                        new_definitions.push((func.get_qualified_id(), rewritten));
                    }
                }
            }
        }
    }
    // Actually do the writing of new definitions.
    for (qfid, def) in new_definitions.into_iter() {
        env.set_function_def(qfid, def);
        debug!(
            "After simplifier, function is `{}`",
            env.dump_fun(&env.get_function(qfid))
        );
    }
}

#[derive(Debug)]
struct ScopedMap<K, V> {
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

// Finds sets of local vars that may be modified, and shouldn't be treated as constant.
// Vars are identified by symbol name and by the scope in which they are defined.
// Scope is either
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
    let mut visiting_binding: ScopedMap<Symbol, NodeId> = ScopedMap::new();
    let mut unsafe_variables: BTreeSet<(Symbol, Option<NodeId>)> = BTreeSet::new();

    // Track when we are in a modifying scope.
    let mut modifying = false;
    // Use a stack to keep the modification status properly scoped.
    let mut modifying_stack = Vec::new();
    exp.visit_positions(&mut |pos, e| {
        use ExpData::*;
        match e {
            Invalid(_) | Value(..) | LoopCont(..) | SpecBlock(..) => {},
            LocalVar(id, sym) => {
                let current_binding = visiting_binding.get(sym);
                if modifying {
                    trace!(
                        "Var {} in binding {:?} used in node {} is unsafe",
                        sym.display(env.symbol_pool()),
                        current_binding,
                        id.as_usize(),
                    );
                    unsafe_variables.insert((*sym, current_binding.copied()));
                } else {
                    trace!(
                        "Var {} in binding {:?} used in node {} is ok",
                        sym.display(env.symbol_pool()),
                        current_binding,
                        id.as_usize(),
                    );
                }
            },
            Temporary(id, idx) => {
                if let Some(sym) = params.get(*idx).map(|p| p.0) {
                    if modifying {
                        let current_binding = visiting_binding.get(&sym);
                        trace!(
                            "Temp {} = Var {} in binding {:?} is unsafe",
                            *idx,
                            sym.display(env.symbol_pool()),
                            current_binding
                        );
                        assert!(current_binding.is_none());
                        unsafe_variables.insert((sym, None));
                    };
                } else {
                    let loc = env.get_node_loc(*id);
                    env.diag(
                        Severity::Bug,
                        &loc,
                        &format!("Use of temporary with no corresponding parameter `{}`", idx),
                    );
                }
            },
            Call(_, op, _explist) => {
                match op {
                    // NOTE: we don't consider values in globals, so no need to
                    // consider BorrowGlobal(ReferenceKind::Mutable)} here.
                    Operation::MoveTo
                    | Operation::MoveFrom
                    | Operation::Move
                    | Operation::Borrow(ReferenceKind::Mutable) => {
                        match pos {
                            VisitorPosition::Pre => {
                                // Add all mentioned vars to modified vars.
                                modifying_stack.push(modifying);
                                modifying = true;
                                trace!("Entering Move/Borrow");
                            },
                            VisitorPosition::Post => {
                                // stop adding vars
                                modifying = modifying_stack.pop().expect("unbalanced visit 1");
                                trace!("Exiting Move/Borrow");
                            },
                            _ => {},
                        }
                    },
                    Operation::MoveFunction(module_id, fun_id) => {
                        let qfid = module_id.qualified(*fun_id);
                        let func_env = env.get_function(qfid);
                        if func_env.is_inline() {
                            // Inline function may modify parameters.
                            match pos {
                                VisitorPosition::Pre => {
                                    // Add all mentioned vars to modified vars.
                                    modifying_stack.push(modifying);
                                    modifying = true;
                                },
                                VisitorPosition::Post => {
                                    // stop adding vars
                                    modifying = modifying_stack.pop().expect("unbalanaced visit 2");
                                },
                                _ => {},
                            }
                        } else {
                            // Function calls other than inline ones cannot modify parameter var.
                            match pos {
                                VisitorPosition::Pre => {
                                    modifying_stack.push(modifying);
                                    modifying = false;
                                },
                                VisitorPosition::Post => {
                                    modifying = modifying_stack.pop().expect("unbalanced visit 3");
                                },
                                _ => {},
                            }
                        }
                    },
                    _ => {
                        // Other operations don't modify argument variables.
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
            Invoke(..) | Return(..) | Quant(..) | Loop(..) | Mutate(..) => {
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
            Lambda(node_id, pat, _) => {
                // Define a new scope for bound vars
                match pos {
                    VisitorPosition::Pre => {
                        trace!("Entering lambda {}", node_id.as_usize());
                        visiting_binding.enter_scope();
                        for (_, sym) in pat.vars() {
                            visiting_binding.insert(sym, *node_id);
                        }
                    },
                    VisitorPosition::Post => {
                        // remove a scope
                        visiting_binding.exit_scope();
                        trace!("Exiting lambda {}", node_id.as_usize());
                    },
                    _ => {},
                };
            },
            Block(node_id, pat, _, _) => {
                // Define a new scope for bound vars
                match pos {
                    VisitorPosition::Pre => {
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
                    },
                    _ => {},
                };
            },
            IfElse(_, _cond, _then, _else) => {
                match pos {
                    VisitorPosition::Pre => {
                        modifying_stack.push(modifying);
                        modifying = false;
                    },
                    VisitorPosition::BeforeThen => {
                        modifying = modifying_stack.pop().expect("unbalanced visit 6");
                    },
                    _ => {},
                };
            },
            Sequence(_, _exp_vec) => match pos {
                VisitorPosition::Pre => {
                    modifying_stack.push(modifying);
                    modifying = false;
                },
                VisitorPosition::PreSequenceValue => {
                    modifying = modifying_stack.pop().expect("unbalanced visit 6");
                },
                _ => {},
            },
            Assign(_loc, pat, _) => {
                match pos {
                    VisitorPosition::Pre => {
                        // add vars in pat to modified vars
                        for (_pat_var_id, sym) in pat.vars() {
                            let current_binding = visiting_binding.get(&sym);
                            trace!(
                                "Var {} in assignment {:?} is unsafe",
                                sym.display(env.symbol_pool()),
                                current_binding
                            );
                            unsafe_variables.insert((sym, current_binding.copied()));
                        }
                    },
                    _ => {},
                };
            },
        };
        true
    });
    unsafe_variables
}

struct SimplifierRewriter<'env> {
    pub env: &'env GlobalEnv,
    pub constant_folder: ConstantFolder<'env>,
    pub eliminate_code: bool,

    // Parameters to currently processed function
    cached_params: Vec<Parameter>,

    // Type Parameters to currently processed function
    cached_type_params: Vec<TypeParameter>,

    // Tracks which definition (`Let` statement `NodeId`) is visible during visit to find modified
    // local vars.  A use of a symbol which is missing must be a `Parameter`.  This is used only
    // to determine if a symbol is in `unsafe_variables`.
    visiting_binding: ScopedMap<Symbol, NodeId>,

    // Unsafe variables are identified by `Symbol` and `Let` statement `NodeId`,
    // except function parameters, which have no `NodeId` so get `None`.
    unsafe_variables: BTreeSet<(Symbol, Option<NodeId>)>,

    // Tracks constant values from scope.
    values: ScopedMap<Symbol, SimpleValue>,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
enum SimpleValue {
    Value(Value),
    Uninitialized,
}

impl<'env> SimplifierRewriter<'env> {
    fn new(env: &'env GlobalEnv, eliminate_code: bool) -> Self {
        let constant_folder = ConstantFolder::new(env, false);
        Self {
            env,
            constant_folder,
            eliminate_code,
            cached_params: Vec::new(),
            cached_type_params: Vec::new(),
            visiting_binding: ScopedMap::new(),
            unsafe_variables: BTreeSet::new(),
            values: ScopedMap::new(),
        }
    }

    /// Process a function.
    pub fn rewrite_function_body(
        &mut self,
        params: &[Parameter],
        type_params: &[TypeParameter],
        exp: Exp,
    ) -> Exp {
        self.cached_params = params.to_vec();
        self.cached_type_params = type_params.to_vec();
        self.unsafe_variables = find_possibly_modified_vars(self.env, params, exp.as_ref());
        self.visiting_binding.clear();
        self.values.clear();
        if log_enabled!(Level::Debug) {
            debug!(
                "Unsafe variables are ({:#?})",
                self.unsafe_variables
                    .iter()
                    .map(|(sym, opt_node)| format!(
                        "{}@{}",
                        sym.display(self.env.symbol_pool()),
                        if let Some(node) = opt_node {
                            node.as_usize().to_string()
                        } else {
                            "None".to_string()
                        }
                    ))
                    .join(", ")
            )
        }
        // Enter Function scope (a specialized `rewrite_enter_scope()` call)
        self.values.enter_scope();
        for param in self.cached_params.iter() {
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
                    let loc = self.env.get_node_loc(id);
                    self.env.diag(
                        Severity::Error,
                        &loc,
                        &format!(
                            "use of unassigned local `{}`",
                            sym.display(self.env.symbol_pool())
                        ),
                    );
                    None
                },
            }
        } else {
            trace!(
                "Found no value for var {} ",
                sym.display(self.env.symbol_pool()),
            );
            None
        }
    }

    // Note that exp has already been rewritten.
    fn expr_to_simple_value(&mut self, exp: Option<Exp>) -> Option<SimpleValue> {
        if let Some(exp) = exp {
            match exp.as_ref() {
                ExpData::Value(_, val) => Some(SimpleValue::Value(val.clone())),
                ExpData::LocalVar(_id, sym) => self.values.get(sym).cloned(),
                ExpData::Temporary(id, idx) => {
                    if let Some(sym) = self.cached_params.get(*idx).map(|p| &p.0) {
                        self.values.get(sym).cloned()
                    } else {
                        panic!(
                            "Invalid index {} for Temporary at node {}",
                            *idx,
                            id.as_usize()
                        )
                    }
                },
                _ => None,
            }
        } else {
            None
        }
    }

    // Expand a `Value::Tuple` value expression to a call to `Tuple`
    // Note that a `Value::Vector` value is left alone.
    fn expand_tuple(&mut self, exp: Exp) -> Exp {
        if let ExpData::Value(id, Value::Tuple(x)) = exp.as_ref() {
            if x.is_empty() {
                ExpData::Call(*id, Operation::Tuple, Vec::new()).into_exp()
            } else {
                let loc = self.env.get_node_loc(*id);
                self.env.diag(
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

    // Try to turn a call to cast(x:T1,:T1) -> x
    fn try_collapse_cast(&mut self, id: NodeId, arg0: &Exp) -> Option<Exp> {
        let arg0_type = self.env.get_node_type(arg0.node_id());
        let result_type = self.env.get_node_type(id);
        if arg0_type == result_type {
            Some(arg0.clone())
        } else {
            None
        }
    }
}

impl<'env> ExpRewriterFunctions for SimplifierRewriter<'env> {
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
        let result = self.rewrite_to_recorded_value(id, &sym);
        if log_enabled!(Level::Trace) {
            if let Some(exp) = &result {
                let in_scope = self.visiting_binding.get(&sym);
                let value = self.values.get(&sym);
                trace!(
                    "Replacing symbol {} at node {} with {}; in_scope={:?}, value={:?}",
                    sym.display(self.env.symbol_pool()),
                    id.as_usize(),
                    exp.display(self.env),
                    in_scope.map(|n| n.as_usize()),
                    value
                );
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

    fn rewrite_if_else(&mut self, _id: NodeId, cond: &Exp, then: &Exp, else_: &Exp) -> Option<Exp> {
        if DISABLE_IFELSE_SIMPLIFICATION {
            None
        } else {
            match cond.as_ref() {
                ExpData::Value(_, Value::Bool(true)) => Some(then.clone()),
                ExpData::Value(_, Value::Bool(false)) => Some(else_.clone()),
                _ => None,
            }
        }
    }

    fn rewrite_sequence(&mut self, id: NodeId, seq: &[Exp]) -> Option<Exp> {
        if self.eliminate_code {
            // Check which elements are side-effect-free
            let mut siter = seq.iter();
            let _ignore = siter.next_back(); // ignore last element, we have to keep it
            let mut side_effect_free_elts: Vec<_> = siter
                .map(|exp| exp.as_ref().is_side_effect_free())
                .collect();
            if side_effect_free_elts.iter().all(|elt| *elt) {
                // All elements other than the last are side-effect free.
                // (Note that this case includes a singleton sequence.)
                seq.iter().next_back().cloned()
            } else if side_effect_free_elts
                .iter()
                .any(|elt_is_side_effect_free| *elt_is_side_effect_free)
            {
                // At least one element can be removed.
                side_effect_free_elts.push(false);
                let new_vec: Vec<_> = zip(seq, side_effect_free_elts)
                    .filter_map(|(elt, is_side_effect_free)| {
                        if !is_side_effect_free {
                            Some(elt.clone())
                        } else {
                            None
                        }
                    })
                    .collect();
                Some(ExpData::Sequence(id, new_vec).into_exp())
            } else {
                None
            }
        } else {
            None
        }
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
                if self.unsafe_variables.contains(&(var, Some(id))) {
                    // Ignore RHS, mark this variable as unsafe.
                    new_binding.push((var, None));
                } else {
                    // Try to evaluate opt_new_binding_exp as a constant/var.
                    // If unrepresentable as a Value, returns None.
                    new_binding.push((var, self.expr_to_simple_value(opt_new_binding_exp)));
                }
            }
        } else {
            // Body with no bindings, values are Uninitialized.
            for (_, var) in pat.vars() {
                if self.unsafe_variables.contains(&(var, Some(id))) {
                    // Ignore RHS, mark this variable as unsafe.
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

    // Note that `rewrite_block` is called *after* `rewrite_exit_scope`.
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
            let pat_type = self.env.get_node_type(pat_id);
            let exp_type = self.env.get_node_type(exp_id);
            let type_display_context = TypeDisplayContext::new(self.env);
            trace!(
                "Starting rewrite_block(id={}, pat={}, opt_binding={}, body={}, pat_type={}, exp_type={}, {})",
                id.as_usize(),
                pat.to_string(self.env, &type_display_context),
                exp.display_verbose(self.env),
                body.display_verbose(self.env),
                pat_type.display(&type_display_context),
                exp_type.display(&type_display_context),
                if pat_type == exp_type { "MATCHES" } else { "NO MATCH" },
            );
        } else {
            trace!(
                "Starting rewrite_block(id={}, pat={}, opt_binding={}, body={})",
                id.as_usize(),
                pat.to_string(self.env, &TypeDisplayContext::new(self.env)),
                "None",
                body.display_verbose(self.env)
            );
        }

        // Simplify binding:
        //   A few ideas for simplification which are implemented below:
        //     (1) if no binding, then simplify to just the body.
        //     (2) if all pattern vars are unused in body and binding is side-effect free, again return body.
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

        // (2) If all pattern vars are unused in body and binding is side-effect free, again return
        // body.  But to avoid introducing a drop of a struct value that might not have "drop",
        // also check that opt_binding type has drop.
        let free_vars = body.free_vars();
        let unused_bound_vars: BTreeSet<_> = bound_vars
            .iter()
            .filter_map(|(id, sym)| {
                let ty = self.env.get_node_type(*id);
                if !free_vars.contains(sym) {
                    trace!(
                        "Sym {} is not in free_vars",
                        sym.display(self.env.symbol_pool())
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
                let opt_type = self.env.get_node_type_opt(node_id);
                if let Some(ty) = opt_type {
                    let ability_set = self.env.type_abilities(&ty, &self.cached_type_params);
                    ability_set.has_ability(Ability::Drop)
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
                binding.is_side_effect_free()
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

        // (3) If some pattern vars are unused in the body, turn them into wildcards.
        let new_pat = if !unused_bound_vars.is_empty() {
            Some(pat.clone().remove_vars(&unused_bound_vars))
        } else {
            None
        };

        // Ideas not yet implemented:
        //     (4) simplify the pattern: if subpat is wildcard and subexpr is side-effect-free,
        //         can remove it and corresponding subexpr.
        //     (5) simplify the pattern: if subpat is wildcard, corresponding subexpr can be
        //         simplified to not produce a value
        //     (6) if body is also a block and its binding has no references to our bound vars,
        //         then merge patterns and blocks
        //     (7) if pattern is a singleton `Tuple` and binding is a `Tuple`, turn it into let x = val.

        if let Some(pat) = new_pat {
            let exp = ExpData::Block(id, pat, opt_binding.clone(), body.clone()).into_exp();
            trace!(
                "Dropping some vars  for rewrite_block(id={}), result = {}",
                id.as_usize(),
                exp.display_verbose(self.env),
            );
            Some(exp)
        } else {
            None
        }
    }

    fn rewrite_exp(&mut self, exp: Exp) -> Exp {
        let old_id = exp.as_ref().node_id().as_usize();
        trace!(
            "Before rewrite, expr {} is `{}`",
            old_id,
            exp.display_verbose(self.env)
        );
        let r = self.rewrite_exp_descent(exp);
        let new_id = r.as_ref().node_id().as_usize();
        trace!(
            "After rewrite, expr {} is now {}: `{}`",
            old_id,
            new_id,
            r.display_verbose(self.env)
        );
        r
    }
}
