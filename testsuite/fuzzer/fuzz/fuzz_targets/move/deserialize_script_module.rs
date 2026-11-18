// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

#![no_main]
use arbitrary::Arbitrary;
use libfuzzer_sys::{fuzz_target, Corpus};
// mod utils;
use move_binary_format::{
    deserializer::DeserializerConfig,
    errors::BinaryLoaderResult,
    file_format::{CompiledModule, CompiledScript},
    file_format_common::VERSION_MAX,
};
use move_vm_types::natives::function::StatusCode;
use std::fmt::Debug;

#[derive(Arbitrary, Debug)]
enum ExecVariant {
    Module(CompiledModule),
    Script(CompiledScript),
    Raw(Vec<u8>),
}

fuzz_target!(|fuzz_data: ExecVariant| -> Corpus { run_case(&fuzz_data) });

fn run_case(data: &ExecVariant) -> Corpus {
    match data {
        ExecVariant::Module(module) => run_case_module(module),
        ExecVariant::Script(script) => run_case_script(script),
        ExecVariant::Raw(raw_data) => run_case_raw(raw_data),
    }
}

fn run_case_module(module: &CompiledModule) -> Corpus {
    let mut module_code = vec![];
    if module
        .serialize_for_version(Some(VERSION_MAX), &mut module_code)
        .is_err()
    {
        return Corpus::Reject;
    }

    match CompiledModule::deserialize_no_check_bounds(&module_code) {
        Ok(mut m) => {
            m.version = module.version;
            assert_eq!(*module, m);
            Corpus::Keep
        },
        Err(e) => {
            if e.major_status() != StatusCode::MALFORMED {
                panic!("Failed to deserialize module after serialization\nCompiledModule:\n{:?}\nSerialized:{:?}", module, module_code);
            }
            Corpus::Reject
        },
    }
}

fn run_case_script(script: &CompiledScript) -> Corpus {
    let mut script_code = vec![];
    if script
        .serialize_for_version(Some(VERSION_MAX), &mut script_code)
        .is_err()
    {
        return Corpus::Reject;
    }

    match CompiledScript::deserialize_no_check_bounds(&script_code) {
        Ok(mut s) => {
            s.version = script.version;
            assert_eq!(*script, s);
            Corpus::Keep
        },
        Err(e) => {
            if e.major_status() != StatusCode::MALFORMED {
                panic!("Failed to deserialize script after serialization\nCompiledScript:\n{:?}\nSerialized:{:?}", script, script_code);
            }
            Corpus::Reject
        },
    }
}

// The binary format admits several encodings of the same module or script (e.g., trailing bytes,
// table order, version mask bits), so the round trip is checked on the deserialized value, not on
// bytes.
fn run_case_raw(raw_data: &[u8]) -> Corpus {
    let config = DeserializerConfig::default();
    let strict_config = DeserializerConfig {
        reject_trailing_bytes: true,
        ..config.clone()
    };

    let module = CompiledModule::deserialize_with_config(raw_data, &config);
    check_strict_against_lenient(
        &module,
        &CompiledModule::deserialize_with_config(raw_data, &strict_config),
    );
    if let Ok(module) = module {
        let mut module_code = vec![];
        module
            .serialize_for_version(Some(VERSION_MAX), &mut module_code)
            .unwrap();
        // The serializer must not emit trailing bytes, so its output is deserialized strictly.
        let mut reserialized =
            CompiledModule::deserialize_with_config(&module_code, &strict_config)
                .expect("Serialized module must deserialize");
        reserialized.version = module.version;
        assert_eq!(module, reserialized);
        return Corpus::Keep;
    }

    let script = CompiledScript::deserialize_with_config(raw_data, &config);
    check_strict_against_lenient(
        &script,
        &CompiledScript::deserialize_with_config(raw_data, &strict_config),
    );
    if let Ok(script) = script {
        let mut script_code = vec![];
        script
            .serialize_for_version(Some(VERSION_MAX), &mut script_code)
            .unwrap();
        let mut reserialized =
            CompiledScript::deserialize_with_config(&script_code, &strict_config)
                .expect("Serialized script must deserialize");
        reserialized.version = script.version;
        assert_eq!(script, reserialized);
        return Corpus::Keep;
    }

    Corpus::Reject
}

/// Rejecting trailing bytes may only turn a successful deserialization into a `MALFORMED` error.
fn check_strict_against_lenient<T: PartialEq + Debug>(
    lenient: &BinaryLoaderResult<T>,
    strict: &BinaryLoaderResult<T>,
) {
    match (lenient, strict) {
        (Ok(lenient), Ok(strict)) => assert_eq!(lenient, strict),
        (Ok(_), Err(err)) => assert_eq!(err.major_status(), StatusCode::MALFORMED),
        (Err(_), Ok(_)) => panic!("Strict deserialization accepted a binary rejected leniently"),
        // Errors may differ: the trailing-bytes check can fail before bounds checks run.
        (Err(_), Err(_)) => {},
    }
}
