// Copyright (c) Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use aptos_types::{chain_id::ChainId, vm::module_metadata::get_compilation_metadata};
use move_binary_format::{
    errors::{Location, PartialVMError, VMResult},
    file_format::CompiledScript,
};
use move_core_types::vm_status::StatusCode;

/// Checks whether the script can be run on mainnet based on the unstable tag in the metadata.
pub fn reject_unstable_bytecode_for_script(
    script: &CompiledScript,
    chain_id: ChainId,
) -> VMResult<()> {
    if chain_id.is_mainnet() {
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
