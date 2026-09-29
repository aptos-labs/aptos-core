// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use aptos_types::vm::module_metadata::get_compilation_metadata;
use move_binary_format::{
    errors::{Location, PartialVMError, VMResult},
    file_format::CompiledScript,
    file_format_common::VERSION_5,
    CompiledModule,
};
use move_core_types::vm_status::StatusCode;

/// Reject publishing of legacy (v5) module bytecode. Publishing only; modules already on chain
/// keep loading and executing at any version.
pub fn reject_legacy_module_bytecode(
    reject_legacy_bytecode: bool,
    modules: &[CompiledModule],
) -> VMResult<()> {
    if reject_legacy_bytecode {
        for module in modules {
            if module.version <= VERSION_5 {
                return Err(PartialVMError::new(StatusCode::CONSTRAINT_NOT_SATISFIED)
                    .with_message(format!(
                        "publishing module bytecode version {} is not allowed; the minimum \
                         publishable version is {}",
                        module.version,
                        VERSION_5 + 1
                    ))
                    .finish(Location::Undefined));
            }
        }
    }
    Ok(())
}

/// Check whether the bytecode can be published to mainnet based on the unstable tag in the metadata
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

/// Check whether the script can be run on mainnet based on the unstable tag in the metadata
pub fn reject_unstable_bytecode_for_script(
    is_mainnet: bool,
    script: &CompiledScript,
) -> VMResult<()> {
    if is_mainnet {
        if let Some(metadata) = get_compilation_metadata(script) {
            if metadata.unstable {
                return Err(PartialVMError::new(StatusCode::UNSTABLE_BYTECODE_REJECTED)
                    .with_message("script marked unstable cannot be run on mainnet".to_string())
                    .finish(Location::Script));
            }
        }
    }
    Ok(())
}
