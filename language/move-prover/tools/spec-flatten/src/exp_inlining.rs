// Copyright (c) The Diem Core Contributors
// SPDX-License-Identifier: Apache-2.0

use anyhow::Result;
use std::collections::BTreeMap;

use bytecode::function_target::FunctionTarget;
use move_model::{
    ast::{Condition, ConditionKind, Exp, ExpData, Operation, Spec, TempIndex},
    model::GlobalEnv,
    symbol::Symbol,
};

use crate::workflow::WorkflowOptions;

pub(crate) fn inline_all_exp_in_spec(
    _options: &WorkflowOptions,
    target: FunctionTarget,
    spec: Spec,
) -> Result<Spec> {
    let env = target.global_env();
    let inliner = ExpInliner { env };

    let Spec {
        loc,
        conditions,
        properties,
        on_impl,
    } = spec;

    let mut local_vars = BTreeMap::new();
    let mut new_conditions = vec![];
    for cond in conditions {
        let Condition {
            loc,
            kind,
            properties,
            exp,
            additional_exps,
        } = cond;

        match &kind {
            ConditionKind::LetPre(sym) | ConditionKind::LetPost(sym) => {
                let var_exp = inliner.inline_exp(&exp, None, Some(&local_vars));
                local_vars.insert(*sym, var_exp);
            }
            _ => {
                let new_exp = inliner.inline_exp(&exp, None, Some(&local_vars));
                let new_additional_exps = additional_exps
                    .into_iter()
                    .map(|e| inliner.inline_exp(&e, None, Some(&local_vars)))
                    .collect();
                let new_cond = Condition {
                    loc,
                    kind,
                    properties,
                    exp: new_exp,
                    additional_exps: new_additional_exps,
                };
                new_conditions.push(new_cond);
            }
        }
    }

    let new_spec = Spec {
        loc,
        conditions: new_conditions,
        properties,
        on_impl,
    };
    Ok(new_spec)
}

struct ExpInliner<'env> {
    env: &'env GlobalEnv,
}

impl ExpInliner<'_> {
    fn inline_exp(
        &self,
        exp: &Exp,
        temp_var_repl: Option<&BTreeMap<TempIndex, Exp>>,
        local_var_repl: Option<&BTreeMap<Symbol, Exp>>,
    ) -> Exp {
        use Operation::*;

        let mut rewriter = |e: Exp| match e.as_ref() {
            ExpData::LocalVar(_, sym) => match local_var_repl {
                None => Err(e),
                Some(var_map) => Ok(var_map.get(sym).unwrap().clone()),
            },
            ExpData::Temporary(_, idx) => match temp_var_repl {
                None => Err(e),
                Some(var_map) => Ok(var_map.get(idx).unwrap().clone()),
            },
            ExpData::Call(node_id, Function(mid, fid, _), args) => {
                let callee_menv = self.env.get_module(*mid);
                let callee_decl = callee_menv.get_spec_fun(*fid);
                debug_assert_eq!(args.len(), callee_decl.params.len());
                if callee_decl.is_native || callee_decl.uninterpreted || callee_decl.body.is_none()
                {
                    Err(e)
                } else {
                    let mut callee_local_vars =
                        local_var_repl.cloned().unwrap_or_else(BTreeMap::new);
                    for (arg_exp, (sym, _)) in args
                        .iter()
                        .map(|e| self.inline_exp(e, temp_var_repl, local_var_repl))
                        .zip(callee_decl.params.iter())
                    {
                        callee_local_vars.insert(*sym, arg_exp);
                    }

                    let callee_targs = self.env.get_node_instantiation(*node_id);
                    let callee_body = ExpData::rewrite_node_id(
                        callee_decl.body.as_ref().unwrap().clone(),
                        &mut |id| ExpData::instantiate_node(self.env, id, &callee_targs),
                    );
                    Ok(self.inline_exp(&callee_body, temp_var_repl, Some(&callee_local_vars)))
                }
            }
            ExpData::Invoke(_, lambda, args) => match lambda.as_ref() {
                ExpData::Lambda(_, locals, body) => {
                    debug_assert_eq!(args.len(), locals.len());
                    let mut lambda_local_vars =
                        local_var_repl.cloned().unwrap_or_else(BTreeMap::new);
                    for (arg_exp, decl) in args
                        .iter()
                        .map(|e| self.inline_exp(e, temp_var_repl, local_var_repl))
                        .zip(locals)
                    {
                        lambda_local_vars.insert(decl.name, arg_exp);
                    }
                    Ok(self.inline_exp(body, temp_var_repl, Some(&lambda_local_vars)))
                }
                _ => Err(e),
            },
            ExpData::Block(_, var_decls, body) => {
                let mut block_local_vars = local_var_repl.cloned().unwrap_or_else(BTreeMap::new);
                for var_decl in var_decls {
                    let var_exp = self.inline_exp(
                        var_decl.binding.as_ref().unwrap(),
                        temp_var_repl,
                        Some(&block_local_vars),
                    );
                    block_local_vars.insert(var_decl.name, var_exp);
                }
                Ok(self.inline_exp(body, temp_var_repl, Some(&block_local_vars)))
            }
            _ => Err(e),
        };
        ExpData::rewrite(exp.clone(), &mut rewriter)
    }
}
