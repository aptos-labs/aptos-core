// Copyright (c) Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

//! Warn when a function returns a `signer`.
//!
//! Returning a signer can leak authority and is usually a security risk.

use move_compiler_v2::external_checks::StacklessBytecodeChecker;
use move_model::{
    model::{GlobalEnv, Visibility},
    ty::Type,
};
use move_stackless_bytecode::function_target::FunctionTarget;

pub struct ReturnSigner {}

impl StacklessBytecodeChecker for ReturnSigner {
    fn get_name(&self) -> String {
        "return_signer".to_string()
    }

    fn check(&self, target: &FunctionTarget) {
        if target.visibility() != Visibility::Public {
            return;
        }

        let env = target.global_env();
        let returns_signer = target
            .get_return_types()
            .into_iter()
            .any(|ty| contains_signer(&ty, env));
        if returns_signer {
            let loc = target.func_env.get_result_type_loc();
            self.report(
                env,
                &loc,
                "Returning a `signer` leaks authority; avoid returning signer values.",
            );
        }
    }
}

fn contains_signer(ty: &Type, _env: &GlobalEnv) -> bool {
    match ty {
        Type::Primitive(move_model::ty::PrimitiveType::Signer) => true,
        Type::Reference(_, inner) => contains_signer(inner, _env),
        Type::Vector(inner) => contains_signer(inner, _env),
        Type::Tuple(elems) => elems.iter().any(|t| contains_signer(t, _env)),
        Type::Struct(_, _, insts) => insts.iter().any(|t| contains_signer(t, _env)),
        _ => false,
    }
}
