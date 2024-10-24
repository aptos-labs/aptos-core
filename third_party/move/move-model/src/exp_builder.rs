// Copyright © Aptos Foundation
// Parts of the project are originally copyright © Meta Platforms, Inc.
// SPDX-License-Identifier: Apache-2.0

//! A helper for building expressions

use crate::{
    ast::{Exp, ExpData, Operation, Pattern, RewriteResult},
    model::{GlobalEnv, Loc, NodeId},
    symbol::Symbol,
    ty::Type,
};
use itertools::Itertools;
use std::collections::BTreeMap;

/// Represents an expression builder.
pub struct ExpBuilder<'a> {
    env: &'a GlobalEnv,
}

impl<'a> ExpBuilder<'a> {
    pub fn new(env: &'a GlobalEnv) -> Self {
        Self { env }
    }

    pub fn env(&self) -> &GlobalEnv {
        self.env
    }

    pub fn not(&self, exp: Exp) -> Exp {
        if let ExpData::Call(_, Operation::Not, args) = exp.as_ref() {
            args.last().expect("malformed expression").clone()
        } else {
            ExpData::Call(self.clone_node_id(exp.node_id()), Operation::Not, vec![exp]).into_exp()
        }
    }

    pub fn if_else(&self, cond: Exp, if_true: Exp, if_false: Exp) -> Exp {
        let loc = self.enclosing_loc([&cond, &if_true, &if_false].into_iter());
        let ty = self.env.get_node_type(if_true.node_id());
        let id = self.new_node_id(loc, ty);
        ExpData::IfElse(id, cond, if_true, if_false).into_exp()
    }

    pub fn if_(&self, cond: Exp, then: Exp) -> Exp {
        let loc = self.enclosing_loc([&cond, &then].into_iter());
        ExpData::IfElse(
            self.new_node_id(loc.clone(), Type::unit()),
            cond,
            then,
            self.nop(&loc),
        )
        .into_exp()
    }

    pub fn break_(&self, loc: &Loc, nest: usize) -> Exp {
        ExpData::LoopCont(self.new_node_id(loc.clone(), Type::unit()), nest, false).into_exp()
    }

    pub fn continue_(&self, loc: &Loc, nest: usize) -> Exp {
        ExpData::LoopCont(self.new_node_id(loc.clone(), Type::unit()), nest, true).into_exp()
    }

    pub fn seq(&self, nop_loc: &Loc, stms: Vec<Exp>) -> Exp {
        if stms.is_empty() {
            self.nop(nop_loc)
        } else {
            let ty = if let Some(last) = stms.last() {
                self.env.get_node_type(last.node_id())
            } else {
                Type::unit()
            };
            let loc = self.enclosing_loc(stms.iter());
            ExpData::Sequence(self.new_node_id(loc, ty), stms).into_exp()
        }
    }

    pub fn block(&self, pat: Pattern, def: Option<Exp>, body: Exp) -> Exp {
        let node_id = self.clone_node_id(body.node_id());
        ExpData::Block(node_id, pat, def, body).into_exp()
    }

    pub fn nop(&self, loc: &Loc) -> Exp {
        ExpData::Sequence(self.new_node_id(loc.clone(), Type::unit()), vec![]).into_exp()
    }

    pub fn unfold(&self, substitution: &BTreeMap<Symbol, Exp>, exp: Exp) -> Exp {
        ExpData::rewrite(exp, &mut |e: Exp| {
            if let ExpData::LocalVar(_, name) = e.as_ref() {
                if let Some(r) = substitution.get(name) {
                    return RewriteResult::Rewritten(ExpData::rewrite_node_id(
                        r.clone(),
                        &mut |id| Some(self.clone_node_id(id)),
                    ));
                }
            }
            RewriteResult::Unchanged(e)
        })
    }

    pub fn match_loop(&self, exp: Exp) -> Option<(NodeId, Exp)> {
        if let ExpData::Loop(id, body) = exp.as_ref() {
            Some((*id, body.clone()))
        } else {
            None
        }
    }

    pub fn extract_first(&self, exp: Exp) -> (Exp, Vec<Exp>) {
        match exp.as_ref() {
            ExpData::Sequence(_, exps) if !exps.is_empty() => {
                (exps[0].clone(), exps.iter().skip(1).cloned().collect())
            },
            _ => (exp, vec![]),
        }
    }

    pub fn extract_last(&self, exp: Exp) -> (Vec<Exp>, Exp) {
        match exp.as_ref() {
            ExpData::Sequence(_, exps) if !exps.is_empty() => {
                let n = exps.len();
                (
                    exps.iter().take(n - 1).cloned().collect(),
                    exps[n - 1].clone(),
                )
            },
            _ => (vec![], exp),
        }
    }

    pub fn match_if(&self, exp: Exp) -> Option<(Exp, Exp)> {
        match exp.as_ref() {
            ExpData::IfElse(_, cond, if_true, if_false) if if_false.is_unit_exp() => {
                Some((cond.clone(), if_true.clone()))
            },
            _ => None,
        }
    }

    pub fn match_if_loop_cont(&self, exp: Exp, nest: usize, is_continue: bool) -> Option<Exp> {
        let (cond, if_true) = self.match_if(exp)?;
        if if_true.is_loop_cont(Some(nest), is_continue) {
            Some(cond)
        } else {
            None
        }
    }

    pub fn new_node_id(&self, loc: Loc, ty: Type) -> NodeId {
        self.env.new_node(loc, ty)
    }

    pub fn clone_node_id(&self, id: NodeId) -> NodeId {
        let env = self.env;
        let new_id = env.new_node(env.get_node_loc(id), env.get_node_type(id));
        if let Some(inst) = env.get_node_instantiation_opt(id) {
            env.set_node_instantiation(new_id, inst.clone())
        }
        new_id
    }

    fn enclosing_loc<'b>(&self, exps: impl Iterator<Item = &'b Exp>) -> Loc {
        Loc::enclosing(
            exps.map(|e| self.env.get_node_loc(e.node_id()))
                .collect_vec()
                .as_slice(),
        )
    }
}
