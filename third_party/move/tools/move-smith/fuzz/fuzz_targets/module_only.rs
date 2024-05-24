// Copyright (c) Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

#![no_main]

use arbitrary::Unstructured;
use libfuzzer_sys::fuzz_target;
use move_smith::{ast::Module, utils::compile_modules, CodeGenerator, MoveSmith};

fuzz_target!(|data: &[u8]| {
    let u = &mut Unstructured::new(data);
    let mut smith = MoveSmith::default();
    let module: Module = match smith.generate_module(u) {
        Ok(module) => module,
        Err(_) => return,
    };
    let code = module.emit_code();
    compile_modules(code);
});
