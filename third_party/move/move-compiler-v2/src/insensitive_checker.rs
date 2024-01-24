// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

//! Context-insensitive and Flow-insensitive checks can be done on the AST.
//!
//! Warnings about Unused parameter and local variable
//!   "Unused assignment or binding for local 's'. Consider removing, replacing with '_' or prefixing with '_' (e.g., '_r_ref')

use codespan_reporting::diagnostic::Severity;
use move_model::{
    ast::{ExpData, TempIndex},
    model::{GlobalEnv, NodeId, Parameter},
    symbol::Symbol,
};
use std::{collections::BTreeSet, iter::Iterator};

/// Warns about all parameters and local variables that are unused.
pub fn run_unused_vars(env: &mut GlobalEnv) {
    for module in env.get_modules() {
        for func in module.get_functions() {
            if let Some(def) = &*func.get_def() {
                let params = &func.get_parameters();
                find_unused_params_and_vars(env, params, def)
            }
        }
    }
}

fn find_unused_params_and_vars(env: &GlobalEnv, params: &[Parameter], exp: &ExpData) {
    let mut visitor = SymbolVisitor::new(env, params);
    exp.visit_pre_post(&mut |post, exp_data| visitor.entry(post, exp_data));
    visitor.check_parameter_usage();
}

struct SymbolVisitor<'env, 'params> {
    env: &'env GlobalEnv,
    params: &'params [Parameter],
    seen_uses_stack: Vec<BTreeSet<Symbol>>,
    seen_uses: BTreeSet<Symbol>,
    used_tmps: BTreeSet<TempIndex>,
}

impl<'env, 'params> SymbolVisitor<'env, 'params> {
    fn new(env: &'env GlobalEnv, params: &'params [Parameter]) -> SymbolVisitor<'env, 'params> {
        SymbolVisitor {
            env,
            params,
            seen_uses_stack: Vec::new(),
            seen_uses: BTreeSet::new(),
            used_tmps: BTreeSet::new(),
        }
    }

    fn entry(&mut self, post: bool, e: &ExpData) -> bool {
        use ExpData::*;
        match e {
            Lambda(_, pat, _) | Block(_, pat, _, _) => {
                if !post {
                    self.enter_scope();
                } else {
                    // postorder
                    for (id, var) in pat.vars() {
                        self.node_symbol_decl_visitor(post, &id, &var);
                    }
                    self.exit_scope();
                }
            },
            Quant(_, _, ranges, ..) => {
                if !post {
                    self.enter_scope();
                } else {
                    // postorder
                    for (id, var) in ranges.iter().flat_map(|(pat, _)| pat.vars().into_iter()) {
                        self.node_symbol_decl_visitor(post, &id, &var);
                    }
                    self.exit_scope();
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

    fn check_parameter_usage(&mut self) {
        let symbol_pool = self.env.symbol_pool();
        for (idx, Parameter(ref sym, _atype, ref loc)) in self.params.iter().enumerate() {
            if !symbol_pool.symbol_starts_with_underscore(*sym)
                && !self.used_tmps.contains(&idx)
                && !self.seen_uses.contains(sym)
            {
                let msg = format!("Unused parameter '{}'.  Consider removing or prefixing with an underscore: '_{}'",
                                  sym.display(symbol_pool),
                                  sym.display(symbol_pool));
                self.env.diag(Severity::Warning, loc, &msg);
            }
        }
    }

    fn enter_scope(&mut self) {
        self.seen_uses_stack
            .push(std::mem::take(&mut self.seen_uses));
        self.seen_uses = BTreeSet::new();
    }

    fn exit_scope(&mut self) {
        let mut new_seen_uses = self
            .seen_uses_stack
            .pop()
            .expect("exits should balance enters");
        self.seen_uses.append(&mut new_seen_uses);
    }

    fn node_symbol_decl_visitor(&mut self, post: bool, id: &NodeId, sym: &Symbol) {
        if post {
            let symbol_pool = self.env.symbol_pool();
            if !self.seen_uses.contains(sym) {
                if !symbol_pool.symbol_starts_with_underscore(*sym) {
                    let loc = self.env.get_node_loc(*id);
                    let msg = format!("Unused local variable '{}'.  Consider removing or prefixing with an underscore: '_{}'",
                                      sym.display(symbol_pool),
                                      sym.display(symbol_pool));
                    self.env.diag(Severity::Warning, &loc, &msg);
                }
            } else {
                self.seen_uses.remove(sym);
            }
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
                let msg = format!("Temporary {} has no associated user symbol", idx);
                self.env.diag(Severity::Bug, &loc, &msg);
            }
        }
    }
}
