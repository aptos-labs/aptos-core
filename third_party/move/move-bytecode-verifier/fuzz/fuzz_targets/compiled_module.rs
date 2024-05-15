// Copyright (c) The Move Contributors
// SPDX-License-Identifier: Apache-2.0

#![no_main]
use libfuzzer_sys::fuzz_target;
use move_binary_format::file_format::CompiledModule;
use move_bytecode_verifier::VerifierConfig;

fuzz_target!(|module: CompiledModule| {
    let _ = move_bytecode_verifier::verify_module(&VerifierConfig::default(), &module);
});
