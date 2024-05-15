// Copyright (c) The Diem Core Contributors
// Copyright (c) The Move Contributors
// SPDX-License-Identifier: Apache-2.0

use move_binary_format::{errors::VMResult, CompiledModule};
use move_bytecode_verifier::{verify_module, VerifierConfig};
use std::time::Instant;

pub mod ability_field_requirements_tests;
pub mod binary_samples;
pub mod bounds_tests;
pub mod catch_unwind;
pub mod code_unit_tests;
pub mod constants_tests;
pub mod control_flow_tests;
pub mod dependencies_tests;
pub mod duplication_tests;
pub mod generic_ops_tests;
pub mod large_type_test;
pub mod limit_tests;
pub mod locals;
pub mod loop_summary_tests;
pub mod many_back_edges;
pub mod multi_pass_tests;
pub mod negative_stack_size_tests;
pub mod reference_safety_tests;
pub mod signature_tests;
pub mod struct_defs_tests;
pub mod vec_pack_tests;

pub(crate) fn verify_module_and_measure_verification_time(
    name: &str,
    config: &VerifierConfig,
    module: &CompiledModule,
) -> VMResult<()> {
    const MAX_MODULE_SIZE: usize = 65355;
    let mut bytes = vec![];
    module.serialize(&mut bytes).unwrap();
    let now = Instant::now();
    let result = verify_module(config, module);
    eprintln!(
        "--> {}: verification time: {:.3}ms, result: {}, size: {}kb",
        name,
        (now.elapsed().as_micros() as f64) / 1000.0,
        if let Err(e) = &result {
            format!("{:?}", e.major_status())
        } else {
            "Ok".to_string()
        },
        bytes.len() / 1000
    );
    // Also check whether the module actually fits into our payload size
    assert!(
        bytes.len() <= MAX_MODULE_SIZE,
        "test module exceeds size limit {} (given size {})",
        MAX_MODULE_SIZE,
        bytes.len()
    );
    result
}
