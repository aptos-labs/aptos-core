// Copyright (c) Verichains
// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use move_stackless_bytecode::function_target::FunctionTarget;

use super::super::{
    super::ResultUsageType,
    utils::{expr_and, expr_or},
};
use crate::decompiler::reconstruct::{
    ast::optimizers::utils::{
        blocks_iter_with_last_effective_indicator, has_effective_statement,
        last_effective_statements,
    },
    DecompiledCodeItem, DecompiledCodeUnit, DecompiledCodeUnitRef, DecompiledExpr,
};

/// if (cond) { expr1 } else { expr2 } -> cond && expr1 || expr2
pub(crate) fn rewrite_short_circuit_if_else(
    unit: &DecompiledCodeUnitRef,
    func_target: &FunctionTarget<'_>,
    _top_level: bool,
) -> Result<DecompiledCodeUnitRef, anyhow::Error> {
    let mut new_unit = DecompiledCodeUnit::new();

    for item in unit.blocks.iter() {
        match item {
            DecompiledCodeItem::WhileStatement { cond, body } => {
                let body = rewrite_short_circuit_if_else(body, func_target, false)?;

                new_unit.add(DecompiledCodeItem::WhileStatement {
                    cond: cond.clone(),
                    body,
                });
            }

            DecompiledCodeItem::IfElseStatement {
                cond,
                if_unit,
                else_unit,
                result_variables,
                use_as_result,
            } => {
                let new_cond =
                    last_effective_statements::<1>(&new_unit.blocks).and_then(|[(idx, item)]| {
                        if let DecompiledCodeItem::AssignStatement {
                            variable,
                            value,
                            is_decl: true,
                        } = item {
                            if cond
                                .is_single_variable_expr()
                                .map_or(false, |v| v == *variable) {
                                Some((idx, value.clone()))
                            } else {
                                None
                            }
                        } else {
                            None
                        }
                    });

                let cond = if let Some((idx, new_cond)) = new_cond {
                    new_unit.blocks.drain(idx..);
                    new_cond
                } else {
                    cond.clone()
                };

                let if_unit = rewrite_short_circuit_if_else(if_unit, func_target, false)?;
                let else_unit = rewrite_short_circuit_if_else(else_unit, func_target, false)?;

                if result_variables.len() == 1
                    && func_target.get_local_type(result_variables[0]).is_bool()
                    && !has_effective_statement(&if_unit.blocks)
                    && !has_effective_statement(&else_unit.blocks)
                    && if_unit.exit.is_some()
                    && else_unit.exit.is_some() {
                    new_unit.add(DecompiledCodeItem::AssignStatement {
                        variable: result_variables[0],
                        value: DecompiledExpr::EvaluationExpr(
                            expr_or(
                                expr_and(
                                    cond.to_expr()?,
                                    if_unit.exit.as_ref().unwrap().to_expr()?,
                                ),
                                else_unit.exit.as_ref().unwrap().to_expr()?,
                            )
                            .borrow()
                            .operation
                            .to_expr(),
                        )
                        .boxed(),

                        is_decl: use_as_result == &ResultUsageType::None,
                    });
                } else {
                    new_unit.add(DecompiledCodeItem::IfElseStatement {
                        cond: cond.clone(),
                        if_unit,
                        else_unit,
                        result_variables: result_variables.clone(),
                        use_as_result: use_as_result.clone(),
                    });
                }
            }

            _ => new_unit.add(item.clone()),
        }
    }

    let effective_blocks: Vec<_> = blocks_iter_with_last_effective_indicator(&new_unit.blocks)
        .enumerate()
        .filter(|(_, item)| item.is_effective)
        .map(|(idx, _)| idx)
        .collect();

    if effective_blocks.len() == 1 {
        if let Some(v) = &unit.exit.as_ref().and_then(|x| x.is_single_variable_expr()) {
            let reduced_value = if let DecompiledCodeItem::AssignStatement {
                variable,
                value,
                is_decl: true,
            } = &new_unit.blocks[effective_blocks[0]] {
                if variable == v {
                    Some(value.clone())
                } else {
                    None
                }
            } else {
                None
            };

            if let Some(reduced_value) = reduced_value {
                new_unit.blocks.drain(effective_blocks[0]..);
                new_unit.exit = Some(reduced_value);
                return Ok(new_unit);
            }
        }
    }

    new_unit.exit = unit.exit.clone();
    new_unit.result_variables = unit.result_variables.clone();

    Ok(new_unit)
}
