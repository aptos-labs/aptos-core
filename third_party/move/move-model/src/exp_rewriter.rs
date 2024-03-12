// Copyright (c) The Diem Core Contributors
// Copyright (c) The Move Contributors
// SPDX-License-Identifier: Apache-2.0

use crate::{
    ast::{Condition, Exp, ExpData, MemoryLabel, Operation, Pattern, Spec, TempIndex, Value},
    model::{GlobalEnv, ModuleId, NodeId, SpecVarId},
    symbol::Symbol,
    ty::Type,
};
use codespan_reporting::diagnostic::Severity;
use itertools::Itertools;
use log::trace;
use move_binary_format::file_format::CodeOffset;
use std::collections::{BTreeMap, BTreeSet};

/// Rewriter for expressions, allowing to substitute locals by expressions as well as instantiate
/// types.
///
/// *** Note: will fail to rewrite variables when they appear in an `Assign` expression.***
/// (For pure functions, or variables guaranteed not to be assigned, this may be OK.)
pub struct ExpRewriter<'env, 'rewriter> {
    env: &'env GlobalEnv,
    replacer: &'rewriter mut dyn FnMut(NodeId, RewriteTarget) -> Option<Exp>,
    type_args: &'rewriter [Type],
    shadowed: Vec<BTreeSet<Symbol>>,
}

/// A target for expression rewrites of either an `Exp::LocalVar` or an `Exp::Temporary`.
/// This is used as a parameter to the `replacer` function which defines the behavior of
/// the rewriter. Notice we use a single function entry point for `replacer` to allow it
/// to be a function which mutates its context.
pub enum RewriteTarget {
    LocalVar(Symbol),
    Temporary(TempIndex),
}

impl<'env, 'rewriter> ExpRewriter<'env, 'rewriter> {
    /// Creates a new rewriter with the given replacer map.
    pub fn new<F>(env: &'env GlobalEnv, replacer: &'rewriter mut F) -> Self
    where
        F: FnMut(NodeId, RewriteTarget) -> Option<Exp>,
    {
        ExpRewriter {
            env,
            replacer,
            type_args: &[],
            shadowed: Vec::new(),
        }
    }

    /// Adds a type argument list to this rewriter. Generic type parameters are replaced by
    /// the given types.
    pub fn set_type_args(mut self, type_args: &'rewriter [Type]) -> Self {
        self.type_args = type_args;
        self
    }

    /// Test for shadowing
    fn is_shadowed(&self, sym: Symbol) -> bool {
        for vars in &self.shadowed {
            if vars.contains(&sym) {
                return true;
            }
        }
        false
    }
}

impl<'env, 'rewriter> ExpRewriterFunctions for ExpRewriter<'env, 'rewriter> {
    fn rewrite_enter_scope<'a>(
        &mut self,
        _id: NodeId,
        vars: impl Iterator<Item = &'a (NodeId, Symbol)>,
    ) {
        self.shadowed
            .push(vars.map(|(_node_id, symbol)| *symbol).collect())
    }

    fn rewrite_exit_scope(&mut self, _id: NodeId) {
        self.shadowed.pop();
    }

    fn rewrite_local_var(&mut self, id: NodeId, sym: Symbol) -> Option<Exp> {
        if self.is_shadowed(sym) {
            None
        } else {
            (*self.replacer)(id, RewriteTarget::LocalVar(sym))
        }
    }

    // As `ExpRewriter` has no mechanism to rewrite a `Pattern::Var` to a `Pattern` we check to make
    // sure that its `replacer` function doesn't want to rewrite a scoped symbol appearing in the
    // pattern for an `Assign`.  Patterns that create scopes (Let and Lambda) are handled by
    // `rewrite_enter_scope`, so are not addressed here.
    fn rewrite_pattern(&mut self, pat: &Pattern, creating_scope: bool) -> Option<Pattern> {
        if !creating_scope {
            // Any rewrites of variables won't happen in an `Assign` statement.
            // Let's try to enforce that aren't rewriting any symbols we find.
            if let Pattern::Var(id, sym) = pat {
                let sym_replacement = (*self.replacer)(*id, RewriteTarget::LocalVar(*sym));
                if let Some(exp) = &sym_replacement {
                    let loc = self.env.get_node_loc(*id);
                    self.env.diag(
                        Severity::Bug,
                        &loc,
                        &format!(
                            "Tried to replace symbol `{}` with expression `{}` in an `Assign` expression in `ExpRewriter`",
                            sym.display(self.env.symbol_pool()),
                            exp.display(self.env),
                        ));
                }
            }
        }
        None
    }

    fn rewrite_temporary(&mut self, id: NodeId, idx: TempIndex) -> Option<Exp> {
        (*self.replacer)(id, RewriteTarget::Temporary(idx))
    }

    fn rewrite_node_id(&mut self, id: NodeId) -> Option<NodeId> {
        ExpData::instantiate_node(self.env, id, self.type_args)
    }
}

