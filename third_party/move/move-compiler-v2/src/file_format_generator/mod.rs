// Copyright © Aptos Foundation
// Parts of the project are originally copyright © Meta Platforms, Inc.
// SPDX-License-Identifier: Apache-2.0

mod function_generator;
mod module_generator;

use crate::file_format_generator::module_generator::ModuleContext;
use module_generator::ModuleGenerator;
use move_binary_format::file_format as FF;
use move_model::model::GlobalEnv;
use move_stackless_bytecode::function_target_pipeline::FunctionTargetsHolder;

pub fn generate_file_format(
    env: &GlobalEnv,
    targets: &FunctionTargetsHolder,
) -> Vec<FF::CompiledModule> {
    let ctx = ModuleContext { env, targets };
    let mut result = vec![];
    for module_env in ctx.env.get_modules() {
        if !module_env.is_target() {
            continue;
        }
        result.push(ModuleGenerator::run(&ctx, &module_env));
    }
    result
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
