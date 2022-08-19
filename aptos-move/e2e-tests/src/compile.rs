// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

//! Support for compiling scripts and modules in tests.

use move_deps::move_ir_compiler::Compiler;

use aptos_types::transaction::{Module, Script};
use move_deps::move_binary_format::CompiledModule;

/// Compile the provided Move code into a blob which can be used as the code to be published
/// (a Module).
pub fn compile_module(code: &str) -> (CompiledModule, Module) {
    let framework_modules = cached_packages::head_release_bundle().compiled_modules();
    let compiled_module = Compiler {
        deps: framework_modules.iter().collect(),
    }
    .into_compiled_module(code)
    .expect("Module compilation failed");
    let module = Module::new(
        Compiler {
            deps: framework_modules.iter().collect(),
        }
        .into_module_blob(code)
        .expect("Module compilation failed"),
    );
    (compiled_module, module)
}

/// Compile the provided Move code into a blob which can be used as the code to be executed
/// (a Script).
pub fn compile_script(code: &str, mut extra_deps: Vec<CompiledModule>) -> Script {
    let mut framework_modules = cached_packages::head_release_bundle().compiled_modules();
    framework_modules.append(&mut extra_deps);
    let compiler = Compiler {
        deps: framework_modules.iter().collect(),
    };
    Script::new(
        compiler
            .into_script_blob(code)
            .expect("Script compilation failed"),
        vec![],
        vec![],
    )
}
