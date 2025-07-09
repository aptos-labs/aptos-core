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
        use ExpData::*;
        if let Some(arg) = self.extract_not(exp.clone()) {
            arg
        } else {
            Call(self.clone_node_id(exp.node_id()), Operation::Not, vec![exp]).into_exp()
        }
    }

    pub fn or(&self, exp1: Exp, exp2: Exp) -> Exp {
        use ExpData::*;
        // Pull Not to the outside so it can be eliminated with if-else
        let id = self.clone_node_id(exp1.node_id());
        if let (Some(arg1), Some(arg2)) = (
            self.extract_not(exp1.clone()),
            self.extract_not(exp2.clone()),
        ) {
            self.not(Call(id, Operation::And, vec![arg1, arg2]).into_exp())
        } else {
            Call(id, Operation::Or, vec![exp1, exp2]).into_exp()
        }
    }

    pub fn if_else(&self, cond: Exp, if_true: Exp, if_false: Exp) -> Exp {
        if if_false.is_unit_exp() {
            self.if_(cond, if_true)
        } else {
            let loc = self.enclosing_loc([&cond, &if_true, &if_false].into_iter());
            let ty = self.env.get_node_type(if_true.node_id());
            let id = self.new_node_id(loc, ty);
            if let Some(arg) = self.extract_not(cond.clone()) {
                ExpData::IfElse(id, arg, if_false, if_true).into_exp()
            } else {
                ExpData::IfElse(id, cond, if_true, if_false).into_exp()
            }
        }
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
        // Filter out nop statements
        let mut stms = stms.into_iter().filter(|e| !e.is_unit_exp()).collect_vec();
        match stms.len() {
            0 => self.nop(nop_loc),
            1 => stms.pop().unwrap(),
            _ => {
                let ty = if let Some(last) = stms.last() {
                    self.env.get_node_type(last.node_id())
                } else {
                    Type::unit()
                };
                let loc = self.enclosing_loc(stms.iter());
                ExpData::Sequence(self.new_node_id(loc, ty), stms).into_exp()
            },
        }
    }

    pub fn block(&self, pat: Pattern, def: Option<Exp>, body: Exp) -> Exp {
        let node_id = self.clone_node_id(body.node_id());
        ExpData::Block(node_id, pat, def, body).into_exp()
    }

    pub fn nop(&self, loc: &Loc) -> Exp {
        ExpData::Sequence(self.new_node_id(loc.clone(), Type::unit()), vec![]).into_exp()
    }

    /// Constructs a loop, simplifying redundant loop.
    pub fn loop_(&self, body: Exp) -> Exp {
        let loc = self.env().get_node_loc(body.node_id());
        if let Some(body_prefix) = self
            .extract_terminated_prefix(&loc, body.clone(), 0, true)
            .filter(|prefix| !prefix.branches_to(0..1))
        {
            body_prefix.rewrite_loop_nest(-1)
        } else {
            let node_id = self.new_node_id(loc, Type::unit());
            ExpData::Loop(node_id, body).into_exp()
        }
    }

    /// Extract the prefix of an expression which is terminated by a break of the
    /// given nest, or, if allow_exit is true, which is exited to outer level via
    /// abort, return, or break[n]/continue[n] with n>nest. This looks at the last statement
    /// of a sequence and, if necessary, recursively descents into blocks.
    /// If the last statement is actually terminating with break_nest, it is removed.
    pub fn extract_terminated_prefix(
        &self,
        default_loc: &Loc,
        exp: Exp,
        break_nest: usize,
        allow_exit: bool,
    ) -> Option<Exp> {
        use ExpData::*;
        let (front, last) = self.extract_last(exp.clone());
        match last.as_ref() {
            LoopCont(_, nest, false) if *nest == break_nest => Some(self.seq(default_loc, front)),
            LoopCont(.., nest, _) if allow_exit && *nest > break_nest => Some(exp),
            Return(..) | Call(_, Operation::Abort, _) if allow_exit => Some(exp),
            Block(id, pat, binding, scope) => {
                let scope = self.extract_terminated_prefix(
                    default_loc,
                    scope.clone(),
                    break_nest,
                    allow_exit,
                )?;
                Some(Block(*id, pat.clone(), binding.clone(), scope).into_exp())
            },
            _ => None,
        }
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

    pub fn extract_not(&self, exp: Exp) -> Option<Exp> {
        if let ExpData::Call(_, Operation::Not, args) = exp.as_ref() {
            Some(args.last().expect("expect arguments for not").clone())
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

    /// Attempts to extract a sequence of conditional breaks:
    ///
    /// ```move
    /// if (c1) break;
    /// if (c2) break;
    /// ..
    /// <rest>
    /// ```
    ///
    /// This will merge conditions as `c1 || c2 || ..` and return the rest as an expression.
    pub fn match_if_break_list(&self, mut exp: Exp) -> Option<(Exp, Exp)> {
        let default_loc = self.env.get_node_loc(exp.node_id());
        let mut cond = None;
        loop {
            let (first, rest) = self.extract_first(exp.clone());
            let Some(c) = self.match_if_loop_cont(first, 0, false) else {
                break;
            };
            match cond {
                None => cond = Some(c),
                Some(old) => cond = Some(self.or(old, c)),
            }
            exp = self.seq(&default_loc, rest)
        }
        cond.map(|c| (c, exp))
    }

    /// Attempts to extract a sequence of if-break-branch
    ///
    /// ```move
    /// branch0 // does not refer to inner loop
    /// if (c1) break;
    /// branch1 // does not refer to inner loop
    /// if (c2) break;
    /// branch2 // does not refer to inner loop
    /// ...
    /// branch_n // we allow the last branch to refer to inner loop; any caller should be aware of this
    /// ```
    ///
    /// This will return the results as a vector <[seq(branch0), c1, seq(branch1), c2, seq(branch2), ...]>
    pub fn match_nested_if_in_loop(&self, mut exp: Exp) -> Option<Vec<Exp>> {
        let default_loc = self.env.get_node_loc(exp.node_id());
        // Vector to track the nested-if sequence
        let mut nested_if = vec![];
        // Vector to track the current sequence of expressions
        let mut cur_seq = vec![];

        loop {
            let (first, rest) = self.extract_first(exp.clone());
            // Found a if-break
            if let Some(c) = self.match_if_loop_cont(first.clone(), 0, false) {
                // Create a new `seq` statment with exps in `cur_seq` and cleans up `cur_seq`
                let seq = self.seq(&default_loc, std::mem::take(&mut cur_seq));
                // Make sure the seq does not refer to the inner loop
                if seq.branches_to(0..1) {
                    return None;
                }
                nested_if.push(seq);
                nested_if.push(c);
            } else {
                // Not an if-break, so we add it to the current sequence
                cur_seq.push(first);
            };
            // No more exps to process
            if rest.is_empty() {
                break;
            }
            exp = self.seq(&default_loc, rest)
        }

        // Do not forget to add the last sequence
        let seq = self.seq(&default_loc, cur_seq.clone());
        nested_if.push(seq);
        Some(nested_if)
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
