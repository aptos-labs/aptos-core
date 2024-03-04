// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

//! A pass that checks for cyclic type instantiation like `f<T>` calls `f<S<T>>`.

use itertools::Itertools;
use move_model::{
    model::{FunId, FunctionEnv, Loc, ModuleEnv, QualifiedInstId},
    ty::Type,
};
use move_stackless_bytecode::{
    function_target::{FunctionData, FunctionTarget},
    function_target_pipeline::{FunctionTargetProcessor, FunctionTargetsHolder, FunctionVariant},
    stackless_bytecode::{Bytecode, Operation},
};

pub struct RecursiveInstantiationChecker {}

impl FunctionTargetProcessor for RecursiveInstantiationChecker {
    fn process(
        &self,
        targets: &mut FunctionTargetsHolder,
        fun_env: &FunctionEnv,
        data: FunctionData,
        _scc_opt: Option<&[FunctionEnv]>,
    ) -> FunctionData {
        if fun_env.is_native() {
            return data;
        }
        let ty_params = (0..fun_env.get_type_parameter_count())
            .map(|i| Type::TypeParameter(i as u16))
            .collect_vec();
        let callee = fun_env.get_qualified_id().instantiate(ty_params);
        let mut call_chain = Vec::new();
        check_recursive_instantiations_in(
            &fun_env.module_env,
            &targets,
            &data,
            &mut call_chain,
            callee,
            fun_env.get_loc(),
        );
        data
    }

    fn name(&self) -> String {
        "RecursiveInstantiationChecker".to_owned()
    }
}

/// Checks if the given type contains type parameters, and returns one if it does.
fn ty_contains_ty_parameter(ty: &Type) -> Option<u16> {
    match ty {
        Type::TypeParameter(i) => Some(*i),
        Type::Vector(ty) => ty_contains_ty_parameter(ty),
        Type::Struct(_, _, insts) => insts.iter().filter_map(ty_contains_ty_parameter).next(),
        Type::Primitive(_) => None,
        _ => panic!("ICE: {:?} used as a type parameter", ty),
    }
}

/// Checks if the given type properly contains type parameters, and returns one if it does.
fn ty_properly_contains_ty_parameter(ty: &Type) -> Option<u16> {
    match ty {
        Type::Vector(ty) => ty_contains_ty_parameter(ty),
        Type::Struct(_, _, insts) => insts.iter().filter_map(ty_contains_ty_parameter).next(),
        Type::Primitive(_) | Type::TypeParameter(_) => None,
        _ => panic!("ICE: {:?} used as a type parameter", ty),
    }
}

/// Returns the display name of a function call with type parameters but without arguments
fn display_call(module_env: &ModuleEnv, call: QualifiedInstId<FunId>) -> String {
    let fun_env = module_env.get_function(call.id);
    let fun_name = fun_env.get_name_str();
    let type_disply_ctx = fun_env.get_type_display_ctx();
    format!(
        "{}<{}>",
        fun_name,
        call.inst
            .iter()
            .map(|ty| ty.display(&type_disply_ctx).to_string())
            .join(", ")
    )
}

/// Add diagnostics when a cyclic instantiation of type parameter `ty_param` is found of the root caller of `callers_chain`.
fn report_recursive_instantiation(
    module_env: &ModuleEnv,
    callers_chain: &mut Vec<(Loc, QualifiedInstId<FunId>)>,
    callee: QualifiedInstId<FunId>,
    _callee_loc: Loc,
    ty_param: u16,
) {
    let root_loc = callers_chain[0].0.clone();
    let root_caller = callers_chain[0].1.id.clone();
    let labels = (1..callers_chain.len())
        .map(|i| {
            let (caller_loc, caller) = &callers_chain[i];
            let callee = if i != callers_chain.len() - 1 {
                &callers_chain[i + 1].1
            } else {
                &callee
            };
            (
                caller_loc.clone(),
                format!(
                    "{:?} calls {:?}",
                    display_call(module_env, caller.clone()),
                    display_call(module_env, callee.clone())
                ),
            )
        })
        .collect_vec();
    let ty_param_display = Type::TypeParameter(ty_param)
        .display(&module_env.get_function(callee.id).get_type_display_ctx())
        .to_string();
    module_env.env.error_with_labels(
        &root_loc,
        &format!("Cyclic type instantiations found for type parameter `{}` of `{}`", ty_param_display, module_env.get_function(root_caller).get_simple_name_string()),
        labels,
    );
}

