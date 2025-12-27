// Copyright (c) Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

//! This module implements a check for loops in ranges (to..from)
//! where `to >= from`, resulting in empty ranges, thus not looping.
//! This lint does not catch other usages of ranges (e.g. Operation::Range)
use legacy_move_compiler::parser::syntax::{FOR_LOOP_UPDATE_ITER_FLAG, FOR_LOOP_UPPER_BOUND_VALUE};
use move_compiler_v2::external_checks::ExpChecker;
use move_model::{
    ast::{ExpData, Value},
    model::{FunctionEnv, GlobalEnv, NodeId},
};
use num::BigInt;

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

        self.range_check(top, bottom, expr.node_id(), env);
    }

    // Extract the loop bounds (x..y) from the desugared loop
    //```move
    //  public fun desugared_for() {
    //    let i = (X);    <--------------------------|
    //    let __update_iter_flag: bool = false;      |-- these two.
    //    let __upper_bound_value: u64 = (Y);  <-----|
    //    loop {
    //      if (true) {
    //        if (__update_iter_flag) {
    //            i = i + 1;
    //        } else {
    //            __update_iter_flag = true;
    //        };
    //        if (i < __upper_bound_value) {
    //          // body
    //        } else {
    //            break;
    //        };
    //      }
    //    }
    //  }
    //```
    fn extract_for_loop_bounds(
        &self,
        expr: &ExpData,
        fenv: &FunctionEnv,
    ) -> Option<(BigInt, BigInt)> {
        let ExpData::Block(.., p_initial, child_1) = expr else {
            return None;
        };

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

        let ExpData::Block(_, pat3, p_final, loop_expr) = child_2.as_ref() else {
            return None;
        };

        if pat3.to_string(fenv) != FOR_LOOP_UPPER_BOUND_VALUE {
            return None;
        }

        if !Self::contains_loop(loop_expr.as_ref()) {
            return None;
        }

        let top_expr = p_final.as_ref()?;
        let ExpData::Value(_, Value::Number(top)) = top_expr.as_ref() else {
            return None;
        };

        Some((bottom.clone(), top.clone()))
    }

    fn contains_loop(expr: &ExpData) -> bool {
        match expr {
            ExpData::Loop(..) => true,
            ExpData::Sequence(_, items) => items.iter().any(|e| Self::contains_loop(e.as_ref())),
            _ => false,
        }
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
            &format!(
                "The range used in this loop is empty, as the start value ({from}) is {g_o_e} the \
                 end value ({to}). The loop will not be executed."
            ),
        );
    }
}
