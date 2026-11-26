// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Fuzz target: checks the production verifier's verdict on generated tests.
//!
//! The last input byte picks the config: below 128 every verifier rule is kept, otherwise one
//! rule may be broken on purpose. Each module and script is assembled, serialized, deserialized,
//! and verified (alone and against its dependencies) with `VerifierConfig::production_testing()`.
//! Findings:
//!   - a panic anywhere in that pipeline;
//!   - `move-asm` rejecting the text, or bytes changing across a serialization round trip;
//!   - an invariant-violation status from the verifier;
//!   - a rejection when no rule was broken, or acceptance of every unit when one was.
//!
//! `MASM_SMITH_STATS=<file>` appends one JSON line per input with each unit's verdict and opcode
//! counts.

#![no_main]

#[path = "common/stats.rs"]
mod stats;

use arbitrary::Unstructured;
use either::Either;
use libfuzzer_sys::fuzz_target;
use masm_smith::{
    assembler_options, generate, split_assembler_sections, stdlib_modules, Config, DimensionFlag,
};
use move_asm::assembler;
use move_binary_format::{
    deserializer::DeserializerConfig,
    errors::VMResult,
    file_format::{Bytecode, CompiledModule, CompiledScript},
    file_format_common::VERSION_MAX,
};
use move_bytecode_verifier::{dependencies, verifier, VerifierConfig};
use move_core_types::vm_status::StatusType;
use serde_json::{json, Value};
use std::collections::BTreeMap;

fuzz_target!(|data: &[u8]| {
    let Some((&selector, gen_bytes)) = data.split_last() else {
        return;
    };
    let mode = select_mode(selector);
    let cfg = config_for(mode, selector);
    let mut u = Unstructured::new(gen_bytes);
    let Ok(test) = generate(&mut u, &cfg) else {
        return;
    };
    let text = test.render();
    let verifier_config = VerifierConfig::production_testing();
    let deserializer_config = DeserializerConfig::default();
    let mut context: Vec<CompiledModule> = stdlib_modules().to_vec();
    let mut units: Option<Vec<Value>> = stats::STATS_PATH.as_ref().map(|_| Vec::new());
    let mut injected: Vec<DimensionFlag> = test.injected_violations.clone();
    let mut any_rejected = false;
    for (section_idx, section) in split_assembler_sections(&text).iter().enumerate() {
        let assembled = match assembler::assemble(&assembler_options(), section, context.iter()) {
            Ok(unit) => unit,
            Err(diags) => {
                let rendered = assembler::diag_to_string("verify_prod", section, diags);
                panic!(
                    "verify_prod: move-asm rejected generated text\n--diags--\n{rendered}\n--full--\n{text}"
                );
            },
        };
        match assembled {
            Either::Left(module) => {
                let bytes = serialize_module(&module, &text);
                match CompiledModule::deserialize_with_config(&bytes, &deserializer_config) {
                    Ok(back) => assert!(
                        serialize_module(&back, &text) == bytes,
                        "verify_prod: module bytes changed across a round trip\n--full--\n{text}"
                    ),
                    Err(err) => assert!(
                        mode.is_some(),
                        "verify_prod: all-valid module failed to deserialize: {err:?}\n--full--\n{text}"
                    ),
                }
                // Verify a mutated clone so the unmutated module stays usable as assembler
                // context for later sections.
                let mut for_verify = module.clone();
                let applied = test.post_process_module(section_idx, &mut for_verify);
                let post_processed = !applied.is_empty();
                injected.extend(applied);
                let verdict = verifier::verify_module_with_config(&verifier_config, &for_verify)
                    .and_then(|()| {
                        dependencies::verify_module(&verifier_config, &for_verify, context.iter())
                    });
                check_verdict(&verdict, mode, "module", &text);
                any_rejected |= verdict.is_err();
                if let Some(units) = units.as_mut() {
                    let code = for_verify
                        .function_defs
                        .iter()
                        .filter_map(|def| def.code.as_ref())
                        .flat_map(|unit| unit.code.iter());
                    units.push(unit_record("module", post_processed, &verdict, code));
                }
                context.push(module);
            },
            Either::Right(script) => {
                let bytes = serialize_script(&script, &text);
                match CompiledScript::deserialize_with_config(&bytes, &deserializer_config) {
                    Ok(back) => assert!(
                        serialize_script(&back, &text) == bytes,
                        "verify_prod: script bytes changed across a round trip\n--full--\n{text}"
                    ),
                    Err(err) => assert!(
                        mode.is_some(),
                        "verify_prod: all-valid script failed to deserialize: {err:?}\n--full--\n{text}"
                    ),
                }
                let verdict = verifier::verify_script_with_config(&verifier_config, &script)
                    .and_then(|()| {
                        dependencies::verify_script(&verifier_config, &script, context.iter())
                    });
                check_verdict(&verdict, mode, "script", &text);
                any_rejected |= verdict.is_err();
                if let Some(units) = units.as_mut() {
                    units.push(unit_record(
                        "script",
                        false,
                        &verdict,
                        script.code.code.iter(),
                    ));
                }
            },
        }
    }
    if !injected.is_empty() && !any_rejected {
        panic!(
            "verify_prod: production verifier accepted a program with injected violations {:?}\n--full--\n{text}",
            injected
        );
    }
    if let (Some(path), Some(units)) = (stats::STATS_PATH.as_ref(), units) {
        let record = json!({
            "mode": mode.map_or("valid", DimensionFlag::as_kebab),
            "injected": injected.iter().map(|dim| dim.as_kebab()).collect::<Vec<_>>(),
            "units": units,
        });
        stats::append_stats(path, &record);
    }
});

