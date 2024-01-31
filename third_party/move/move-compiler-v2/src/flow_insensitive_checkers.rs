// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

//! Flow-insensitive checks can be done on the AST.
//!
//! Warnings about Unused parameter and local variable
//!   "Unused assignment or binding for local 's'. Consider removing, replacing with '_' or prefixing with '_' (e.g., '_r_ref')

use codespan_reporting::diagnostic::Severity;
use move_model::{
    ast::{ExpData, TempIndex},
    model::{GlobalEnv, Loc, NodeId, Parameter},
    symbol::Symbol,
};
use std::{collections::BTreeSet, iter::Iterator};

/// Warns about all parameters and local variables that are unused.
pub fn check_for_unused_vars_and_params(env: &mut GlobalEnv) {
    for module in env.get_modules() {
        if module.is_target() {
            for func in module.get_functions() {
                if let Some(def) = func.get_def() {
                    let params = &func.get_parameters();
                    find_unused_params_and_vars(env, params, def)
                }
            }
        }
    }
}

fn find_unused_params_and_vars(env: &GlobalEnv, params: &[Parameter], exp: &ExpData) {
    let mut visitor = SymbolVisitor::new(env, params);
    exp.visit_pre_post(&mut |post, exp_data| visitor.entry(post, exp_data));
    visitor.check_parameter_usage();
}

/// Tracks things of type `V` which are visible from below in a tree, such as free/used variables,
/// etc.  `values` is cleared when entering a scope, but the old value saved so it can be re-added
/// once the scope is finished.
struct ScopedVisibleSet<V> {
    saved: Vec<BTreeSet<V>>,
    values: BTreeSet<V>,
}

impl<V> ScopedVisibleSet<V>
where
    V: Ord + Copy,
{
    pub fn new() -> Self {
        Self {
            saved: Vec::new(),
            values: BTreeSet::new(),
        }
    }

    /// Save and clear the current set.
    pub fn enter_scope(&mut self) {
        self.saved.push(std::mem::take(&mut self.values));
    }

    /// Combine the current values with that previously saved
    /// in a corresponding `enter_scope` call.
    pub fn exit_scope(&mut self) {
        let mut saved_values = self
            .saved
            .pop()
            .expect("exit_scope calls should balance enter_scope calls");
        self.values.append(&mut saved_values);
    }

    /// Add a value to the current values.
    pub fn insert(&mut self, value: V) {
        self.values.insert(value);
    }

    /// Remove a value from the current scope.
    pub fn remove(&mut self, value: &V) {
        self.values.remove(value);
    }

    pub fn contains(&self, value: &V) -> bool {
        self.values.contains(value)
    }
}

// Visits all symbols in a function.
struct SymbolVisitor<'env, 'params> {
    env: &'env GlobalEnv,
    params: &'params [Parameter],
    seen_uses: ScopedVisibleSet<Symbol>,
}

impl<'env, 'params> SymbolVisitor<'env, 'params> {
    fn new(env: &'env GlobalEnv, params: &'params [Parameter]) -> SymbolVisitor<'env, 'params> {
        SymbolVisitor {
            env,
            params,
            seen_uses: ScopedVisibleSet::new(),
        }
    }

    fn entry(&mut self, post: bool, e: &ExpData) -> bool {
        use ExpData::*;
        match e {
            Block(_, pat, _, _) => {
                if !post {
                    self.seen_uses.enter_scope();
                } else {
                    // postorder
                    for (id, var) in pat.vars() {
                        self.node_symbol_decl_visitor(post, &id, &var, "local variable");
                    }
                    self.seen_uses.exit_scope();
                }
            },
            Lambda(_, pat, _) => {
                if !post {
                    self.seen_uses.enter_scope();
                } else {
                    // postorder
                    for (id, var) in pat.vars() {
                        self.node_symbol_decl_visitor(post, &id, &var, "parameter");
                    }
                    self.seen_uses.exit_scope();
                }
            },
            Quant(_, _, ranges, ..) => {
                if !post {
                    self.seen_uses.enter_scope();
                } else {
                    // postorder
                    for (id, var) in ranges.iter().flat_map(|(pat, _)| pat.vars().into_iter()) {
                        self.node_symbol_decl_visitor(post, &id, &var, "range parameter");
                    }
                    self.seen_uses.exit_scope();
                }
            },
            Assign(_, pat, _) => {
                for (id, sym) in pat.vars().iter() {
                    self.node_symbol_use_visitor(post, id, sym);
                }
            },
            LocalVar(id, sym) => {
                self.node_symbol_use_visitor(post, id, sym);
            },
            Temporary(id, idx) => {
                self.node_tmp_use_visitor(post, id, idx);
            },
            _ => {},
        }
        true // always continue
    }

    fn check_symbol_usage(&mut self, loc: &Loc, sym: &Symbol, kind: &str) {
        let symbol_pool = self.env.symbol_pool();
        if !symbol_pool.symbol_starts_with_underscore(*sym) && !self.seen_uses.contains(sym) {
            let msg = format!(
                "Unused {} `{}`. Consider removing or prefixing with an underscore: `_{}`",
                kind,
                sym.display(symbol_pool),
                sym.display(symbol_pool)
            );
            self.env.diag(Severity::Warning, loc, &msg);
        }
    }

    fn check_parameter_usage(&mut self) {
        for Parameter(sym, _atype, loc) in self.params.iter() {
            self.check_symbol_usage(loc, sym, "parameter");
        }
    }

    fn node_symbol_decl_visitor(&mut self, post: bool, id: &NodeId, sym: &Symbol, kind: &str) {
        if post {
            let loc = self.env.get_node_loc(*id);
            self.check_symbol_usage(&loc, sym, kind);
            self.seen_uses.remove(sym);
        }
    }

    fn node_symbol_use_visitor(&mut self, post: bool, _id: &NodeId, sym: &Symbol) {
        if post {
            self.seen_uses.insert(*sym);
        }
    }

    fn node_tmp_use_visitor(&mut self, post: bool, id: &NodeId, idx: &TempIndex) {
        if post {
            if let Some(sym) = self.params.get(*idx).map(|p| p.0) {
                self.node_symbol_use_visitor(post, id, &sym)
            } else {
                let loc = self.env.get_node_loc(*id);
                let msg = format!("Temporary `{}` has no associated user symbol.", idx);
                self.env.diag(Severity::Bug, &loc, &msg);
            }
        }
    }
}
