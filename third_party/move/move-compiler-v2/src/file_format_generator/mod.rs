// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

mod function_generator;
pub mod module_generator;
mod peephole_optimizer;

use crate::{file_format_generator::module_generator::ModuleContext, options::Options, Experiment};
use legacy_move_compiler::compiled_unit as CU;
use module_generator::ModuleGenerator;
use move_binary_format::{file_format as FF, internals::ModuleIndex, module_script_conversion};
use move_command_line_common::{address::NumericalAddress, parser::NumberFormat};
use move_model::{ast::ModuleName, model::GlobalEnv};
use move_stackless_bytecode::function_target_pipeline::FunctionTargetsHolder;
use move_symbol_pool::Symbol;
use std::collections::BTreeMap;

pub fn generate_file_format(
    env: &mut GlobalEnv,
    targets: &FunctionTargetsHolder,
) -> Vec<CU::CompiledUnit> {
    let ctx = ModuleContext { env, targets };
    let mut result = vec![];
    let options = env
        .get_extension::<Options>()
        .expect("Options is available");
    let compile_test_code = options.compile_test_code;
    let mut module_data = BTreeMap::new();
    let mut script_module_data = BTreeMap::new();
    let mut script_index = 0;
    for module_env in ctx.env.get_modules() {
        if !module_env.is_target() {
            continue;
        }
        assert!(compile_test_code || !module_env.is_test_only());
        let (ff_module, source_map, main_handle) = ModuleGenerator::run(&ctx, &module_env);
        if module_env.is_script_module() {
            let FF::CompiledModule {
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
            if let Some(FF::FunctionDefinition {
                code: Some(code), ..
            }) = function_defs.pop()
            {
                let FF::FunctionHandle {
                    parameters,
                    type_parameters,
                    name,
                    ..
                } = main_handle.expect("main handle defined");
                let name = Symbol::from(identifiers[name.into_index()].as_str());
                let script = FF::CompiledScript {
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
                    // TODO(#16278): support rac
                    access_specifiers: None,
                };
                if options.experiment_on(Experiment::ATTACH_COMPILED_MODULE) {
                    let module_name =
                        ModuleName::pseudo_script_name(env.symbol_pool(), script_index);
                    script_index += 1;
                    let module = module_script_conversion::script_into_module(
                        script.clone(),
                        &module_name.name().display(env.symbol_pool()).to_string(),
                    );
                    script_module_data.insert(module_env.get_id(), (module, source_map.clone()));
                }
                result.push(CU::CompiledUnitEnum::Script(CU::NamedCompiledScript {
                    package_name: None,
                    name,
                    script,
                    source_map,
                }))
            } else {
                ctx.internal_error(module_env.get_loc(), "inconsistent script module");
            }
        } else {
            if options.experiment_on(Experiment::ATTACH_COMPILED_MODULE) {
                module_data.insert(module_env.get_id(), (ff_module.clone(), source_map.clone()));
            }
            result.push(CU::CompiledUnitEnum::Module(CU::NamedCompiledModule {
                package_name: None,
                address: NumericalAddress::new(
                    module_env.get_name().addr().expect_numerical().into_bytes(),
                    NumberFormat::Hex,
                ),
                name: Symbol::from(ctx.symbol_to_str(module_env.get_name().name())),
                module: ff_module,
                source_map,
            }));
        }
    }
    if options.experiment_on(Experiment::ATTACH_COMPILED_MODULE) {
        for (id, (m, map)) in module_data {
            env.attach_compiled_module(id, m, map)
        }
        for (id, (m, map)) in script_module_data {
            env.attach_compiled_module(id, m, map)
        }
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
const MAX_STRUCT_VARIANT_COUNT: usize = FF::TableIndex::MAX as usize;
const MAX_STRUCT_VARIANT_INST_COUNT: usize = FF::TableIndex::MAX as usize;
const MAX_FIELD_COUNT: usize = FF::TableIndex::MAX as usize;
const MAX_FIELD_INST_COUNT: usize = FF::TableIndex::MAX as usize;
const MAX_FUNCTION_COUNT: usize = FF::TableIndex::MAX as usize;
const MAX_FUNCTION_INST_COUNT: usize = FF::TableIndex::MAX as usize;
const MAX_FUNCTION_DEF_COUNT: usize = FF::TableIndex::MAX as usize;
pub const MAX_LOCAL_COUNT: usize = FF::LocalIndex::MAX as usize;
