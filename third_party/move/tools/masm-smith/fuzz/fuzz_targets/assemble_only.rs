// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Fuzz target: generates tests that break every verifier rule it can and checks that `move-asm`
//! still accepts the text. Breaking a verifier rule must never break the syntax.

#![no_main]

use arbitrary::Unstructured;
use libfuzzer_sys::fuzz_target;
use masm_smith::{
    assembler_options, generate, split_assembler_sections, stdlib_modules, Config, DimensionFlag,
};
use move_asm::assembler;
use move_binary_format::file_format::CompiledModule;

fuzz_target!(|data: &[u8]| {
    let mut cfg = Config::default();
    for &dim in DimensionFlag::ALL {
        cfg.set_flag(dim, false);
    }
    let mut u = Unstructured::new(data);
    let Ok(test) = generate(&mut u, &cfg) else {
        return;
    };
    let text = test.render();
    let mut context: Vec<CompiledModule> = stdlib_modules().to_vec();
    for section in split_assembler_sections(&text) {
        let opts = assembler_options();
        match assembler::assemble(&opts, &section, context.iter()) {
            Ok(either::Either::Left(module)) => context.push(module),
            // Scripts assemble standalone; they never join the module context.
            Ok(either::Either::Right(_)) => {},
            Err(diags) => {
                let rendered = assembler::diag_to_string("assemble_only", &section, diags);
                panic!(
                    "assemble_only: rendered text didn't parse with all-flags-off config\n--section--\n{section}\n--diags--\n{rendered}\n--full--\n{text}"
                );
            },
        }
    }
});
