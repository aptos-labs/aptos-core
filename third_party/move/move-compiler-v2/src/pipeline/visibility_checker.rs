// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

//! Implements a visibility checker, checking for visibility violations at function callsites.

use move_binary_format::file_format::Visibility;
use move_model::model::FunctionEnv;
use move_stackless_bytecode::{
    function_target::{FunctionData, FunctionTarget},
    function_target_pipeline::{FunctionTargetProcessor, FunctionTargetsHolder},
    stackless_bytecode::{Bytecode, Operation},
};
pub struct VisibilityChecker();

impl FunctionTargetProcessor for VisibilityChecker {
    fn process(
        &self,
        _targets: &mut FunctionTargetsHolder,
        fun_env: &FunctionEnv,
        data: FunctionData,
        _scc_opt: Option<&[FunctionEnv]>,
    ) -> FunctionData {
        if fun_env.is_native() {
            // We don't have to look inside native functions.
            return data;
        }
        let func_target = FunctionTarget::new(fun_env, &data);
        let global_env = func_target.global_env();
        let caller_mod_name = func_target.module_env().get_name();
        let caller_mod_id = func_target.module_env().get_id();
        for bytecode in func_target.get_bytecode() {
            if let Bytecode::Call(
                attr_id,
                _,
                Operation::Function(callee_mod_id, callee_fun_id, _),
                _,
                _,
            ) = bytecode
            {
                if *callee_mod_id == caller_mod_id {
                    // If the callee is in the same module as the caller, it is visible.
                    continue;
                }
                let callee_env = global_env.get_function(callee_mod_id.qualified(*callee_fun_id));
                match callee_env.visibility() {
                    Visibility::Public => {
                        // Public functions are visible from any caller.
                        continue;
                    },
                    _ if func_target.module_env().is_script_module() => {
                        // Only public functions are visible from scripts.
                        global_env.error(
                            &func_target.get_bytecode_loc(*attr_id),
                            &format!(
                                "function `{}` cannot be called from a script, because it is not public",
                                callee_env.get_full_name_with_address()
                            ),
                        );
                    },
                    Visibility::Friend => {
                        // Friend functions are visible from a caller whose module is a friend of the callee's module.
                        // For the purposes of this check, we assume friend declarations are valid.
                        // Validity of friend declarations should be checked elsewhere.
                        if !callee_env.module_env.has_friend(&caller_mod_id) {
                            global_env.error(
                                &func_target.get_bytecode_loc(*attr_id),
                                &format!(
                                    "friend function `{}` cannot be called here because `{}` is not a friend of `{}`",
                                    callee_env.get_full_name_with_address(),
                                    caller_mod_name.display_full(global_env),
                                    callee_env.module_env.get_full_name_str()
                                ),
                            );
                        }
                    },
                    Visibility::Private => {
                        // Private functions are not visible outside of the callee's module.
                        global_env.error(
                            &func_target.get_bytecode_loc(*attr_id),
                            &format!(
                                "function `{}` cannot be called here because it is private to module `{}`",
                                callee_env.get_full_name_with_address(),
                                callee_env.module_env.get_full_name_str()
                            ),
                        );
                    },
                }
            }
        }
        data
    }

    fn name(&self) -> String {
        "VisibilityChecker".to_owned()
    }
}