// ======================================================================================
// Expression rewriting trait

/// A general trait for expression rewriting.
///
/// This allows customization by re-implementing any of the `rewrite_local_var`,
/// `rewrite_temporary`, etc. functions. Each expression node has an equivalent of such
/// a function.
///
/// This rewriter takes care of preserving sharing between expressions: only expression trees
/// which are actually modified are reconstructed.
///
/// For most rewriting problems, there are already specializations of this trait, like `ExpRewriter`
/// in this module, and `Exp::rewrite` in the AST module.
///
/// When custom implementing this trait, consider the semantics of the generic logic used.
/// When any of the `rewrite_<exp-variant>` functions is called, any arguments have been already
/// recursively rewritten, inclusive of the passed node id. To implement a pre-descent
/// transformation, you need to implement the `rewrite_exp` function and after pre-processing,
/// continue (or not) descent with `rewrite_exp_descent` for sub-expressions.
#[allow(unused)] // for trait default parameters
pub trait ExpRewriterFunctions {
    /// Top-level entry for rewriting an expression. Can be re-implemented to do some
    /// pre/post processing embedding a call to `rewrite_exp_descent`.
    fn rewrite_exp(&mut self, exp: Exp) -> Exp {
        self.rewrite_exp_descent(exp)
    }

    fn rewrite_vec(&mut self, exps: &[Exp]) -> Vec<Exp> {
        exps.iter().map(|e| self.rewrite_exp(e.clone())).collect()
    }

    // Functions to specialize for the rewriting problem
    // --------------------------------------------------

