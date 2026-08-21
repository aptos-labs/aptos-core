// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

#![allow(dead_code)]

use aptos_types::block_executor::config::BlockExecutorModuleCacheLocalConfig;
use move_bytecode_verifier::VerifierConfig;
use move_vm_runtime::config::VMConfig;
use move_vm_types::loaded_data::runtime_types::TypeBuilder;
use std::env;

pub(crate) fn env_u64(name: &str, default: u64) -> u64 {
    match env::var(name) {
        Ok(raw) => raw
            .parse()
            .unwrap_or_else(|err| panic!("{name} must be a u64, got {raw:?}: {err}")),
        Err(env::VarError::NotPresent) => default,
        Err(err) => panic!("failed to read {name}: {err}"),
    }
}

pub(crate) fn env_optional_u64(name: &str) -> Option<u64> {
    match env::var(name) {
        Ok(raw) => Some(
            raw.parse()
                .unwrap_or_else(|err| panic!("{name} must be a u64, got {raw:?}: {err}")),
        ),
        Err(env::VarError::NotPresent) => None,
        Err(err) => panic!("failed to read {name}: {err}"),
    }
}

pub(crate) fn env_optional_u32(name: &str) -> Option<u32> {
    match env::var(name) {
        Ok(raw) => Some(
            raw.parse()
                .unwrap_or_else(|err| panic!("{name} must be a u32, got {raw:?}: {err}")),
        ),
        Err(env::VarError::NotPresent) => None,
        Err(err) => panic!("failed to read {name}: {err}"),
    }
}

pub(crate) fn env_optional_usize(name: &str) -> Option<usize> {
    match env::var(name) {
        Ok(raw) => Some(
            raw.parse()
                .unwrap_or_else(|err| panic!("{name} must be a usize, got {raw:?}: {err}")),
        ),
        Err(env::VarError::NotPresent) => None,
        Err(err) => panic!("failed to read {name}: {err}"),
    }
}

pub(crate) fn env_optional_bool(name: &str) -> Option<bool> {
    match env::var(name) {
        Ok(raw) => match raw.as_str() {
            "1" | "true" | "TRUE" | "yes" | "YES" | "on" | "ON" => Some(true),
            "0" | "false" | "FALSE" | "no" | "NO" | "off" | "OFF" => Some(false),
            _ => panic!("{name} must be a bool, got {raw:?}"),
        },
        Err(env::VarError::NotPresent) => None,
        Err(err) => panic!("failed to read {name}: {err}"),
    }
}

pub(crate) fn apply_verifier_config_overrides(config: &mut VerifierConfig) {
    if let Some(value) = env_optional_usize("APTOS_FUZZ_MAX_TYPE_NODES") {
        config.max_type_nodes = Some(value);
    }
    if let Some(value) = env_optional_usize("APTOS_FUZZ_MAX_TYPE_DEPTH") {
        config.max_type_depth = Some(value);
    }
}

pub(crate) fn apply_vm_config_overrides(config: &mut VMConfig) {
    apply_verifier_config_overrides(&mut config.verifier_config);

    if let Some(value) = env_optional_u64("APTOS_FUZZ_LAYOUT_MAX_SIZE") {
        config.layout_max_size = value;
    }
    if let Some(value) = env_optional_u64("APTOS_FUZZ_LAYOUT_MAX_DEPTH") {
        config.layout_max_depth = value;
    }
    if let Some(value) = env_optional_u64("APTOS_FUZZ_MAX_VALUE_NEST_DEPTH") {
        config.max_value_nest_depth = Some(value);
    }
    if let Some(value) = env_optional_u64("APTOS_FUZZ_TYPE_MAX_COST") {
        config.type_max_cost = value;
    }

    let max_ty_size = env_optional_u64("APTOS_FUZZ_TYPE_BUILDER_MAX_SIZE");
    let max_ty_depth = env_optional_u64("APTOS_FUZZ_TYPE_BUILDER_MAX_DEPTH");
    if max_ty_size.is_some() || max_ty_depth.is_some() {
        config.ty_builder = TypeBuilder::with_limits(
            max_ty_size.unwrap_or(128),
            max_ty_depth.unwrap_or(20),
            config.check_depth_on_type_counts,
        );
    }
}

pub(crate) fn module_cache_config_from_env() -> BlockExecutorModuleCacheLocalConfig {
    let mut config = BlockExecutorModuleCacheLocalConfig::default();
    if let Some(value) = env_optional_usize("APTOS_FUZZ_MAX_MODULE_CACHE_SIZE_BYTES") {
        config.max_module_cache_size_in_bytes = value;
    }
    if let Some(value) = env_optional_usize("APTOS_FUZZ_MAX_STRUCT_NAME_INDEX_MAP_ENTRIES") {
        config.max_struct_name_index_map_num_entries = value;
    }
    if let Some(value) = env_optional_usize("APTOS_FUZZ_MAX_INTERNED_TYS") {
        config.max_interned_tys = value;
    }
    if let Some(value) = env_optional_usize("APTOS_FUZZ_MAX_INTERNED_TY_VECS") {
        config.max_interned_ty_vecs = value;
    }
    if let Some(value) = env_optional_usize("APTOS_FUZZ_MAX_LAYOUT_CACHE_SIZE") {
        config.max_layout_cache_size = value;
    }
    if let Some(value) = env_optional_usize("APTOS_FUZZ_MAX_INTERNED_MODULE_IDS") {
        config.max_interned_module_ids = value;
    }
    config
}
