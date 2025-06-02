// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use move_binary_format::{
    access::ModuleAccess,
    errors::{Location, PartialVMError, VMResult},
    CompiledModule,
};
use move_core_types::vm_status::StatusCode;

/// Validate that only system address can publish new non-entry natives.
pub(crate) fn validate_module_natives(modules: &[CompiledModule]) -> VMResult<()> {
    for module in modules {
        let module_address = module.self_addr();
        for native in module.function_defs().iter().filter(|def| def.is_native()) {
            if native.is_entry || !module_address.is_special() {
                return Err(
                    PartialVMError::new(StatusCode::USER_DEFINED_NATIVE_NOT_ALLOWED)
                        .with_message(
                            "Cannot publish native function to non-special address".to_string(),
                        )
                        .finish(Location::Module(module.self_id())),
                );
            }
        }
    }
    Ok(())
}
