// Copyright (c) The Diem Core Contributors
// Copyright (c) The Move Contributors
// SPDX-License-Identifier: Apache-2.0

//! This module contains the public APIs supported by the bytecode verifier.
use crate::{
    ability_field_requirements, check_duplication::DuplicationChecker,
    code_unit_verifier::CodeUnitVerifier, constants, features::FeatureVerifier, friends,
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
use std::time::Instant;

/// Configuration for the bytecode verifier.
///
/// Always add new fields to the end, as we rely on the hash or serialized bytes of config to
/// detect if it has changed (e.g., new feature flag was enabled). Also, do not delete existing
/// fields, or change the type of existing field.
#[derive(Debug, Clone, Eq, PartialEq, Serialize)]
pub struct VerifierConfig {
    pub scope: VerificationScope,
    pub max_loop_depth: Option<usize>,
    pub max_function_parameters: Option<usize>,
    pub max_generic_instantiation_length: Option<usize>,
    pub max_basic_blocks: Option<usize>,
    pub max_value_stack_size: usize,
    pub max_type_nodes: Option<usize>,
    pub max_push_size: Option<usize>,
    pub max_struct_definitions: Option<usize>,
    pub max_struct_variants: Option<usize>,
    pub max_fields_in_struct: Option<usize>,
    pub max_function_definitions: Option<usize>,
    pub max_back_edges_per_function: Option<usize>,
    pub max_back_edges_per_module: Option<usize>,
    pub max_basic_blocks_in_script: Option<usize>,
    pub max_per_fun_meter_units: Option<u128>,
    pub max_per_mod_meter_units: Option<u128>,
    pub use_signature_checker_v2: bool,
    pub sig_checker_v2_fix_script_ty_param_count: bool,
    pub enable_enum_types: bool,
    pub enable_resource_access_control: bool,
    pub enable_function_values: bool,
    /// Maximum number of function return values.
    pub max_function_return_values: Option<usize>,
    /// Maximum depth of a type node.
    pub max_type_depth: Option<usize>,
    /// If enabled, signature checker V2 also checks parameter and return types in function
    /// signatures.
    pub sig_checker_v2_fix_function_signatures: bool,
}

/// Scope of verification.
#[derive(Debug, Clone, Eq, PartialEq, Serialize)]
pub enum VerificationScope {
    /// Do all verification
    Everything,
    /// The remaining variants are for testing and should never be used in production
    Nothing,
}

/// Helper for a "canonical" verification of a module.
///
/// Clients that rely on verification should call the proper passes
/// internally rather than using this function.
///
/// This function is intended to provide a verification path for clients
/// that do not require full control over verification. It is advised to
/// call this umbrella function instead of each individual checkers to
/// minimize the code locations that need to be updated should a new checker
/// is introduced.
pub fn verify_module(module: &CompiledModule) -> VMResult<()> {
    verify_module_with_config(&VerifierConfig::default(), module)
}

pub fn verify_module_with_config_for_test(
    name: &str,
    config: &VerifierConfig,
    module: &CompiledModule,
) -> VMResult<()> {
    verify_module_with_config_for_test_with_version(name, config, module, None)
}

pub fn verify_module_with_config_for_test_with_version(
    name: &str,
    config: &VerifierConfig,
    module: &CompiledModule,
    bytecode_version: Option<u32>,
) -> VMResult<()> {
    const MAX_MODULE_SIZE: usize = 65355;
    let mut bytes = vec![];
    module
        .serialize_for_version(bytecode_version, &mut bytes)
        .unwrap();
    let now = Instant::now();
    let result = verify_module_with_config(config, module);
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

pub fn verify_module_with_config(config: &VerifierConfig, module: &CompiledModule) -> VMResult<()> {
    if config.verify_nothing() {
        return Ok(());
    }
    let prev_state = move_core_types::state::set_state(VMState::VERIFIER);
    let result = std::panic::catch_unwind(|| {
        // Always needs to run bound checker first as subsequent passes depend on it
        BoundsChecker::verify_module(module).map_err(|e| {
            // We can't point the error at the module, because if bounds-checking
            // failed, we cannot safely index into module's handle to itself.
            e.finish(Location::Undefined)
        })?;
        FeatureVerifier::verify_module(config, module)?;
        LimitsVerifier::verify_module(config, module)?;
        DuplicationChecker::verify_module(module)?;

        if config.use_signature_checker_v2 {
            signature_v2::verify_module(config, module)?;
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

/// Helper for a "canonical" verification of a script.
///
/// Clients that rely on verification should call the proper passes
/// internally rather than using this function.
///
/// This function is intended to provide a verification path for clients
/// that do not require full control over verification. It is advised to
/// call this umbrella function instead of each individual checkers to
/// minimize the code locations that need to be updated should a new checker
/// is introduced.
pub fn verify_script(script: &CompiledScript) -> VMResult<()> {
    verify_script_with_config(&VerifierConfig::default(), script)
}

pub fn verify_script_with_config(config: &VerifierConfig, script: &CompiledScript) -> VMResult<()> {
    if config.verify_nothing() {
        return Ok(());
    }
    let prev_state = move_core_types::state::set_state(VMState::VERIFIER);
    let result = std::panic::catch_unwind(|| {
        // Always needs to run bound checker first as subsequent passes depend on it
        BoundsChecker::verify_script(script).map_err(|e| {
            // We can't point the error at the script, because if bounds-checking
            // failed, we cannot safely index into script
            e.finish(Location::Undefined)
        })?;
        FeatureVerifier::verify_script(config, script)?;
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
        Self {
            scope: VerificationScope::Everything,
            max_loop_depth: None,
            max_function_parameters: None,
            max_generic_instantiation_length: None,
            max_basic_blocks: None,
            max_type_nodes: None,
            // Max size set to 1024 to match the size limit in the interpreter.
            max_value_stack_size: 1024,
            // Max number of pushes in one function
            max_push_size: None,
            // Max count of structs in a module
            max_struct_definitions: None,
            // Max count of fields in a struct
            max_fields_in_struct: None,
            // Max count of variants in a struct
            max_struct_variants: None,
            // Max count of functions in a module
            max_function_definitions: None,
            // Max size set to 10000 to restrict number of pushes in one function
            // max_push_size: Some(10000),
            // max_dependency_depth: Some(100),
            // max_struct_definitions: Some(200),
            // max_fields_in_struct: Some(30),
            // max_function_definitions: Some(1000),
            max_back_edges_per_function: None,
            max_back_edges_per_module: None,
            max_basic_blocks_in_script: None,
            // General metering for the verifier.
            // max_per_fun_meter_units: Some(1000 * 8000),
            // max_per_mod_meter_units: Some(1000 * 8000),
            max_per_fun_meter_units: None,
            max_per_mod_meter_units: None,

            use_signature_checker_v2: true,

            sig_checker_v2_fix_script_ty_param_count: true,
            sig_checker_v2_fix_function_signatures: true,

            enable_enum_types: true,
            enable_resource_access_control: true,
            enable_function_values: true,

            max_function_return_values: None,
            max_type_depth: None,
        }
    }
}

impl VerifierConfig {
    /// Returns truly unbounded config, even relaxing metering.
    pub fn unbounded() -> Self {
        Self {
            max_per_fun_meter_units: None,
            max_per_mod_meter_units: None,
            ..VerifierConfig::default()
        }
    }

    /// An approximation of what config is used in production.
    pub fn production() -> Self {
        Self {
            scope: VerificationScope::Everything,
            max_loop_depth: Some(5),
            max_generic_instantiation_length: Some(32),
            max_function_parameters: Some(128),
            max_basic_blocks: Some(1024),
            max_basic_blocks_in_script: Some(1024),
            max_value_stack_size: 1024,
            max_type_nodes: Some(128),
            max_push_size: Some(10000),
            max_struct_definitions: Some(200),
            max_fields_in_struct: Some(30),
            max_struct_variants: Some(90),
            max_function_definitions: Some(1000),

            // Do not use back edge constraints as they are superseded by metering
            max_back_edges_per_function: None,
            max_back_edges_per_module: None,

            // Same as the default.
            max_per_fun_meter_units: Some(1000 * 8000),
            max_per_mod_meter_units: Some(1000 * 8000),

            use_signature_checker_v2: true,
            sig_checker_v2_fix_script_ty_param_count: true,
            sig_checker_v2_fix_function_signatures: true,

            enable_enum_types: true,
            enable_resource_access_control: true,
            enable_function_values: true,

            max_function_return_values: Some(128),
            max_type_depth: Some(20),
        }
    }

    /// Set verification scope
    pub fn set_scope(self, scope: VerificationScope) -> Self {
        Self { scope, ..self }
    }

    /// Returns true if verification is disabled.
    pub fn verify_nothing(&self) -> bool {
        matches!(self.scope, VerificationScope::Nothing)
    }
}
