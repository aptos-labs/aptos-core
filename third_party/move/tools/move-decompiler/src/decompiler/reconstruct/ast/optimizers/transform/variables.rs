// Copyright (c) Verichains
// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use std::collections::HashMap;

use crate::decompiler::reconstruct::{DecompiledCodeItem, DecompiledCodeUnit};

pub(crate) fn rename_variables(
    unit: &mut DecompiledCodeUnit,
    renamed_variables: &HashMap<usize, usize>,
) {
    unit.exit
        .as_mut()
        .map(|x| x.rename_variables(renamed_variables));

    unit.result_variables = unit
        .result_variables
        .iter()
        .map(|x| renamed_variables[x])
        .collect();

    for item in unit.blocks.iter_mut() {
        match item {
            DecompiledCodeItem::AbortStatement(x) | DecompiledCodeItem::ReturnStatement(x) => {
                x.rename_variables(renamed_variables)
            }

            DecompiledCodeItem::PreDeclareStatement { variable } => {
                *variable = renamed_variables[variable];
            }

            DecompiledCodeItem::AssignStatement {
                variable, value, ..
            } => {
                *variable = renamed_variables[variable];
                value.rename_variables(renamed_variables);
            }

            DecompiledCodeItem::PossibleAssignStatement {
                variable, value, ..
            } => {
                *variable = renamed_variables[variable];
                value.rename_variables(renamed_variables);
            }

            DecompiledCodeItem::AssignTupleStatement {
                variables, value, ..
            } => {
                for v in variables.iter_mut() {
                    *v = renamed_variables[v];
                }
                value.rename_variables(renamed_variables);
            }

            DecompiledCodeItem::BreakStatement
            | DecompiledCodeItem::ContinueStatement
            | DecompiledCodeItem::CommentStatement(_) => {}
            DecompiledCodeItem::AssignStructureStatement {
                variables, value, ..
            } => {
                for v in variables.iter_mut() {
                    v.1 = renamed_variables[&v.1];
                }
                value.rename_variables(renamed_variables);
            }

            DecompiledCodeItem::Statement { expr } => {
                expr.rename_variables(renamed_variables);
            }

            DecompiledCodeItem::IfElseStatement {
                cond,
                if_unit,
                else_unit,
                result_variables,
                ..
            } => {
                cond.rename_variables(renamed_variables);
                for v in result_variables.iter_mut() {
                    *v = renamed_variables[v];
                }
                rename_variables(if_unit, renamed_variables);
                rename_variables(else_unit, renamed_variables);
            }

            DecompiledCodeItem::WhileStatement { cond, body } => {
                cond.as_mut().map(|x| x.rename_variables(renamed_variables));
                rename_variables(body, renamed_variables);
            }
        }
    }
}
