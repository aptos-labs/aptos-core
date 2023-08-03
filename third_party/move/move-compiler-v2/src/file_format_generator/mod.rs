// Copyright © Aptos Foundation
// Parts of the project are originally copyright © Meta Platforms, Inc.
// SPDX-License-Identifier: Apache-2.0

mod function_generator;
mod module_generator;

use crate::file_format_generator::module_generator::ModuleContext;
use module_generator::ModuleGenerator;
use move_binary_format::{
    file_format as FF,
    file_format::{CompiledScript, FunctionDefinition, FunctionHandle},
    CompiledModule,
};
use move_model::model::GlobalEnv;
use move_stackless_bytecode::function_target_pipeline::FunctionTargetsHolder;

pub fn generate_file_format(
    env: &GlobalEnv,
    targets: &FunctionTargetsHolder,
) -> (Vec<FF::CompiledModule>, Vec<FF::CompiledScript>) {
    let ctx = ModuleContext { env, targets };
    let mut modules = vec![];
    let mut scripts = vec![];
    for module_env in ctx.env.get_modules() {
        if !module_env.is_target() {
            continue;
        }
        let (ff_module, main_handle) = ModuleGenerator::run(&ctx, &module_env);
        if module_env.is_script_module() {
            let CompiledModule {
                version,
                module_handles,
                struct_handles,
                function_handles,
                mut function_defs,
                function_instantiations,
                signatures,
                identifiers,
                address_identifiers,
                constant_pool,
                metadata,
                ..
            } = ff_module;
            if let Some(FunctionDefinition {
                code: Some(code), ..
            }) = function_defs.pop()
            {
                let FunctionHandle {
                    parameters,
                    type_parameters,
                    ..
                } = main_handle.expect("main handle defined");
                scripts.push(CompiledScript {
                    version,
                    module_handles,
                    struct_handles,
                    function_handles,
                    function_instantiations,
                    signatures,
                    identifiers,
                    address_identifiers,
                    constant_pool,
                    metadata,
                    code,
                    type_parameters,
                    parameters,
                })
            } else {
                ctx.internal_error(module_env.get_loc(), "inconsistent script module");
            }
        } else {
            modules.push(ff_module)
        }
    }
    (modules, scripts)
}

const MAX_MODULE_COUNT: usize = FF::TableIndex::MAX as usize;
const MAX_IDENTIFIER_COUNT: usize = FF::TableIndex::MAX as usize;
const MAX_ADDRESS_COUNT: usize = FF::TableIndex::MAX as usize;
const MAX_CONST_COUNT: usize = FF::TableIndex::MAX as usize;
const MAX_STRUCT_COUNT: usize = FF::TableIndex::MAX as usize;
const MAX_SIGNATURE_COUNT: usize = FF::TableIndex::MAX as usize;
const MAX_STRUCT_DEF_COUNT: usize = FF::TableIndex::MAX as usize;
const MAX_STRUCT_DEF_INST_COUNT: usize = FF::TableIndex::MAX as usize;
const MAX_FIELD_COUNT: usize = FF::TableIndex::MAX as usize;
const MAX_FIELD_INST_COUNT: usize = FF::TableIndex::MAX as usize;
const MAX_FUNCTION_COUNT: usize = FF::TableIndex::MAX as usize;
const MAX_FUNCTION_INST_COUNT: usize = FF::TableIndex::MAX as usize;
const MAX_FUNCTION_DEF_COUNT: usize = FF::TableIndex::MAX as usize;
const MAX_LOCAL_COUNT: usize = FF::LocalIndex::MAX as usize;
