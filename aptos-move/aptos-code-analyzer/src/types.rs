// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use move_binary_format::{access::ModuleAccess, file_format::FunctionDefinition, CompiledModule};
use move_core_types::identifier::Identifier;

pub trait FunctionPass {
    fn run_on_function(&mut self, function_name: Identifier, function: &FunctionDefinition);
}

pub trait ModulePass {
    fn run_on_module(&mut self, module: &CompiledModule);
}

impl<P: FunctionPass> ModulePass for P {
    fn run_on_module(&mut self, module: &CompiledModule) {
        for function in module.function_defs() {
            let handle = module.function_handle_at(function.function);
            let function_name = module.identifier_at(handle.name).to_owned();
            self.run_on_function(function_name, function);
        }
    }
}
