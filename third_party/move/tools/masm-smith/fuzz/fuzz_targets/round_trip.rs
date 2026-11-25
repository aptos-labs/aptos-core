// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Fuzz target: generates a test with the default config, assembles each module and script, and
//! runs the verifier on each. Any assembler or verifier error, panic, or crash is a finding.

#![no_main]

use arbitrary::Unstructured;
use either::Either;
use libfuzzer_sys::fuzz_target;
use masm_smith::{assembler_options, generate, split_assembler_sections, stdlib_modules, Config};
use move_asm::assembler;
use move_binary_format::file_format::CompiledModule;
use move_bytecode_verifier::verifier;

fuzz_target!(|data: &[u8]| {
    let mut u = Unstructured::new(data);
    let test = match generate(&mut u, &Config::default()) {
        Ok(t) => t,
        Err(_) => return,
    };
    let text = test.render();
    let mut context: Vec<CompiledModule> = stdlib_modules().to_vec();
    for (section_idx, section) in split_assembler_sections(&text).iter().enumerate() {
        let opts = assembler_options();
        match assembler::assemble(&opts, section, context.iter()) {
            Ok(Either::Left(mut module)) => {
                test.post_process_module(section_idx, &mut module);
                if let Err(e) = verifier::verify_module(&module) {
                    panic!(
                        "round_trip: verifier rejected default-config module: {e:?}\n--section--\n{section}\n--full--\n{text}"
                    );
                }
                context.push(module);
            },
            Ok(Either::Right(script)) => {
                if let Err(e) = verifier::verify_script(&script) {
                    panic!(
                        "round_trip: verifier rejected default-config script: {e:?}\n--section--\n{section}\n--full--\n{text}"
                    );
                }
            },
            Err(diags) => {
                let rendered = assembler::diag_to_string("round_trip", section, diags);
                panic!(
                    "round_trip: move-asm failed to assemble default-config text\n--section--\n{section}\n--diags--\n{rendered}\n--full--\n{text}"
                );
            },
        }
    }
});
