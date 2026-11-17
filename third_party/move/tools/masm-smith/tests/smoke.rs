// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Checks that tests generated with the default config assemble and pass the verifier, for the
//! seeds in `SEEDS` and a random sweep. Under the default config, a verifier rejection is a
//! generator bug.

use arbitrary::Unstructured;
use either::Either;
use masm_smith::{
    assembler_options, generate, split_assembler_sections, stdlib_modules, Config, MasmTest,
};
use move_asm::assembler;
use move_binary_format::file_format::CompiledModule;
use move_bytecode_verifier::{dependencies, verifier, VerifierConfig};

const SEEDS: &[&[u8]] = &[
    &[],
    &[0],
    &[0xFF; 32],
    &[0xDE, 0xAD, 0xBE, 0xEF, 0x01, 0x02, 0x03, 0x04],
    &[
        0x10, 0x20, 0x30, 0x40, 0x50, 0x60, 0x70, 0x80, 0x90, 0xA0, 0xB0, 0xC0, 0xD0, 0xE0, 0xF0,
    ],
];

/// Walk the rendered `.masm` sections in order, assembling each with the previously-assembled
/// modules as context, then verifying. Mirrors the transactional-test runner's behavior so
/// generators that produce cross-module references can be tested end to end.
fn assemble_and_verify_test(label: &str, test: &MasmTest) {
    let text = test.render();
    let mut context: Vec<CompiledModule> = stdlib_modules().to_vec();
    for (section_idx, section) in split_assembler_sections(&text).iter().enumerate() {
        let opts = assembler_options();
        match assembler::assemble(&opts, section, context.iter()) {
            Ok(Either::Left(mut module)) => {
                test.post_process_module(section_idx, &mut module);
                if let Err(err) = verifier::verify_module(&module) {
                    panic!(
                        "{label} section #{section_idx}: module rejected: {:?}\n---section---\n{}\n---full text---\n{}",
                        err, section, text
                    );
                }
                context.push(module);
            },
            Ok(Either::Right(script)) => {
                if let Err(err) = verifier::verify_script(&script) {
                    panic!(
                        "{label} section #{section_idx}: script rejected: {:?}\n---section---\n{}\n---full text---\n{}",
                        err, section, text
                    );
                }
            },
            Err(diags) => {
                let rendered = assembler::diag_to_string(label, section, diags);
                panic!(
                    "{label} section #{section_idx}: move-asm failed to assemble\n---section---\n{}\n---diags---\n{}\n---full text---\n{}",
                    section, rendered, text
                );
            },
        }
    }
}

#[test]
fn stress_sweep_assembles_and_verifies() {
    let iterations: u32 = std::env::var("MASM_SMITH_STRESS_ITERS")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(64);
    for i in 0..iterations {
        let mut seed = Vec::with_capacity(128);
        for b in 0u32..128 {
            seed.push(((b.wrapping_mul(31).wrapping_add(i.wrapping_mul(17))) ^ 0xA5) as u8);
        }
        let mut u = Unstructured::new(&seed);
        let test = match generate(&mut u, &Config::default()) {
            Ok(t) => t,
            Err(masm_smith::Error::OutOfInput) => continue,
            Err(other) => panic!("iteration #{i} generation error: {other}"),
        };
        assemble_and_verify_test(&format!("stress#{i}"), &test);
    }
}

#[test]
fn each_seed_assembles_and_verifies() {
    for (i, seed) in SEEDS.iter().enumerate() {
        let mut u = Unstructured::new(seed);
        let test = match generate(&mut u, &Config::default()) {
            Ok(t) => t,
            Err(masm_smith::Error::OutOfInput) => continue,
            Err(other) => panic!("seed #{} generation error: {other}", i),
        };
        let text = test.render();
        let sections = split_assembler_sections(&text);
        assert!(
            !sections.is_empty(),
            "seed #{i}: render produced no assembler sections\n---text---\n{text}"
        );
        assemble_and_verify_test(&format!("seed#{i}"), &test);
    }
}

#[test]
fn stdlib_calls_assemble_verify_and_link() {
    let mut stdlib_calls = 0usize;
    for i in 0u32..256 {
        let seed: Vec<u8> = (0u32..256)
            .map(|offset| (i.wrapping_mul(2654435761).wrapping_add(offset * 40503) >> 7) as u8)
            .collect();
        let mut u = Unstructured::new(&seed);
        let Ok(test) = generate(&mut u, &Config::default()) else {
            continue;
        };
        let text = test.render();
        stdlib_calls += text.matches("call 0x1::").count();
        let mut context: Vec<CompiledModule> = stdlib_modules().to_vec();
        for section in split_assembler_sections(&text) {
            let unit = assembler::assemble(&assembler_options(), &section, context.iter())
                .unwrap_or_else(|_| panic!("seed #{i}: assembly failed\n{section}"));
            match unit {
                Either::Left(module) => {
                    verifier::verify_module(&module)
                        .and_then(|()| {
                            dependencies::verify_module(
                                &VerifierConfig::default(),
                                &module,
                                context.iter(),
                            )
                        })
                        .unwrap_or_else(|err| panic!("seed #{i}: {err:?}\n{section}"));
                    context.push(module);
                },
                Either::Right(script) => {
                    verifier::verify_script(&script)
                        .and_then(|()| {
                            dependencies::verify_script(
                                &VerifierConfig::default(),
                                &script,
                                context.iter(),
                            )
                        })
                        .unwrap_or_else(|err| panic!("seed #{i}: {err:?}\n{section}"));
                },
            }
        }
    }
    assert!(stdlib_calls > 0, "no stdlib calls emitted across the sweep");
}
