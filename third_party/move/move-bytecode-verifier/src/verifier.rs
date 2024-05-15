// Copyright (c) The Diem Core Contributors
// Copyright (c) The Move Contributors
// SPDX-License-Identifier: Apache-2.0

//! This module contains the public APIs supported by the bytecode verifier.
use crate::{
    ability_field_requirements, check_duplication::DuplicationChecker,
    code_unit_verifier::CodeUnitVerifier, constants, friends,
    instantiation_loops::InstantiationLoopChecker, instruction_consistency::InstructionConsistency,
    limits::LimitsVerifier, script_signature,
    script_signature::no_additional_script_signature_checks, signature::SignatureChecker,
    signature_v2, struct_defs::RecursiveStructDefChecker,
};
use move_binary_format::{
    check_bounds::BoundsChecker,
    errors::{Location, PartialVMError, VMResult},
    file_format::{CompiledModule, CompiledScript},
};
use move_core_types::{state::VMState, vm_status::StatusCode};
use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
pub struct VerifierConfig {
    pub max_loop_depth: Option<usize>,
    pub max_function_parameters: Option<usize>,
    pub max_generic_instantiation_length: Option<usize>,
    pub max_basic_blocks: Option<usize>,
    pub max_value_stack_size: usize,
    pub max_type_nodes: Option<usize>,
    // Max number of pushes in one function.
    pub max_push_size: Option<usize>,
    // Max depth in dependency tree for both direct and friend dependencies.
    pub max_dependency_depth: Option<usize>,
    pub max_struct_definitions: Option<usize>,
    pub max_fields_in_struct: Option<usize>,
    pub max_function_definitions: Option<usize>,
    pub max_back_edges_per_function: Option<usize>,
    pub max_back_edges_per_module: Option<usize>,
    pub max_basic_blocks_in_script: Option<usize>,

    // General metering for the verifier.
    pub max_per_fun_meter_units: Option<u128>,
    pub max_per_mod_meter_units: Option<u128>,

    pub use_signature_checker_v2: bool,
    pub sig_checker_v2_fix_script_ty_param_count: bool,
}

pub fn verify_module(config: &VerifierConfig, module: &CompiledModule) -> VMResult<()> {
    let prev_state = move_core_types::state::set_state(VMState::VERIFIER);
    let result = std::panic::catch_unwind(|| {
        BoundsChecker::verify_module(module).map_err(|e| {
            // We can't point the error at the module, because if bounds-checking
            // failed, we cannot safely index into module's handle to itself.
            e.finish(Location::Undefined)
        })?;
        LimitsVerifier::verify_module(config, module)?;
        DuplicationChecker::verify_module(module)?;

        if config.use_signature_checker_v2 {
            signature_v2::verify_module(module)?;
        } else {
            SignatureChecker::verify_module(module)?;
        }

        InstructionConsistency::verify_module(module)?;
        constants::verify_module(module)?;
        friends::verify_module(module)?;
        if !config.use_signature_checker_v2 {
            // This has been merged into the new signature checker so no need to run it if that one is enabled.
            ability_field_requirements::verify_module(module)?;
        }
        RecursiveStructDefChecker::verify_module(module)?;
        InstantiationLoopChecker::verify_module(module)?;
        CodeUnitVerifier::verify_module(config, module)?;

        // Add the failpoint injection to test the catch_unwind behavior.
        fail::fail_point!("verifier-failpoint-panic");

        script_signature::verify_module(module, no_additional_script_signature_checks)
    })
    .unwrap_or_else(|_| {
        Err(
            PartialVMError::new(StatusCode::VERIFIER_INVARIANT_VIOLATION)
                .finish(Location::Undefined),
        )
    });
    move_core_types::state::set_state(prev_state);
    result
}

pub fn verify_script(config: &VerifierConfig, script: &CompiledScript) -> VMResult<()> {
    let prev_state = move_core_types::state::set_state(VMState::VERIFIER);
    let result = std::panic::catch_unwind(|| {
        BoundsChecker::verify_script(script).map_err(|e| e.finish(Location::Script))?;
        LimitsVerifier::verify_script(config, script)?;
        DuplicationChecker::verify_script(script)?;

        if config.use_signature_checker_v2 {
            signature_v2::verify_script(config, script)?;
        } else {
            SignatureChecker::verify_script(script)?;
        }

        InstructionConsistency::verify_script(script)?;
        constants::verify_script(script)?;
        CodeUnitVerifier::verify_script(config, script)?;
        script_signature::verify_script(script, no_additional_script_signature_checks)
    })
    .unwrap_or_else(|_| {
        Err(
            PartialVMError::new(StatusCode::VERIFIER_INVARIANT_VIOLATION)
                .with_message("[VM] bytecode verifier panicked for script".to_string())
                .finish(Location::Undefined),
        )
    });
    move_core_types::state::set_state(prev_state);

    result
}

impl Default for VerifierConfig {
    fn default() -> Self {
        VerifierConfig::unbounded()
    }
}

impl VerifierConfig {
    /// Returns truly unbounded config, even relaxing metering.
    pub fn unbounded() -> Self {
        Self {
            max_loop_depth: None,
            max_function_parameters: None,
            max_generic_instantiation_length: None,
            max_basic_blocks: None,
            max_type_nodes: None,
            // Max size set to 1024 to match the size limit in the interpreter.
            max_value_stack_size: 1024,
            max_push_size: None,
            max_dependency_depth: None,
            max_struct_definitions: None,
            max_fields_in_struct: None,
            max_function_definitions: None,
            max_back_edges_per_function: None,
            max_back_edges_per_module: None,
            max_basic_blocks_in_script: None,

            max_per_fun_meter_units: None,
            max_per_mod_meter_units: None,

            use_signature_checker_v2: true,
            sig_checker_v2_fix_script_ty_param_count: true,
        }
    }

    /// An approximation of what config is used in production.
    pub fn bounded() -> Self {
        Self {
            max_loop_depth: Some(5),
            max_generic_instantiation_length: Some(32),
            max_function_parameters: Some(128),
            max_basic_blocks: Some(1024),
            max_basic_blocks_in_script: Some(1024),
            max_value_stack_size: 1024,
            max_type_nodes: Some(256),
            max_push_size: Some(10000),
            max_dependency_depth: Some(100),
            max_struct_definitions: Some(200),
            max_fields_in_struct: Some(30),
            max_function_definitions: Some(1000),

            // Do not use back edge constraints as they are superseded by metering.
            max_back_edges_per_function: None,
            max_back_edges_per_module: None,

            max_per_fun_meter_units: Some(1000 * 8000),
            max_per_mod_meter_units: Some(1000 * 8000),

            use_signature_checker_v2: true,
            sig_checker_v2_fix_script_ty_param_count: true,
        }
    }
}
