// Copyright (c) Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

//! This module implements a check for loops in ranges (to..from)
//! where `to >= from`, resulting in empty ranges, thus not looping.
//! This lint does not catch other usages of ranges (e.g. Operation::Range)
use legacy_move_compiler::parser::syntax::FOR_LOOP_UPDATE_ITER_FLAG;
use move_compiler_v2::external_checks::ExpChecker;
use move_model::{
    ast::{ExpData, Value},
    model::{FunctionEnv, GlobalEnv, NodeId},
};
use num::BigInt;

// This is not exposed by `legacy_move_compiler::parser::syntax`.
// If this is changed in the future, it should be changed here.
const FOR_LOOP_UPPER_BOUND_VALUE: &str = "__upper_bound_value";
const FOR_LOOP_ITER_VALUE: &str = "i";

#[derive(Default)]
pub struct EmptyRange;

impl ExpChecker for EmptyRange {
    fn get_name(&self) -> String {
        "empty_range".to_string()
    }

    fn visit_expr_pre(&mut self, fenv: &FunctionEnv, expr: &ExpData) {
        self.detect_for_loop_with_range(expr, fenv);
    }
}

impl EmptyRange {
    fn detect_for_loop_with_range(&mut self, expr: &ExpData, fenv: &FunctionEnv) {
        let env = fenv.env();

        let (bottom, top) = match self.extract_for_loop_bounds(expr, fenv) {
            Some(bounds) => bounds,
            None => return,
        };

        self.range_check(bottom, top, expr.node_id(), env);
    }

    fn extract_for_loop_bounds(
        &self,
        expr: &ExpData,
        fenv: &FunctionEnv,
    ) -> Option<(BigInt, BigInt)> {
        // Check if `then` is of the form:
        //   Sequence([IfElse(LocalVar(FOR_LOOP_UPDATE_ITER_FLAG), ...)])
        // If so, it is the `for` loop.
        let ExpData::Block(_, pat1, p_initial, child_1) = expr else {
            return None;
        };

        if pat1.to_string(fenv) != FOR_LOOP_ITER_VALUE {
            return None;
        }

        let bottom_expr = p_initial.as_ref()?;
        let ExpData::Value(_, Value::Number(bottom)) = bottom_expr.as_ref() else {
            return None;
        };

        let ExpData::Block(_, pat2, _, child_2) = child_1.as_ref() else {
            return None;
        };

        if pat2.to_string(fenv) != FOR_LOOP_UPDATE_ITER_FLAG {
            return None;
        }

        let ExpData::Block(_, pat3, p_final, _) = child_2.as_ref() else {
            return None;
        };

        if pat3.to_string(fenv) != FOR_LOOP_UPPER_BOUND_VALUE {
            return None;
        }

        let top_expr = p_final.as_ref()?;
        let ExpData::Value(_, Value::Number(top)) = top_expr.as_ref() else {
            return None;
        };

        Some((bottom.clone(), top.clone()))
    }

    fn range_check(&self, to: BigInt, from: BigInt, nid: NodeId, env: &GlobalEnv) {
        if to > from {
            return;
        }

        let g_o_e = if to == from {
            "equal to"
        } else {
            "greater than"
        };

        self.report(
            env,
            &env.get_node_loc(nid),
            &format!("This range is empty, as {from} is {g_o_e} {to}"),
        );
    }
}