    fn rewrite_enter_scope<'a>(
        &mut self,
        id: NodeId,
        vars: impl Iterator<Item = &'a (NodeId, Symbol)>,
    ) {
    }
    fn rewrite_exit_scope(&mut self, id: NodeId) {}
    fn rewrite_node_id(&mut self, id: NodeId) -> Option<NodeId> {
        None
    }
    fn rewrite_local_var(&mut self, id: NodeId, sym: Symbol) -> Option<Exp> {
        None
    }
    fn rewrite_temporary(&mut self, id: NodeId, idx: TempIndex) -> Option<Exp> {
        None
    }
    fn rewrite_value(&mut self, id: NodeId, value: &Value) -> Option<Exp> {
        None
    }
    fn rewrite_spec_var(
        &mut self,
        id: NodeId,
        mid: ModuleId,
        vid: SpecVarId,
        label: &Option<MemoryLabel>,
    ) -> Option<Exp> {
        None
    }
    fn rewrite_call(&mut self, id: NodeId, oper: &Operation, args: &[Exp]) -> Option<Exp> {
        None
    }
    fn rewrite_invoke(&mut self, id: NodeId, target: &Exp, args: &[Exp]) -> Option<Exp> {
        None
    }
    fn rewrite_lambda(&mut self, id: NodeId, pat: &Pattern, body: &Exp) -> Option<Exp> {
        None
    }
    // Optionally can rewrite pat and return new value, otherwise is unchanged.
    fn rewrite_enter_block_scope(
        &mut self,
        id: NodeId,
        pat: &Pattern,
        binding: &Option<Exp>,
    ) -> Option<Pattern> {
        // Default is just to enter var scopes, but some rewriters may want to
        // do something clever with pat and binding.
        self.rewrite_enter_scope(id, pat.vars().iter());
        None
    }
    fn rewrite_assign(&mut self, id: NodeId, lhs: &Pattern, rhs: &Exp) -> Option<Exp> {
        None
    }
    // Note that `rewrite_block` is called *after* `rewrite_exit_scope`.
    // (So all parameters here have already been processed.)
    fn rewrite_block(
        &mut self,
        id: NodeId,
        pat: &Pattern,
        binding: &Option<Exp>,
        body: &Exp,
    ) -> Option<Exp> {
        None
    }
    // Optionally rewrite a pattern, which may be in `Let`, `Lambda`, or `Assign` expression.
    //
    // Parameter`creating_scope` is `true` for `Let` and `Lambda` operations, which create a new
    // variable scope.  It is `false` for `Assign` operations, which do not create a new variable
    // scope.
    //
    // Note that any subpatterns in `pat` (if any) are visited before the enclosing `Pattern`.
    fn rewrite_pattern(&mut self, pat: &Pattern, creating_scope: bool) -> Option<Pattern> {
        None
    }
    fn rewrite_quant(
        &mut self,
        id: NodeId,
        ranges: &[(Pattern, Exp)],
        triggers: &[Vec<Exp>],
        cond: &Option<Exp>,
        body: &Exp,
    ) -> Option<Exp> {
        None
    }
    fn rewrite_if_else(&mut self, id: NodeId, cond: &Exp, then: &Exp, else_: &Exp) -> Option<Exp> {
        None
    }
    fn rewrite_sequence(&mut self, id: NodeId, seq: &[Exp]) -> Option<Exp> {
        None
    }
    fn rewrite_spec(&mut self, id: NodeId, spec: &Spec) -> Option<Spec> {
        None
    }
    // Might only be useful with V1-compiled code
    fn rewrite_offset_spec(&mut self, offset: CodeOffset, spec: &Spec) -> Option<Spec> {
        None
    }

    // Core traversal functions, not intended to be re-implemented
    // -----------------------------------------------------------

    fn rewrite_exp_descent(&mut self, exp: Exp) -> Exp {
        use ExpData::*;
        match exp.as_ref() {
            Value(id, value) => {
                let (id_changed, new_id) = self.internal_rewrite_id(*id);
                if let Some(new_exp) = self.rewrite_value(new_id, value) {
                    new_exp
                } else if id_changed {
                    Value(new_id, value.clone()).into_exp()
                } else {
                    exp
                }
            },
            LocalVar(id, sym) => {
                let (id_changed, new_id) = self.internal_rewrite_id(*id);
                if let Some(new_exp) = self.rewrite_local_var(new_id, *sym) {
                    new_exp
                } else if id_changed {
                    LocalVar(new_id, *sym).into_exp()
                } else {
                    exp
                }
            },
            Temporary(id, idx) => {
                let (id_changed, new_id) = self.internal_rewrite_id(*id);
                if let Some(new_exp) = self.rewrite_temporary(new_id, *idx) {
                    new_exp
                } else if id_changed {
                    Temporary(new_id, *idx).into_exp()
                } else {
                    exp
                }
            },
            Call(id, oper, args) => {
                let (id_changed, new_id) = self.internal_rewrite_id(*id);
                let new_args_opt = self.internal_rewrite_vec(args);
                let args_ref = if let Some(new_args) = &new_args_opt {
                    new_args.as_slice()
                } else {
                    args.as_slice()
                };
                if let Some(new_exp) = self.rewrite_call(new_id, oper, args_ref) {
                    new_exp
                } else if new_args_opt.is_some() || id_changed {
                    let args_owned = if let Some(new_args) = new_args_opt {
                        new_args
                    } else {
                        args.to_owned()
                    };
                    Call(new_id, oper.clone(), args_owned).into_exp()
                } else {
                    exp
                }
            },
            Invoke(id, target, args) => {
                let (id_changed, new_id) = self.internal_rewrite_id(*id);
                let (target_changed, new_target) = self.internal_rewrite_exp(target);
                let new_args_opt = self.internal_rewrite_vec(args);
                let args_ref = if let Some(new_args) = &new_args_opt {
                    new_args.as_slice()
                } else {
                    args.as_slice()
                };
                if let Some(new_exp) = self.rewrite_invoke(new_id, &new_target, args_ref) {
                    new_exp
                } else if id_changed || target_changed || new_args_opt.is_some() {
                    let args_owned = if let Some(new_args) = new_args_opt {
                        new_args
                    } else {
                        args.to_owned()
                    };
                    Invoke(new_id, new_target, args_owned).into_exp()
                } else {
                    exp
                }
            },
            Lambda(id, pat, body) => {
                let (id_changed, new_id) = self.internal_rewrite_id(*id);
                let (pat_changed, new_pat) = self.internal_rewrite_pattern(pat, true);
                self.rewrite_enter_scope(new_id, new_pat.vars().iter());
                let (body_changed, new_body) = self.internal_rewrite_exp(body);
                self.rewrite_exit_scope(new_id);
                if let Some(new_exp) = self.rewrite_lambda(new_id, &new_pat, &new_body) {
                    new_exp
                } else if id_changed || pat_changed || body_changed {
                    Lambda(new_id, new_pat, new_body).into_exp()
                } else {
                    exp
                }
            },
            Block(id, pat, binding, body) => {
                let (id_changed, new_id) = self.internal_rewrite_id(*id);
                // Note that `binding` expr must be evaluated *before* we enter new pattern scope.
                let (binding_changed, new_binding) = if let Some(b) = binding {
                    let (changed, b) = self.internal_rewrite_exp(b);
                    (changed, Some(b))
                } else {
                    (false, None)
                };
                let (mut pat_changed, new_pat) = self.internal_rewrite_pattern(pat, true);
                let optional_pat = self.rewrite_enter_block_scope(new_id, &new_pat, &new_binding);
                let (body_changed, new_body) = self.internal_rewrite_exp(body);
                self.rewrite_exit_scope(new_id);
                let newer_pat = if let Some(rewritten_pat) = optional_pat {
                    pat_changed = true;
                    trace!(
                        "Node {} Pat changed from {:#?} to  {:#?}",
                        id.as_usize(),
                        &new_pat,
                        &rewritten_pat,
                    );
                    rewritten_pat
                } else {
                    trace!(
                        "Node {} Pat unchanged {:#?} unchanged",
                        id.as_usize(),
                        &new_pat,
                    );
                    new_pat
                };
                if let Some(new_exp) =
                    self.rewrite_block(new_id, &newer_pat, &new_binding, &new_body)
                {
                    new_exp
                } else if id_changed || pat_changed || binding_changed || body_changed {
                    Block(new_id, newer_pat, new_binding, new_body).into_exp()
                } else {
                    exp
                }
            },
            Quant(id, kind, ranges, triggers, cond, body) => {
                let (id_changed, new_id) = self.internal_rewrite_id(*id);
                let (ranges_changed, new_ranges) = self.internal_rewrite_quant_ranges(ranges);
                self.rewrite_enter_scope(
                    new_id,
                    ranges
                        .iter()
                        .flat_map(|(pat, _)| pat.vars())
                        .collect::<Vec<_>>()
                        .iter(),
                );
                let mut triggers_changed = false;
                let new_triggers = triggers
                    .iter()
                    .map(|p| {
                        let (c, new_p) = self
                            .internal_rewrite_vec(p)
                            .map(|pr| (true, pr))
                            .unwrap_or_else(|| (false, p.clone()));
                        triggers_changed = triggers_changed || c;
                        new_p
                    })
                    .collect_vec();
                let mut cond_changed = false;
                let new_cond = cond.as_ref().map(|c| {
                    let (c, new_c) = self.internal_rewrite_exp(c);
                    cond_changed = c;
                    new_c
                });
                let (body_changed, new_body) = self.internal_rewrite_exp(body);
                self.rewrite_exit_scope(new_id);
                if let Some(new_exp) =
                    self.rewrite_quant(new_id, &new_ranges, &new_triggers, &new_cond, &new_body)
                {
                    new_exp
                } else if id_changed
                    || ranges_changed
                    || triggers_changed
                    || cond_changed
                    || body_changed
                {
                    Quant(new_id, *kind, new_ranges, new_triggers, new_cond, new_body).into_exp()
                } else {
                    exp
                }
            },
            IfElse(id, cond, then, else_) => {
                let (id_changed, new_id) = self.internal_rewrite_id(*id);
                let (cond_changed, new_cond) = self.internal_rewrite_exp(cond);
                let (then_changed, new_then) = self.internal_rewrite_exp(then);
                let (else_changed, new_else) = self.internal_rewrite_exp(else_);
                if let Some(new_exp) = self.rewrite_if_else(new_id, &new_cond, &new_then, &new_else)
                {
                    new_exp
                } else if id_changed || cond_changed || then_changed || else_changed {
                    IfElse(new_id, new_cond, new_then, new_else).into_exp()
                } else {
                    exp
                }
            },
            Sequence(id, es) => {
                let (id_changed, new_id) = self.internal_rewrite_id(*id);
                let changed_vec = self.internal_rewrite_vec(es);
                let vec_changed = changed_vec.is_some();
                let new_vec = changed_vec.unwrap_or_else(|| es.clone());
                if let Some(new_exp) = self.rewrite_sequence(new_id, &new_vec) {
                    new_exp
                } else if id_changed || vec_changed {
                    Sequence(new_id, new_vec).into_exp()
                } else {
                    exp
                }
            },
            Loop(id, body) => {
                let (id_changed, new_id) = self.internal_rewrite_id(*id);
                let (body_changed, new_body) = self.internal_rewrite_exp(body);
                if id_changed || body_changed {
                    Loop(new_id, new_body).into_exp()
                } else {
                    exp
                }
            },
            LoopCont(id, do_cont) => {
                let (id_changed, new_id) = self.internal_rewrite_id(*id);
                if id_changed {
                    LoopCont(new_id, *do_cont).into_exp()
                } else {
                    exp
                }
            },
            Return(id, val) => {
                let (id_changed, new_id) = self.internal_rewrite_id(*id);
                let (val_changed, new_val) = self.internal_rewrite_exp(val);
                if id_changed || val_changed {
                    Return(new_id, new_val).into_exp()
                } else {
                    exp
                }
            },
            Assign(id, lhs, rhs) => {
                let (id_changed, new_id) = self.internal_rewrite_id(*id);
                let (rhs_changed, new_rhs) = self.internal_rewrite_exp(rhs);
                let (lhs_changed, new_lhs) = self.internal_rewrite_pattern(lhs, false);
                if let Some(new_exp) = self.rewrite_assign(new_id, &new_lhs, &new_rhs) {
                    new_exp
                } else if id_changed || lhs_changed || rhs_changed {
                    Assign(new_id, new_lhs, new_rhs).into_exp()
                } else {
                    exp
                }
            },
            Mutate(id, lhs, rhs) => {
                let (id_changed, new_id) = self.internal_rewrite_id(*id);
                let (rhs_changed, new_rhs) = self.internal_rewrite_exp(rhs);
                let (lhs_changed, new_lhs) = self.internal_rewrite_exp(lhs);
                if id_changed || lhs_changed || rhs_changed {
                    Mutate(new_id, new_lhs, new_rhs).into_exp()
                } else {
                    exp
                }
            },
            SpecBlock(id, spec) => {
                let (id_changed, new_id) = self.internal_rewrite_id(*id);
                let (spec_changed, new_spec) = self.internal_rewrite_spec(new_id, spec.clone());
                if id_changed || spec_changed {
                    SpecBlock(new_id, new_spec).into_exp()
                } else {
                    exp
                }
            },
            // This can happen since we are calling the rewriter during type checking, and
            // we may have encountered an error which is represented as an Invalid expression.
            Invalid(id) => Invalid(*id).into_exp(),
        }
    }

    fn internal_rewrite_pattern_vector(
        &mut self,
        pat_vec: &[Pattern],
        creating_scope: bool,
    ) -> (bool, Vec<Pattern>) {
        let rewritten: Vec<_> = pat_vec
            .iter()
            .map(|pat| self.internal_rewrite_pattern(pat, creating_scope))
            .collect();
        let changed = rewritten.iter().any(|(changed, pat)| *changed);
        (
            changed,
            rewritten.into_iter().map(|(changed, pat)| pat).collect(),
        )
    }

    fn internal_rewrite_pattern(&mut self, pat: &Pattern, creating_scope: bool) -> (bool, Pattern) {
        match pat {
            Pattern::Tuple(_, pattern_vec) | Pattern::Struct(_, _, pattern_vec) => {
                let (changed, final_pattern_vec) =
                    self.internal_rewrite_pattern_vector(pattern_vec, creating_scope);
                if changed {
                    let new_pat = match pat {
                        Pattern::Tuple(id, _) => Pattern::Tuple(*id, final_pattern_vec),
                        Pattern::Struct(id, struct_id, _) => {
                            Pattern::Struct(*id, struct_id.clone(), final_pattern_vec)
                        },
                        _ => unreachable!(),
                    };
                    if let Some(rewritten_new_pat) = self.rewrite_pattern(&new_pat, creating_scope)
                    {
                        return (true, rewritten_new_pat);
                    } else {
                        return (changed, new_pat);
                    }
                }
            },
            _ => {},
        }
        if let Some(rewritten_pat) = self.rewrite_pattern(pat, creating_scope) {
            (true, rewritten_pat)
        } else {
            (false, pat.clone())
        }
    }

    fn internal_rewrite_spec_condition(&mut self, condition: Condition) -> (bool, Condition) {
        let new_exp = self.rewrite_exp(condition.exp.clone());
        let maybe_new_additional_exps = self.internal_rewrite_vec(&condition.additional_exps);
        if let Some(new_additional_exps) = maybe_new_additional_exps {
            (true, Condition {
                exp: new_exp,
                additional_exps: new_additional_exps,
                ..condition
            })
        } else {
            let changed = !ExpData::ptr_eq(&condition.exp, &new_exp);
            if changed {
                (true, Condition {
                    exp: new_exp,
                    ..condition
                })
            } else {
                (false, condition)
            }
        }
    }

    fn internal_rewrite_spec_conditions(
        &mut self,
        conditions: Vec<Condition>,
    ) -> (bool, Vec<Condition>) {
        let (tests, rewritten_conds): (Vec<bool>, Vec<Condition>) = conditions
            .into_iter()
            .map(|cond| self.internal_rewrite_spec_condition(cond))
            .unzip();
        let summary_bool = tests.into_iter().any(|x| x);
        (summary_bool, rewritten_conds)
    }

    // Might only be used with v1 compile chain.
    fn internal_rewrite_spec_on_impl(
        &mut self,
        mut on_impl: BTreeMap<CodeOffset, Spec>,
    ) -> (bool, BTreeMap<CodeOffset, Spec>) {
        let mut changed = false;
        for (key, value) in on_impl.iter_mut() {
            let old_value = std::mem::take(value);
            let (changed_value, new_spec) = self.internal_rewrite_offset_spec(*key, old_value);
            *value = new_spec;
            changed = changed || changed_value;
        }
        (changed, on_impl)
    }

    fn rewrite_spec_update_map(
        &mut self,
        mut update_map: BTreeMap<NodeId, Condition>,
    ) -> (bool, BTreeMap<NodeId, Condition>) {
        let (changed_vec, new_map): (Vec<bool>, BTreeMap<NodeId, Condition>) = update_map
            .into_iter()
            .map(|(id, cond)| {
                let (changed, new_cond) = self.internal_rewrite_spec_condition(cond);
                (changed, (id, new_cond))
            })
            .unzip();
        let changed = changed_vec.into_iter().any(|x| x);
        (changed, new_map)
    }

    fn internal_rewrite_offset_spec(&mut self, offset: CodeOffset, spec: Spec) -> (bool, Spec) {
        let (conditions_changed, new_conditions) =
            self.internal_rewrite_spec_conditions(spec.conditions);
        let (on_impl_changed, new_on_impl) = self.internal_rewrite_spec_on_impl(spec.on_impl);
        let (update_map_changed, new_update_map) = self.rewrite_spec_update_map(spec.update_map);
        let newspec = Spec {
            conditions: new_conditions,
            on_impl: new_on_impl,
            update_map: new_update_map,
            ..spec
        };
        if let Some(newer_spec) = self.rewrite_offset_spec(offset, &newspec) {
            (true, newer_spec)
        } else {
            (
                conditions_changed || on_impl_changed || update_map_changed,
                newspec,
            )
        }
    }

    fn internal_rewrite_spec(&mut self, id: NodeId, spec: Spec) -> (bool, Spec) {
        let (conditions_changed, new_conditions) =
            self.internal_rewrite_spec_conditions(spec.conditions);
        let (on_impl_changed, new_on_impl) = self.internal_rewrite_spec_on_impl(spec.on_impl);
        let (update_map_changed, new_update_map) = self.rewrite_spec_update_map(spec.update_map);
        let newspec = Spec {
            conditions: new_conditions,
            on_impl: new_on_impl,
            update_map: new_update_map,
            ..spec
        };
        if let Some(newer_spec) = self.rewrite_spec(id, &newspec) {
            (true, newer_spec)
        } else {
            (
                conditions_changed || on_impl_changed || update_map_changed,
                newspec,
            )
        }
    }

    fn internal_rewrite_id(&mut self, id: NodeId) -> (bool, NodeId) {
        if let Some(new_id) = self.rewrite_node_id(id) {
            (true, new_id)
        } else {
            (false, id)
        }
    }

    fn internal_rewrite_exp(&mut self, exp: &Exp) -> (bool, Exp) {
        let new_exp = self.rewrite_exp(exp.clone());
        (!ExpData::ptr_eq(exp, &new_exp), new_exp)
    }

    fn internal_rewrite_vec(&mut self, exps: &[Exp]) -> Option<Vec<Exp>> {
        // The vector rewrite works a bit different as we try to avoid constructing
        // new vectors if nothing changed, and optimize common cases of 0-3 arguments.
        let (changevec, resvec): (Vec<_>, Vec<_>) = exps
            .iter()
            .map(|exp| self.internal_rewrite_exp(exp))
            .unzip();
        let changed = changevec.into_iter().any(|x| x);
        if changed {
            Some(resvec)
        } else {
            None
        }
    }

    fn internal_rewrite_quant_ranges(
        &mut self,
        ranges: &[(Pattern, Exp)],
    ) -> (bool, Vec<(Pattern, Exp)>) {
        let (changevec, new_ranges): (Vec<_>, Vec<_>) = ranges
            .iter()
            .map(|(pat, exp)| {
                let (pat_changed, new_pat) = self.internal_rewrite_pattern(pat, true);
                let (exp_changed, new_exp) = self.internal_rewrite_exp(exp);
                (pat_changed || exp_changed, (new_pat, new_exp))
            })
            .unzip();
        let change = changevec.into_iter().any(|x| x);
        (change, new_ranges)
    }
}
