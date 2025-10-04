// Copyright (c) Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

//! This module implements an expression linter that checks for calls to
//! aptos_framework::randomness::u*_integer() whose results are passed as the
//! dividend to a modulo operation.

use move_compiler_v2::external_checks::ExpChecker;
use move_model::{
    ast::{ExpData, Operation},
    model::FunctionEnv,
};
use once_cell::sync::Lazy;
use std::collections::HashMap;

static FUNCTIONS: Lazy<HashMap<String, String>> = Lazy::new(|| {
    let mut ret = HashMap::new();
    for i in 0..6 {
        let k = format!("randomness::u{}_integer", 8 << i);
        let v = format!("u{}_range", 8 << i);
        ret.insert(k, v);
    }
    ret
});

#[derive(Default)]
pub struct RandomModulo;

impl ExpChecker for RandomModulo {
    fn get_name(&self) -> String {
        "random_modulo".to_string()
    }

    fn visit_expr_pre(&mut self, function: &FunctionEnv, expr: &ExpData) {
        let env = function.env();

        let ExpData::Call(node_id, Operation::Mod, exps) = expr else {
            return;
        };

        if exps.len() != 2 {
            return;
        }

        let Some(first) = exps.first() else {
            return;
        };
        let ExpData::Call(node_id2, f, args) = first as &ExpData else {
            return;
        };
        if !matches!(f, Operation::MoveFunction(_, _)) {
            return;
        }
        let s = f.display(env, *node_id2).to_string();
        if !args.is_empty() {
            return;
        }

        let Some(suggestion) = FUNCTIONS.get(&s) else {
            return;
        };

        self.report(env, &env.get_node_loc(*node_id), format!("Using % to generate random numbers may introduce bias. Consider using {}(0, N) instead", suggestion).as_str());
    }
}
