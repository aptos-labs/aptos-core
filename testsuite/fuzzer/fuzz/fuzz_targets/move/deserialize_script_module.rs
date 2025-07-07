// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

#![no_main]
use arbitrary::Arbitrary;
use libfuzzer_sys::{fuzz_target, Corpus};
// mod utils;
use move_binary_format::{
    deserializer::DeserializerConfig,
    file_format::{CompiledModule, CompiledScript},
    file_format_common::VERSION_MAX,
};
use move_vm_types::natives::function::StatusCode;

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

fn run_case_raw(raw_data: &Vec<u8>) -> Corpus {
    if let Ok(m) = CompiledModule::deserialize_with_config(raw_data, &DeserializerConfig::default())
    {
        let mut module_code = vec![];
        m.serialize_for_version(Some(VERSION_MAX), &mut module_code)
            .unwrap();
        assert_eq!(*raw_data, module_code);
        return Corpus::Keep;
    }

    if let Ok(s) = CompiledScript::deserialize_with_config(raw_data, &DeserializerConfig::default())
    {
        let mut script_code = vec![];
        s.serialize_for_version(Some(VERSION_MAX), &mut script_code)
            .unwrap();
        assert_eq!(*raw_data, script_code);
        return Corpus::Keep;
    }

    Corpus::Reject
}
