// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

//! Records the production bytecode-verifier configuration (deliverable A3 of
//! `third_party/move/lean/doc/designs/bytecode-verifier-soundness.md`).
//!
//! The CLI (`src/main.rs`):
//!
//! ```text
//! move-bytecode-lean-config-export [--output FILE]
//! ```
//!
//! Computes `aptos_prod_verifier_config` from fixed inputs and prints them
//! with the resulting configuration as JSON. The inputs are the latest gas
//! feature version, the default feature flags, and the mainnet timed features
//! at `TIMESTAMP`. The Lean specification records the same value in
//! `Spec/Config.lean`, and its tests check the two agree.

use aptos_gas_schedule::LATEST_GAS_FEATURE_VERSION;
use aptos_types::{
    chain_id::ChainId,
    on_chain_config::{Features, TimedFeaturesBuilder},
};
use aptos_vm_environment::prod_configs::aptos_prod_verifier_config;
use move_bytecode_verifier::{verifier::VerificationScope, VerifierConfig};
use serde_json::{json, Value};

pub const FORMAT: &str = "move-bytecode-lean-config/1";

/// 2026-10-01T00:00:00Z, the date the configuration was pinned.
pub const TIMESTAMP_MICROS: u64 = 1_790_812_800_000_000;

fn opt(n: Option<usize>) -> Value {
    match n {
        Some(n) => json!(n),
        None => Value::Null,
    }
}

fn opt_u128(n: Option<u128>) -> Value {
    match n {
        Some(n) => json!(n.to_string()),
        None => Value::Null,
    }
}

/// Every field, by exhaustive destructuring, so a new field must be recorded.
pub fn config(c: &VerifierConfig) -> Value {
    let VerifierConfig {
        scope,
        max_loop_depth,
        max_function_parameters,
        max_generic_instantiation_length,
        max_basic_blocks,
        max_value_stack_size,
        max_type_nodes,
        max_push_size,
        max_struct_definitions,
        max_struct_variants,
        max_fields_in_struct,
        max_function_definitions,
        max_back_edges_per_function,
        max_back_edges_per_module,
        max_basic_blocks_in_script,
        max_per_fun_meter_units,
        max_per_mod_meter_units,
        _use_signature_checker_v2,
        sig_checker_v2_fix_script_ty_param_count,
        enable_enum_types,
        enable_resource_access_control,
        enable_function_values,
        max_function_return_values,
        max_type_depth,
        sig_checker_v2_fix_function_signatures,
        check_function_type_abilities,
    } = c;
    let scope = match scope {
        VerificationScope::Everything => "Everything",
        VerificationScope::Nothing => "Nothing",
    };
    json!({
        "scope": scope,
        "max_loop_depth": opt(*max_loop_depth),
        "max_function_parameters": opt(*max_function_parameters),
        "max_generic_instantiation_length": opt(*max_generic_instantiation_length),
        "max_basic_blocks": opt(*max_basic_blocks),
        "max_value_stack_size": max_value_stack_size,
        "max_type_nodes": opt(*max_type_nodes),
        "max_push_size": opt(*max_push_size),
        "max_struct_definitions": opt(*max_struct_definitions),
        "max_struct_variants": opt(*max_struct_variants),
        "max_fields_in_struct": opt(*max_fields_in_struct),
        "max_function_definitions": opt(*max_function_definitions),
        "max_back_edges_per_function": opt(*max_back_edges_per_function),
        "max_back_edges_per_module": opt(*max_back_edges_per_module),
        "max_basic_blocks_in_script": opt(*max_basic_blocks_in_script),
        "max_per_fun_meter_units": opt_u128(*max_per_fun_meter_units),
        "max_per_mod_meter_units": opt_u128(*max_per_mod_meter_units),
        "use_signature_checker_v2": _use_signature_checker_v2,
        "sig_checker_v2_fix_script_ty_param_count": sig_checker_v2_fix_script_ty_param_count,
        "enable_enum_types": enable_enum_types,
        "enable_resource_access_control": enable_resource_access_control,
        "enable_function_values": enable_function_values,
        "max_function_return_values": opt(*max_function_return_values),
        "max_type_depth": opt(*max_type_depth),
        "sig_checker_v2_fix_function_signatures": sig_checker_v2_fix_function_signatures,
        "check_function_type_abilities": check_function_type_abilities,
    })
}

/// `aptos_prod_verifier_config` on the recorded inputs.
pub fn production_config() -> VerifierConfig {
    let features = Features::default();
    let timed_features = TimedFeaturesBuilder::new(ChainId::mainnet(), TIMESTAMP_MICROS).build();
    aptos_prod_verifier_config(LATEST_GAS_FEATURE_VERSION, &features, &timed_features)
}

/// The recorded inputs and the resulting configuration.
pub fn document() -> Value {
    json!({
        "format": FORMAT,
        "inputs": {
            "gas_feature_version": LATEST_GAS_FEATURE_VERSION,
            "features": "default",
            "timed_features_chain": "mainnet",
            "timed_features_timestamp_micros": TIMESTAMP_MICROS.to_string(),
        },
        "verifier_config": config(&production_config()),
    })
}
