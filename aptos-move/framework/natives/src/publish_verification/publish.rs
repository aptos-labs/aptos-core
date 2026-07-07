// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Shared module-publishing validation, reused by AptosVM's legacy publish path
//! and by the `code::verify_package` native. This performs the Aptos-specific
//! checks (unstable bytecode, native functions, expected modules / allowed
//! dependencies, module metadata, resource groups, events). The bytecode
//! verification, compatibility, and linking checks live in `StagingModuleStorage`
//! (move-vm-runtime).

use crate::publish_verification::{event_validation, native_validation, resource_groups};
use aptos_types::{
    on_chain_config::Features,
    vm::module_metadata::{get_compilation_metadata, verify_module_metadata_for_module_publishing},
};
use move_binary_format::{
    access::ModuleAccess,
    errors::{Location, PartialVMError, VMError, VMResult},
    CompiledModule,
};
use move_core_types::{account_address::AccountAddress, vm_status::StatusCode};
use move_vm_runtime::{module_traversal::TraversalContext, ModuleStorage};
use move_vm_types::gas::DependencyGasMeter;
use std::collections::{BTreeMap, BTreeSet};

pub fn metadata_validation_error(msg: &str) -> VMError {
    PartialVMError::new(StatusCode::CONSTRAINT_NOT_SATISFIED)
        .with_message(format!("metadata and code bundle mismatch: {}", msg))
        .finish(Location::Undefined)
}

/// Check whether the bytecode can be published to mainnet based on the unstable
/// tag in the compilation metadata.
pub fn reject_unstable_bytecode(is_mainnet: bool, modules: &[CompiledModule]) -> VMResult<()> {
    if is_mainnet {
        for module in modules {
            if let Some(metadata) = get_compilation_metadata(module) {
                if metadata.unstable {
                    return Err(PartialVMError::new(StatusCode::UNSTABLE_BYTECODE_REJECTED)
                        .with_message(
                            "code marked unstable is not published on mainnet".to_string(),
                        )
                        .finish(Location::Undefined));
                }
            }
        }
    }
    Ok(())
}

/// Validate a publish request: reject unstable bytecode, validate native
/// functions, check the modules against the expected-module set and the
/// allowed-dependency whitelist, validate module metadata, resource groups, and
/// events.
pub fn validate_publish_request(
    features: &Features,
    is_mainnet: bool,
    module_storage: &impl ModuleStorage,
    traversal_context: &mut TraversalContext,
    gas_meter: &mut dyn DependencyGasMeter,
    modules: &[CompiledModule],
    mut expected_modules: BTreeSet<String>,
    allowed_deps: Option<BTreeMap<AccountAddress, BTreeSet<String>>>,
) -> VMResult<()> {
    reject_unstable_bytecode(is_mainnet, modules)?;
    native_validation::validate_module_natives(modules)?;

    for m in modules {
        if !expected_modules.remove(m.self_id().name().as_str()) {
            return Err(metadata_validation_error(&format!(
                "unregistered module: '{}'",
                m.self_id().name()
            )));
        }
        if let Some(allowed) = &allowed_deps {
            for dep in m.immediate_dependencies() {
                if !allowed
                    .get(dep.address())
                    .map(|modules| modules.contains("") || modules.contains(dep.name().as_str()))
                    .unwrap_or(false)
                {
                    return Err(metadata_validation_error(&format!(
                        "unregistered dependency: '{}'",
                        dep
                    )));
                }
            }
        }
        verify_module_metadata_for_module_publishing(m, features)
            .map_err(|err| metadata_validation_error(&err.to_string()))?;
    }

    resource_groups::validate_resource_groups(
        features,
        module_storage,
        traversal_context,
        gas_meter,
        modules,
    )?;
    event_validation::validate_module_events(features, module_storage, traversal_context, modules)?;

    if !expected_modules.is_empty() {
        return Err(metadata_validation_error(
            "not all registered modules published",
        ));
    }
    Ok(())
}
