// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::move_vm_ext::SessionExt;
use aptos_types::transaction::EntryFunction;
use move_binary_format::{
    errors::{Location, VMResult},
    CompiledModule,
};

/// Returns true if function has an annotation that it uses randomness.
pub(crate) fn does_entry_function_use_randomness(
    session: &mut SessionExt,
    entry_fn: &EntryFunction,
) -> VMResult<bool> {
    let module_bytes = session.load_module(entry_fn.module())?;
    let module = CompiledModule::deserialize_with_config(
        &module_bytes,
        &session.get_vm_config().deserializer_config,
    )
    .map_err(|e| e.finish(Location::Undefined))?;

    let metadata = aptos_framework::get_metadata_from_compiled_module(&module);
    if let Some(metadata) = metadata {
        Ok(metadata
            .fun_attributes
            .get(entry_fn.function().as_str())
            .map(|attrs| attrs.iter().any(|attr| attr.is_uses_randomness()))
            .unwrap_or(false))
    } else {
        Ok(false)
    }
}
