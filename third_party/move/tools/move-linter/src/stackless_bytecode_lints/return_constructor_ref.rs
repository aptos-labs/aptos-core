// Copyright (c) Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

//! Warn when a function returns an `object::ConstructorRef`.
//!
//! Constructor references confer the authority to create an object; returning them can leak
//! privileged capability. Avoid returning them from functions.

use move_compiler_v2::external_checks::StacklessBytecodeChecker;
use move_model::{
    model::{GlobalEnv, Visibility},
    ty::Type,
};
use move_stackless_bytecode::function_target::FunctionTarget;

pub struct ReturnConstructorRef {}

impl StacklessBytecodeChecker for ReturnConstructorRef {
    fn get_name(&self) -> String {
        "return_constructor_ref".to_string()
    }

    fn check(&self, target: &FunctionTarget) {
        if target.visibility() != Visibility::Public {
            return;
        }
        
        let env = target.global_env();
        let returns_constructor_ref = target
            .get_return_types()
            .into_iter()
            .any(|ty| contains_constructor_ref(&ty, env));
        if returns_constructor_ref {
            let loc = target.func_env.get_result_type_loc();
            self.report(
                env,
                &loc,
                "Returning an `object::ConstructorRef` leaks authority; avoid returning constructor refs.",
            );
        }
    }
}

fn contains_constructor_ref(ty: &Type, env: &GlobalEnv) -> bool {
    match ty {
        Type::Reference(_, inner) => contains_constructor_ref(inner, env),
        Type::Vector(inner) => contains_constructor_ref(inner, env),
        Type::Tuple(elems) => elems.iter().any(|t| contains_constructor_ref(t, env)),
        Type::Struct(_, _, insts) => {
            if let Some((se, _)) = ty.get_struct(env) {
                let module_name = se.module_env.get_name().display(env).to_string();
                let struct_name = se.get_name().display(se.symbol_pool()).to_string();
                if module_name == "object" && struct_name == "ConstructorRef" {
                    return true;
                }
            }
            insts.iter().any(|t| contains_constructor_ref(t, env))
        },
        _ => false,
    }
}
