// Copyright (c) Verichains
// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use std::{
    cmp::Ordering,
    collections::{HashMap, HashSet},
};

use move_stackless_bytecode::function_target::FunctionTarget;

use crate::decompiler::{naming::Naming, reconstruct::ast::DecompiledExprRef};

use self::transform::{
    assert::*, cleanup_tail_exit::*, if_else::*, let_return::*, loops::*, non_source_blocks::*,
    variables::*,
};

use super::super::DecompiledCodeUnitRef;
mod transform;
mod utils;
mod variable_declaration;

use utils::*;
use variable_declaration::*;

pub struct OptimizerSettings {
    pub disable_optimize_variables_declaration: bool,
}

impl Default for OptimizerSettings {
    fn default() -> Self {
        Self {
            disable_optimize_variables_declaration: false,
        }
    }
}

pub(crate) fn run(
    unit: &DecompiledCodeUnitRef,
    func_target: &FunctionTarget<'_>,
    naming: &Naming,
    settings: &OptimizerSettings,
) -> Result<(DecompiledCodeUnitRef, HashSet<usize>), anyhow::Error> {
    let mut unit = unit.clone();

    cleanup_tail_exit(&mut unit)?;
    let mut unit = rewrite_short_circuit_if_else(&unit, func_target, true)?;

    rewrite_loop(&mut unit)?;
    rewrite_let_var_return(&mut unit)?;
    let mut unit = rewrite_assert(&unit)?;
    rewrite_let_if_return(&mut unit)?;

    if !settings.disable_optimize_variables_declaration {
        rename_variables_by_order(&mut unit, func_target);
        unit = optimize_variables_declaration(&unit, naming)?;
    }

    let mut unit = remove_non_source_blocks(&unit)?;

    rename_variables_by_order(&mut unit, func_target);

    let mut referenced_variables = HashSet::new();
    let mut implicit_referenced_variables = HashSet::new();
    collect_referenced_variables(
        &unit,
        &mut referenced_variables,
        &mut implicit_referenced_variables,
    );

    Ok((unit, referenced_variables))
}

fn rename_variables_by_order(unit: &mut DecompiledCodeUnitRef, func_target: &FunctionTarget<'_>) {
    let mut live_variables = HashSet::new();
    for i in 0..func_target.get_parameter_count() {
        live_variables.insert(i);
    }
    let mut implicit_variables = HashSet::new();
    collect_live_variables(&unit, &mut live_variables, &mut implicit_variables);

    // there maybe some implicit variables that are in live_variables already, just remove them
    implicit_variables = implicit_variables
        .difference(&live_variables)
        .map(|x| *x)
        .collect();

    let live_variables = live_variables.into_iter().collect::<Vec<_>>();

    let mut variables_declaration_order = Vec::new();
    get_variable_declaration_order(unit, &mut variables_declaration_order);

    let mut renamed_variables = HashMap::new();
    for i in 0..func_target.get_parameter_count() {
        renamed_variables.insert(i, renamed_variables.len());
    }
    for v in variables_declaration_order {
        if !renamed_variables.contains_key(&v) {
            renamed_variables.insert(v, renamed_variables.len());
        }
    }

    for v in live_variables.iter() {
        if !renamed_variables.contains_key(v) {
            renamed_variables.insert(*v, renamed_variables.len());
        }
    }
    let mut implicit_variables = implicit_variables.into_iter().collect::<Vec<_>>();
    implicit_variables.sort();
    for v in implicit_variables.iter() {
        if !renamed_variables.contains_key(v) {
            renamed_variables.insert(*v, renamed_variables.len());
        }
    }
    rename_variables(unit, &renamed_variables);
}

