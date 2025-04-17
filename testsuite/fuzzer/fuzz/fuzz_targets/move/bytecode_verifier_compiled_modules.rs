// Copyright (c) The Move Contributors
// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

#![no_main]
use libfuzzer_sys::{fuzz_target, Corpus};
use move_binary_format::errors::VMError;
use move_bytecode_verifier::VerifierConfig;
use move_core_types::vm_status::StatusType;
mod utils;
use utils::vm::RunnableState;

fn check_for_invariant_violation_vmerror(e: VMError) {
    if e.status_type() == StatusType::InvariantViolation
        // ignore known false positive
        && !e
            .message()
            .is_some_and(|m| m.starts_with("too many type parameters/arguments in the program"))
    {
        panic!("invariant violation {:?}", e);
    }
}

fuzz_target!(|fuzz_data: RunnableState| -> Corpus {
    let verifier_config = VerifierConfig::production();

    for m in fuzz_data.dep_modules.iter() {
        if let Err(e) = move_bytecode_verifier::verify_module_with_config(&verifier_config, m) {
            //check_for_invariant_violation_vmerror(e);
            return Corpus::Keep;
        }
    }
    Corpus::Keep
});
