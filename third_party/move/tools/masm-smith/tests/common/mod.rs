// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Helpers shared by the integration tests that generate VM-target programs.

#![allow(dead_code)]

use arbitrary::Unstructured;
use either::Either;
use masm_smith::{
    assembler_options, generate, split_assembler_sections, stdlib_modules, Config, MasmTest,
};
use move_asm::assembler;
use move_binary_format::file_format::CompiledModule;
use move_bytecode_verifier::{dependencies, verifier, VerifierConfig};
use move_transactional_test_runner::vm_test_harness::{run_test_with_config, TestRunConfig};

/// Assembles every section and verifies it, with linking checks against the latest version of
/// every earlier module and the stdlib. Panics with the rendered text on failure.
pub fn assert_verifies(test: &MasmTest, label: &str) {
    let config = VerifierConfig::production_testing();
    let text = test.render();
    let mut context: Vec<CompiledModule> = stdlib_modules().to_vec();
    for section in split_assembler_sections(&text) {
        let unit = assembler::assemble(&assembler_options(), &section, context.iter())
            .unwrap_or_else(|_| panic!("{label}: assembly failed\n{text}"));
        let verdict = match &unit {
            Either::Left(module) => verifier::verify_module_with_config(&config, module)
                .and_then(|()| dependencies::verify_module(&config, module, context.iter())),
            Either::Right(script) => verifier::verify_script_with_config(&config, script)
                .and_then(|()| dependencies::verify_script(&config, script, context.iter())),
        };
        if let Err(err) = verdict {
            panic!("{label}: {err:?}\n{text}");
        }
        if let Either::Left(module) = unit {
            context.retain(|existing| existing.self_id() != module.self_id());
            context.push(module);
        }
    }
}

/// Deterministic pseudorandom bytes for a test seed.
pub fn random_bytes(seed: u32, len: usize) -> Vec<u8> {
    masm_smith::seeded_bytes(u64::from(seed) ^ 0x9E37_79B9_7F4A_7C15, len)
}

pub fn valid_program(seed: u32) -> Option<MasmTest> {
    let test = generate(
        &mut Unstructured::new(&random_bytes(seed, 4096)),
        &Config::vm_fuzzing(),
    )
    .ok()?;
    (test.injected_violations.is_empty() && test.post_process.is_none()).then_some(test)
}

/// Runs `test` with paranoid checks and the effects report, returning the captured output.
pub fn run_output(test: &MasmTest, label: &str) -> String {
    unsafe { std::env::set_var("UPDATE_BASELINE", "1") };
    let dir = tempfile::tempdir().expect("temp dir");
    let path = dir.path().join("test.masm");
    std::fs::write(&path, test.render()).expect("write test");
    let config = TestRunConfig::default()
        .with_runtime_ref_checks()
        .with_effects_report()
        .with_serde_round_trip();
    run_test_with_config(config, &path)
        .unwrap_or_else(|err| panic!("{label}: runner error {err}\n{}", test.render()));
    std::fs::read_to_string(path.with_extension("exp")).expect("read output")
}