/// Checks if calling `callee` forms a recursive instantiation with respect to `callers_chain`,
/// - `module_env`: the module environment where the calls happen
/// - `root_caller_data`: the function data of the root caller, which is removed from the target holder during processing
/// - `callers_chain`, `callee`: `callers_chain[i]` calls `callers_chain[i + 1]` and the last element calls `callee`.
/// - `callee_loc`: location of `callee`
/// Requires: the root call is instantiated with `Type::Parameter(_)`
fn check_recursive_instantiations_in(
    module_env: &ModuleEnv,
    function_targt_holder: &FunctionTargetsHolder,
    root_caller_data: &FunctionData,
    callers_chain: &mut Vec<(Loc, QualifiedInstId<FunId>)>,
    callee: QualifiedInstId<FunId>,
    callee_loc: Loc,
) -> bool {
    if callee.module_id != module_env.get_id() {
        // because we don't have cyclic module dependencies
        return true;
    }
    for (_, caller) in callers_chain.iter() {
        if caller.to_qualified_id() == callee.to_qualified_id() {
            // the root caller is the function we're checking for
            let (_, checking_for) = &callers_chain[0];
            if checking_for.to_qualified_id() != callee.to_qualified_id() {
                // check and report diagnostics when `callee` is checked
                return true;
            }
            if let Some(ty_param) = callee
                .inst
                .iter()
                .filter_map(ty_properly_contains_ty_parameter)
                .next()
            {
                report_recursive_instantiation(
                    &module_env,
                    callers_chain,
                    callee,
                    callee_loc,
                    ty_param,
                );
                return false;
            }
        }
    }
    // recursively checks for callees of `callee`
    check_callees_of(
        module_env,
        function_targt_holder,
        root_caller_data,
        callers_chain,
        callee,
        callee_loc,
    )
}

/// Checks if calling callees of `caller` can result in recursive instantiations with respect to `callers_chain`
/// Parameter similar as `check_recursive_instantiations_in`
fn check_callees_of(
    module_env: &ModuleEnv,
    function_targt_holder: &FunctionTargetsHolder,
    root_caller_data: &FunctionData,
    callers_chain: &mut Vec<(Loc, QualifiedInstId<FunId>)>,
    caller: QualifiedInstId<FunId>,
    caller_loc: Loc,
) -> bool {
    if caller.module_id != module_env.get_id() {
        // because we don't have cyclic module dependencies
        return true;
    }
    let caller_data = if callers_chain.is_empty() {
        root_caller_data
    } else {
        function_targt_holder
            .get_data(&caller.to_qualified_id(), &FunctionVariant::Baseline)
            .expect(&format!("function data of {:?}", caller.to_qualified_id()))
    };
    let caller_env = module_env.get_function(caller.id);
    let caller_target = FunctionTarget::new(&caller_env, caller_data);
    for instr in &caller_data.code {
        if let Bytecode::Call(callee_attr_id, _, Operation::Function(mid, fid, ty_params), _, _) =
            instr
        {
            let ty_params_instantiated = Type::instantiate_vec(ty_params.clone(), &caller.inst);
            let callee = mid.qualified_inst(*fid, ty_params_instantiated);
            let callee_loc = caller_target.get_bytecode_loc(*callee_attr_id);
            callers_chain.push((caller_loc.clone(), caller.clone()));
            if !check_recursive_instantiations_in(
                module_env,
                function_targt_holder,
                root_caller_data,
                callers_chain,
                callee,
                callee_loc,
            ) {
                callers_chain.pop();
                return false;
            }
            callers_chain.pop();
        }
    }
    true
}
