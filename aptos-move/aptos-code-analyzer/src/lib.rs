// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use move_binary_format::CompiledModule;

pub mod passes;

pub trait ModulePass {
    fn run_on_module(&mut self, module: &CompiledModule);
}
