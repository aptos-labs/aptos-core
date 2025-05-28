// Copyright (c) Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

//! This module implements an expression linter that checks for needless references
//! taken for field access.
//! E.g., `(&s).f` can be simplified to `s.f`.
//!       `(&mut s).f = 42;` can be simplified to `s.f = 42;`.
//! making code easier to read in these cases.

use move_compiler_v2::external_checks::ExpChecker;
use move_model::{
    ast::{ExpData, Operation},
    model::FunctionEnv,
};

#[derive(Default)]
pub struct NeedlessRefInFieldAccess;

impl ExpChecker for NeedlessRefInFieldAccess {
    fn get_name(&self) -> String {
        "needless_ref_in_field_access".to_string()
    }

    fn visit_expr_pre(&mut self, function: &FunctionEnv, expr: &ExpData) {
        use ExpData::Call;
        use Operation::{Borrow, Select, SelectVariants};
        let Call(_, select @ (Select(..) | SelectVariants(..)), args) = expr else {
            return;
        };
        debug_assert!(
            args.len() == 1,
            "there should be exactly one argument for field access"
        );
        let Call(id, Borrow(kind), ..) = args[0].as_ref() else {
            return;
        };
        let (module_id, struct_id, field_id) = match select {
            Select(module_id, struct_id, field_id) => (module_id, struct_id, field_id),
            SelectVariants(module_id, struct_id, field_ids) => (
                module_id,
                struct_id,
                field_ids.first().expect("non-empty field selection"),
            ),
            _ => unreachable!("select is limited to the two variants above"),
        };
        let env = function.env();
        let field_name = env
            .get_module(*module_id)
            .into_struct(*struct_id)
            .get_field(*field_id)
            .get_name()
            .display(env.symbol_pool())
            .to_string();
        let ref_kind = kind.to_string();
        self.report(
            env,
            &env.get_node_loc(*id),
            &format!(
                "Needless {} taken for field access: \
                consider removing {} and directly accessing the field `{}`",
                ref_kind, ref_kind, field_name
            ),
        );
    }
}
