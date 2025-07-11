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
use std::{collections::BTreeMap, vec};

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

    pub fn and(&self, exp1: Exp, exp2: Exp) -> Exp {
        use ExpData::*;
        // Pull Not to the outside so it can be eliminated with if-else
        let id = self.clone_node_id(exp1.node_id());
        if let (Some(arg1), Some(arg2)) = (
            self.extract_not(exp1.clone()),
            self.extract_not(exp2.clone()),
        ) {
            self.not(Call(id, Operation::Or, vec![arg1, arg2]).into_exp())
        } else {
            Call(id, Operation::And, vec![exp1, exp2]).into_exp()
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
        // We can skip creating a loop if the loop body
        //   - always terminates
        //   - does not `continue` itself
        //   - without considering the terminating expression, does not `break` itself
        let branch_cond = |loop_nest: usize, nest: usize, _: bool| nest == loop_nest;
        if let Some(body_prefix) = self
            .extract_terminated_prefix(&loc, body.clone(), 0, true)
            .filter(|prefix| !prefix.customizable_branches_to(branch_cond))
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

    /// Attempts to extract an `if-then` expression where the then branch terminates a loop
    ///   via `break[*]|continue[>0]|return|abort`.
    pub fn match_if_then_break(&self, exp: Exp) -> Option<(Exp, Exp)> {
        let node_id = exp.node_id();
        let default_loc = self.env().get_node_loc(node_id);
        match exp.as_ref() {
            ExpData::IfElse(_, cond, if_true, if_false)
                if if_false.is_unit_exp()
                    && self
                        .extract_terminated_prefix(&default_loc, if_true.clone(), 0, true)
                        .is_some() =>
            {
                Some((cond.clone(), if_true.clone()))
            },
            _ => None,
        }
    }

    /// Attempts to extract an `if-then-else` expression where the else branch terminates a loop
    ///   via `break[*]|continue[>0]|return|abort`, but the then branch does not.
    pub fn match_if_else_break(&self, exp: Exp) -> Option<(Exp, Exp, Exp)> {
        let node_id = exp.node_id();
        let default_loc = self.env().get_node_loc(node_id);
        match exp.as_ref() {
            ExpData::IfElse(_, cond, if_true, if_false)
            if self.extract_terminated_prefix(&default_loc, if_true.clone(), 0, true).is_none() // true branch does not break
            && self.extract_terminated_prefix(&default_loc, if_false.clone(), 0, true).is_some() // false branch breaks
            => {
                Some((cond.clone(), if_true.clone(), if_false.clone()))
            },
            _ => None,
        }
    }

    /// Attempts to extract an `if-then-else` expression where the then branch terminates a loop
    ///   via `break[*]|continue[>0]|return|abort`, but the else branch does not.
    pub fn match_if_break_else(&self, exp: Exp) -> Option<(Exp, Exp, Exp)> {
        let node_id = exp.node_id();
        let default_loc = self.env().get_node_loc(node_id);
        match exp.as_ref() {
            ExpData::IfElse(_, cond, if_true, if_false)
            if self.extract_terminated_prefix(&default_loc, if_true.clone(), 0, true).is_some() // true branch does not break
            && self.extract_terminated_prefix(&default_loc, if_false.clone(), 0, true).is_none() // false branch breaks
            => {
                Some((cond.clone(), if_true.clone(), if_false.clone()))
            },
            _ => None,
        }
    }

    /// Attempts to match a nested if-break structure:
    ///
    /// ```move
    /// if (c_1) {
    ///     if (c_2) {
    ///         ...
    ///         if (c_n) {
    ///             <then_branch> which break[*]|continue[>0]|return|abort
    ///         }
    ///     }
    /// }
    /// ```
    ///
    /// This will merge conditions as `c_1 && c_2 && ..` and return the <then_branch> as an expression.
    pub fn match_nested_if_break(&self, mut exp: Exp) -> Option<(Exp, Exp)> {
        let node_id = exp.node_id();
        let default_loc = self.env().get_node_loc(node_id);
        let mut cond = None;
        loop {
            let (c, if_true) = self.match_if(exp)?;
            match cond {
                None => cond = Some(c),
                Some(old) => cond = Some(self.and(old, c)),
            }
            if self
                .extract_terminated_prefix(&default_loc, if_true.clone(), 0, true)
                .is_some()
            {
                return Some((cond.expect("condition must be present"), if_true));
            }
            exp = if_true;
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

    /// Attempts to extract a sequence of if-break-else-branch
    ///
    /// ```move
    /// <begin_stmts>
    /// if (c_1) break;
    /// <else_branch_1>
    /// if (c_2) break;
    /// <else_branch_2>
    /// if (c_n) break;
    /// <else_branch_n>
    /// ```
    /// This will return the results as a vector <[seq(begin_stmts), c_1, seq(else_branch_1), c_2, seq(else_branch_2), ...]>
    pub fn match_if_break_else_list(&self, mut exp: Exp, nest: usize) -> Option<Vec<Exp>> {
        let default_loc = self.env.get_node_loc(exp.node_id());
        // Global vector to track the `if-break-then` list
        let mut if_branch_list = vec![];
        // Local vector to track the current sequence of expressions that represent the <else_branch> before an if-break
        let mut cur_branch = vec![];

        loop {
            let (first, rest) = self.extract_first(exp.clone());
            // Found a `if-break`
            if let Some(c) = self.match_if_loop_cont(first.clone(), nest, false) {
                // save the <else_branch> before the if-break
                let seq = self.seq(&default_loc, std::mem::take(&mut cur_branch));
                if_branch_list.push(seq);
                // save the condition
                if_branch_list.push(c);
            } else {
                cur_branch.push(first);
            };
            // No more exps to process
            if rest.is_empty() {
                break;
            }
            exp = self.seq(&default_loc, rest)
        }

        // Do not forget to add <else_branch_n>
        let seq = self.seq(&default_loc, cur_branch.clone());
        if_branch_list.push(seq);
        Some(if_branch_list)
    }

    /// Attempts to extract a sequence of if-then-break-else-branch
    ///
    /// ```move
    /// <begin_stmts>
    /// if (c_1) {
    ///     <then_branch_1> which break[*]|continue[>0]|return|abort
    /// }
    /// <else_branch_1>
    /// if (c_2) {
    ///     <then_branch_2> which break[*]|continue[>0]|return|abort
    /// }
    /// <else_branch_2>
    /// if (c_n) {
    ///     <then_branch_n> which break[*]|continue[>0]|return|abort
    /// }
    /// <else_branch_n>
    /// ```
    ///
    /// This will return the results as a vector <[seq(begin_stmts), c_1, seq(then_branch_1), seq(else_branch_1), c_2, seq(then_branch_2), seq(else_branch_2), ...]>
    pub fn match_if_branch_break_branch_list(&self, mut exp: Exp) -> Option<Vec<Exp>> {
        let default_loc = self.env.get_node_loc(exp.node_id());
        // Vector to track the global `if-then-else` list
        let mut if_else_list = vec![];
        // Vector to track the current sequence of expressions that represent the <else_branch> before an `if-then`
        let mut cur_branch = vec![];

        loop {
            let (first, rest) = self.extract_first(exp.clone());
            // Found a `if-then-break`
            if let Some((c, if_true)) = self.match_if_then_break(first.clone()) {
                // Save the previous <else_branch> (meanwhile clean up `cur_branch`)
                let seq = self.seq(&default_loc, std::mem::take(&mut cur_branch));
                if_else_list.push(seq);
                // Save the condition
                if_else_list.push(c);
                // Save the <then_branch>
                if_else_list.push(if_true);
            } else {
                // Not an `if-then-break`, so we add it to the current sequence
                cur_branch.push(first);
            };
            // No more exps to process
            if rest.is_empty() {
                break;
            }
            exp = self.seq(&default_loc, rest)
        }

        // Do not forget to add <else_branch_n>
        let seq = self.seq(&default_loc, cur_branch.clone());
        if_else_list.push(seq);
        Some(if_else_list)
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
