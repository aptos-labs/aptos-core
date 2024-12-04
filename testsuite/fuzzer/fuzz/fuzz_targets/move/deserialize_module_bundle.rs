// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

#![no_main]
use arbitrary::Arbitrary;
use libfuzzer_sys::{fuzz_target, Corpus};
// mod utils;
use move_binary_format::{deserializer::DeserializerConfig, CompiledModule, file_format::CompiledScript};


#[derive(Arbitrary, Debug)]
struct FuzzData {
    flip: bool,
    module: CompiledModule,
    script: CompiledScript,
}

fuzz_target!(|fuzz_data: FuzzData| -> Corpus {
    run_case(&fuzz_data)
});

fn run_case(data: &FuzzData) -> Corpus {
    if data.flip {
        run_case_module(data)
    } else {
        run_case_script(data)
    }
}

fn run_case_module(data: &FuzzData) -> Corpus {
    let mut module_code  = vec![];
    if data.module.serialize(&mut module_code).is_err() {
        return Corpus::Reject;
    }
    match CompiledModule::deserialize_with_config(&module_code, &DeserializerConfig::default()) {
        Ok(_) => Corpus::Keep,
        Err(_) => Corpus::Reject,
    }
}

fn run_case_script(data: &FuzzData) -> Corpus {
    let mut script_code  = vec![];
    if data.script.serialize(&mut script_code).is_err() {
        return Corpus::Reject;
    }
    match CompiledScript::deserialize_with_config(&script_code, &DeserializerConfig::default()) {
        Ok(_) => Corpus::Keep,
        Err(_) => Corpus::Reject,
    }
}
