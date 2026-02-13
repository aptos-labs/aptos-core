// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Support for compiling scripts and modules in tests.

use aptos_types::transaction::{Module, Script};
use move_asm::assembler;
use move_binary_format::CompiledModule;

/// Compile the provided masm code into a blob which can be used as the code to be published
/// (a Module).
pub fn compile_module(code: &str) -> (CompiledModule, Module) {
    let framework_modules = aptos_cached_packages::head_release_bundle().compiled_modules();
    let options = assembler::Options::default();
    let compiled_module = assembler::assemble(&options, code, framework_modules.iter())
        .expect("Module assembly failed")
        .left()
        .expect("Expected module, got script");
    let mut module_bytes = vec![];
    compiled_module
        .serialize(&mut module_bytes)
        .expect("Module serialization failed");
    let module = Module::new(module_bytes);
    (compiled_module, module)
}

/// Compile the provided masm code into a blob which can be used as the code to be executed
/// (a Script).
pub fn compile_script(code: &str, extra_deps: Vec<CompiledModule>) -> Script {
    let mut framework_modules = aptos_cached_packages::head_release_bundle().compiled_modules();
    framework_modules.extend(extra_deps);
    let options = assembler::Options::default();
    let compiled_script = assembler::assemble(&options, code, framework_modules.iter())
        .expect("Script assembly failed")
        .right()
        .expect("Expected script, got module");
    let mut script_bytes = vec![];
    compiled_script
        .serialize(&mut script_bytes)
        .expect("Script serialization failed");
    Script::new(script_bytes, vec![], vec![])
}
