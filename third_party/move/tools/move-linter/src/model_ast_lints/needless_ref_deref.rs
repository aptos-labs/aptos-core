// Copyright (c) Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

//! This module implements an expression linter that checks for immutable reference
//! taken for a dereference (`&*`). Such pairs of operators are needless and can be
//! removed to make the code easier to read.

use move_compiler_v2::external_checks::ExpChecker;
use move_model::{
    ast::{ExpData, Operation},
    model::FunctionEnv,
    ty::ReferenceKind,
};

#[derive(Default)]
pub struct NeedlessRefDeref;

impl ExpChecker for NeedlessRefDeref {
    fn get_name(&self) -> String {
        "needless_ref_deref".to_string()
    }

    fn visit_expr_pre(&mut self, function: &FunctionEnv, expr: &ExpData) {
        use ExpData::Call;
        use Operation::{Borrow, Deref};
        use ReferenceKind::Immutable;
        let Call(id, Borrow(Immutable), args) = expr else {
            return;
        };
        debug_assert!(
            args.len() == 1,
            "there should be exactly one argument for borrow"
        );
        let Call(_, Deref, _) = args[0].as_ref() else {
            return;
        };
        let env = function.env();
        self.report(
            env,
            &env.get_node_loc(*id),
            "Needless pair of `&` and `*` operators: consider removing them",
        );
    }
}
