// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Detects function calls that can be written using receiver-style (method) syntax.

use move_compiler_v2::external_checks::ExpChecker;
use move_model::{
    ast::{ExpData, Operation},
    model::{FunctionEnv, SurfaceSyntax},
    well_known::{VECTOR_BORROW, VECTOR_BORROW_MUT},
};

#[derive(Default)]
pub struct UseReceiverStyle;

impl ExpChecker for UseReceiverStyle {
    fn get_name(&self) -> String {
        "use_receiver_style".to_string()
    }

    fn visit_expr_pre(&mut self, function: &FunctionEnv, expr: &ExpData) {
        let ExpData::Call(id, Operation::MoveFunction(mid, fid), _) = expr else {
            return;
        };
        let env = function.env();

        if env.has_surface_syntax(*id, SurfaceSyntax::ReceiverCall)
            || env.has_surface_syntax(*id, SurfaceSyntax::IndexNotation)
        {
            return;
        }

        let called_fun = env.get_function(mid.qualified(*fid));
        if !called_fun.is_receiver_function() {
            return;
        }

        // vector::borrow/borrow_mut are handled by use_index_syntax.
        if called_fun.is_well_known(VECTOR_BORROW_MUT) || called_fun.is_well_known(VECTOR_BORROW) {
            return;
        }

        let func_name = called_fun.get_name_str();
        let replacement = if called_fun.get_parameter_count() > 1 {
            format!("<first_arg>.{}(<rest_args>)", func_name)
        } else {
            format!("<arg>.{}()", func_name)
        };
        self.report(
            env,
            &env.get_node_loc(*id),
            &format!(
                "this function call can be written as `{}` using receiver-style syntax.",
                replacement,
            ),
        );
    }
}