fn optimize_variables_declaration(
    unit: &DecompiledCodeUnitRef,
    naming: &Naming,
) -> Result<DecompiledCodeUnitRef, anyhow::Error> {
    use super::super::DecompiledCodeItem as I;
    let mut solver: VariableDeclarationOptimizer = VariableDeclarationOptimizer::new();
    fn initialize_solver(solver: &mut VariableDeclarationOptimizer, unit: &DecompiledCodeUnitRef) {
        for item in unit.blocks.iter() {
            match item {
                I::IfElseStatement {
                    if_unit,
                    else_unit,
                    cond,
                    ..
                } => {
                    solver.add_expr(cond);
                    initialize_solver(solver, if_unit);
                    initialize_solver(solver, else_unit);
                }
                I::WhileStatement { body, cond } => {
                    if let Some(cond) = cond {
                        solver.add_expr(cond);
                    }
                    initialize_solver(solver, body);
                }
                I::ReturnStatement(expr)
                | I::AbortStatement(expr)
                | I::AssignStatement { value: expr, .. }
                | I::AssignTupleStatement { value: expr, .. }
                | I::AssignStructureStatement { value: expr, .. }
                | I::Statement { expr, .. } => {
                    solver.add_expr(expr);
                }
                I::BreakStatement
                | I::ContinueStatement
                | I::CommentStatement(_)
                | I::PreDeclareStatement { .. } => {}
                I::PossibleAssignStatement {
                    assigment_id: _,
                    value,
                    variable,
                    is_decl,
                } => {
                    if *is_decl {
                        solver.add_variable(variable, value);
                    }
                }
            }
        }
    }

    fn apply_variable_declaration(
        unit: &DecompiledCodeUnitRef,
        should_declare: &HashSet<usize>,
    ) -> Result<DecompiledCodeUnitRef, anyhow::Error> {
        let mut new_unit = unit.clone();
        new_unit.blocks.clear();
        for item in unit.blocks.iter() {
            match item {
                I::IfElseStatement {
                    if_unit,
                    else_unit,
                    cond,
                    result_variables,
                    use_as_result,
                } => {
                    let new_if_unit = apply_variable_declaration(if_unit, should_declare)?;
                    let new_else_unit = apply_variable_declaration(else_unit, should_declare)?;
                    new_unit.blocks.push(I::IfElseStatement {
                        if_unit: new_if_unit,
                        else_unit: new_else_unit,
                        cond: cond.commit_pending_variables(should_declare),
                        result_variables: result_variables.clone(),
                        use_as_result: use_as_result.clone(),
                    });
                }
                I::WhileStatement { body, cond } => {
                    let new_body = apply_variable_declaration(body, should_declare)?;
                    new_unit.blocks.push(I::WhileStatement {
                        body: new_body,
                        cond: cond
                            .clone()
                            .map(|c| c.commit_pending_variables(should_declare)),
                    });
                }
                I::ReturnStatement(expr) => {
                    new_unit.blocks.push(I::ReturnStatement(
                        expr.commit_pending_variables(should_declare),
                    ));
                }
                I::AbortStatement(expr) => {
                    new_unit.blocks.push(I::AbortStatement(
                        expr.commit_pending_variables(should_declare),
                    ));
                }
                I::AssignStatement {
                    variable,
                    value,
                    is_decl,
                } => {
                    new_unit.blocks.push(I::AssignStatement {
                        variable: *variable,
                        value: value.commit_pending_variables(should_declare),
                        is_decl: *is_decl,
                    });
                }
                I::AssignTupleStatement {
                    variables,
                    value,
                    is_decl,
                } => {
                    new_unit.blocks.push(I::AssignTupleStatement {
                        variables: variables.clone(),
                        value: value.commit_pending_variables(should_declare),
                        is_decl: *is_decl,
                    });
                }
                I::AssignStructureStatement {
                    structure_visible_name,
                    variables,
                    value,
                } => {
                    new_unit.blocks.push(I::AssignStructureStatement {
                        structure_visible_name: structure_visible_name.clone(),
                        variables: variables.clone(),
                        value: value.commit_pending_variables(should_declare),
                    });
                }
                I::Statement { expr } => {
                    new_unit.blocks.push(I::Statement {
                        expr: expr.commit_pending_variables(should_declare),
                    });
                }
                I::BreakStatement | I::ContinueStatement | I::CommentStatement(_) => {
                    new_unit.blocks.push(item.clone());
                }
                I::PreDeclareStatement { variable } => {
                    new_unit.blocks.push(I::PreDeclareStatement {
                        variable: *variable,
                    });
                }
                I::PossibleAssignStatement {
                    assigment_id: _,
                    value,
                    variable,
                    is_decl,
                } => {
                    if *is_decl && should_declare.contains(variable) {
                        new_unit.blocks.push(I::AssignStatement {
                            variable: *variable,
                            value: value.commit_pending_variables(should_declare),
                            is_decl: true,
                        });
                    }
                }
            }
        }
        new_unit.exit = new_unit
            .exit
            .map(|x| x.commit_pending_variables(should_declare));
        Ok(new_unit)
    }

    #[derive(Debug)]
    struct ExprCost {
        source_len: usize,
    }

    let expr_cost = |expr: &DecompiledExprRef| -> ExprCost {
        let source = expr.to_source(naming).unwrap();

        ExprCost {
            source_len: source.len(),
        }
    };

    // heuristic - less is better
    fn cost_compare(a: &Vec<ExprCost>, b: &Vec<ExprCost>) -> Ordering {
        const LINE_LENGTH: usize = 100;

        let max_source_len_a = a.iter().map(|x| x.source_len).max().unwrap_or(0);
        let max_source_len_b = b.iter().map(|x| x.source_len).max().unwrap_or(0);

        let a_source_len_overflow = max_source_len_a > LINE_LENGTH;
        let b_source_len_overflow = max_source_len_b > LINE_LENGTH;

        if a_source_len_overflow != b_source_len_overflow {
            return if a_source_len_overflow {
                Ordering::Greater
            } else {
                Ordering::Less
            };
        }

        if a_source_len_overflow {
            let ord = max_source_len_a.cmp(&max_source_len_b);
            if ord != Ordering::Equal {
                return ord;
            }
        }

        let ord = a.len().cmp(&b.len());
        if ord != Ordering::Equal {
            return ord;
        }

        if a_source_len_overflow {
            let ord = max_source_len_b.cmp(&max_source_len_a);
            if ord != Ordering::Equal {
                return ord;
            }
        }
        for (a, b) in a.iter().zip(b.iter()) {
            let ord = a.source_len.cmp(&b.source_len);
            if ord != Ordering::Equal {
                return ord;
            }
        }

        Ordering::Equal
    }

    initialize_solver(&mut solver, unit);
    solver.cleanup_non_referenced_variables();
    let should_declare = solver.solve(&expr_cost, &cost_compare);
    apply_variable_declaration(unit, &should_declare)
}
