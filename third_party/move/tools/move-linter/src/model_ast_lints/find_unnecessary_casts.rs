// Copyright (c) Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

//! This module implements an expression linter that checks for unnecessary type casts
//! where the source expression already has the same type as the target type.
//!
//! For example:
//! * `x as u64` when `x` is already of type `u64` => unnecessary cast, use `x` directly
//!
//! The linter helps improve code readability by removing redundant type information.

use move_compiler_v2::external_checks::ExpChecker;
use move_model::{
    ast::{ExpData, Operation},
    model::FunctionEnv,
};

#[derive(Default)]
pub struct FindUnnecessaryCasts;

impl ExpChecker for FindUnnecessaryCasts {
    fn get_name(&self) -> String {
        "find_unnecessary_casts".to_string()
    }

    fn visit_expr_pre(&mut self, function: &FunctionEnv, expr: &ExpData) {
        let ExpData::Call(id, Operation::Cast, args) = expr else {
            return;
        };

        if args.len() != 1 {
            return;
        }

        let env = function.env();
        let source_type = env.get_node_type(args[0].node_id());
        let target_type = env.get_node_type(*id);

        if source_type == target_type {
            self.report(
                env,
                &env.get_node_loc(*id),
                "Unnecessary cast: the expression is already of the target type. Consider removing the cast for better readability.",
            );
        }
    }
}