/// `None` means every dimension stays valid.
fn select_mode(selector: u8) -> Option<DimensionFlag> {
    if selector < 128 {
        return None;
    }
    let dims = DimensionFlag::ALL;
    Some(dims[selector as usize % dims.len()])
}

fn config_for(mode: Option<DimensionFlag>, selector: u8) -> Config {
    let mut cfg = Config::default();
    if let Some(dim) = mode {
        cfg.set_flag(dim, false);
        // Alternate sparse and frequent injection to exercise both mostly valid modules and
        // more violation sites. A violation still needs an eligible generation site.
        cfg.violation_injection_prob = if selector.is_multiple_of(2) { 32 } else { 255 };
    }
    cfg
}

fn serialize_module(module: &CompiledModule, text: &str) -> Vec<u8> {
    let mut bytes = Vec::new();
    module
        .serialize_for_version(Some(VERSION_MAX), &mut bytes)
        .unwrap_or_else(|err| {
            panic!("verify_prod: module serialization failed: {err:?}\n--full--\n{text}")
        });
    bytes
}

fn serialize_script(script: &CompiledScript, text: &str) -> Vec<u8> {
    let mut bytes = Vec::new();
    script
        .serialize_for_version(Some(VERSION_MAX), &mut bytes)
        .unwrap_or_else(|err| {
            panic!("verify_prod: script serialization failed: {err:?}\n--full--\n{text}")
        });
    bytes
}

fn check_verdict(verdict: &VMResult<()>, mode: Option<DimensionFlag>, kind: &str, text: &str) {
    let Err(err) = verdict else {
        return;
    };
    let status = err.major_status();
    if status.status_type() == StatusType::InvariantViolation {
        panic!("verify_prod: invariant violation {status:?} verifying {kind}: {err:?}\n--full--\n{text}");
    }
    if mode.is_none() {
        panic!(
            "verify_prod: production verifier rejected all-valid {kind}: {err:?}\n--full--\n{text}"
        );
    }
}

fn unit_record<'a>(
    kind: &str,
    post_processed: bool,
    verdict: &VMResult<()>,
    code: impl Iterator<Item = &'a Bytecode>,
) -> Value {
    let mut opcodes: BTreeMap<String, u64> = BTreeMap::new();
    for instr in code {
        let debug = format!("{instr:?}");
        let name = debug.split('(').next().unwrap_or(&debug).to_owned();
        *opcodes.entry(name).or_default() += 1;
    }
    let verdict = match verdict {
        Ok(()) => "OK".to_owned(),
        Err(err) => format!("{:?}", err.major_status()),
    };
    json!({
        "kind": kind,
        "post_processed": post_processed,
        "verdict": verdict,
        "opcodes": opcodes,
    })
}
