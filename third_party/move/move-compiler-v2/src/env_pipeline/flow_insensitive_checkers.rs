// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

//! Flow-insensitive checks can be done on the AST.
//!
//! Warnings about Unused parameter and local variable
//!   "Unused assignment or binding for local 's'. Consider removing, replacing with '_' or prefixing with '_' (e.g., '_r_ref')

use codespan_reporting::diagnostic::Severity;
use move_model::{
    ast::{ExpData, TempIndex, VisitorPosition},
    model::{GlobalEnv, Loc, NodeId, Parameter},
    symbol::Symbol,
    well_known,
};
use std::{collections::BTreeSet, fmt, fmt::Formatter, iter::Iterator};

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
    exp.visit_positions(&mut |position, exp_data| visitor.entry(position, exp_data));
    visitor.check_parameter_usage();
}

#[derive(Clone, Copy, PartialEq)]
enum UsageKind {
    Parameter,
    RangeParameter,
    LocalVar,
    Lambda,
}

impl fmt::Display for UsageKind {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            UsageKind::Parameter => "parameter",
            UsageKind::RangeParameter => "range parameter",
            UsageKind::LocalVar => "local variable",
            UsageKind::Lambda => "anonymous function parameter",
        })
    }
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

    fn entry(&mut self, position: VisitorPosition, e: &ExpData) -> bool {
        use ExpData::*;
        use VisitorPosition::*;
        match e {
            Block(_, pat, _, _) => {
                match position {
                    BeforeBody => self.seen_uses.enter_scope(),
                    Post => {
                        for (id, var) in pat.vars() {
                            self.node_symbol_decl_visitor(true, &id, &var, UsageKind::LocalVar)
                        }
                        self.seen_uses.exit_scope();
                    },
                    Pre | MidMutate | BeforeThen | BeforeElse | PreSequenceValue
                    | BeforeMatchBody(_) | AfterMatchBody(_) => {},
                };
            },
            Match(_, _, arms) => match position {
                BeforeMatchBody(_) => self.seen_uses.enter_scope(),
                AfterMatchBody(idx) => {
                    for (id, var) in arms[idx].pattern.vars() {
                        self.node_symbol_decl_visitor(true, &id, &var, UsageKind::LocalVar)
                    }
                    self.seen_uses.exit_scope();
                },
                Pre | Post | BeforeBody | MidMutate | BeforeThen | BeforeElse
                | PreSequenceValue => {},
            },
            Lambda(_, pat, _, _, _) => {
                match position {
                    Pre => self.seen_uses.enter_scope(),
                    Post => {
                        for (id, var) in pat.vars() {
                            self.node_symbol_decl_visitor(true, &id, &var, UsageKind::Lambda);
                        }
                        self.seen_uses.exit_scope();
                    },
                    BeforeBody | MidMutate | BeforeThen | BeforeElse | PreSequenceValue
                    | BeforeMatchBody(_) | AfterMatchBody(_) => {},
                };
            },
            Quant(_, _, ranges, ..) => {
                match position {
                    Pre => self.seen_uses.enter_scope(),
                    Post => {
                        for (id, var) in ranges.iter().flat_map(|(pat, _)| pat.vars().into_iter()) {
                            self.node_symbol_decl_visitor(
                                true,
                                &id,
                                &var,
                                UsageKind::RangeParameter,
                            );
                        }
                        self.seen_uses.exit_scope();
                    },
                    BeforeBody | MidMutate | BeforeThen | BeforeElse | PreSequenceValue
                    | BeforeMatchBody(_) | AfterMatchBody(_) => {},
                };
            },
            Assign(_, pat, _) => {
                if let Post = position {
                    for (id, sym) in pat.vars().iter() {
                        self.node_symbol_use_visitor(true, id, sym);
                    }
                }
            },
            LocalVar(id, sym) => {
                if let Post = position {
                    self.node_symbol_use_visitor(true, id, sym);
                }
            },
            Temporary(id, idx) => {
                if let Post = position {
                    self.node_tmp_use_visitor(true, id, idx);
                }
            },
            _ => {},
        }
        true // always continue
    }

    fn check_symbol_usage(&mut self, loc: &Loc, sym: &Symbol, kind: UsageKind) {
        let symbol_pool = self.env.symbol_pool();
        let receiver_param_name = symbol_pool.make(well_known::RECEIVER_PARAM_NAME);
        if !symbol_pool.symbol_starts_with_underscore(*sym)
            && !self.seen_uses.contains(sym)
            // The `self` parameter is exempted from the check
            && (sym != &receiver_param_name || kind != UsageKind::Parameter)
        {
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
            self.check_symbol_usage(loc, sym, UsageKind::Parameter);
        }
    }

    fn node_symbol_decl_visitor(&mut self, post: bool, id: &NodeId, sym: &Symbol, kind: UsageKind) {
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
