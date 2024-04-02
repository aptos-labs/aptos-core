// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::move_vm_ext::{AptosMoveResolver, SessionExt};
use aptos_types::transaction::EntryFunction;
use move_binary_format::errors::VMResult;

/// Returns true if function has an attribute that it uses randomness.
pub(crate) fn has_randomness_attribute(
    resolver: &impl AptosMoveResolver,
    session: &mut SessionExt,
    entry_fn: &EntryFunction,
) -> VMResult<bool> {
    let module = session
        .get_move_vm()
        .load_module(entry_fn.module(), resolver)?;
    let metadata = aptos_framework::get_metadata_from_compiled_module(&module);
    if let Some(metadata) = metadata {
        Ok(metadata
            .fun_attributes
            .get(entry_fn.function().as_str())
            .map(|attrs| attrs.iter().any(|attr| attr.is_randomness()))
            .unwrap_or(false))
    } else {
        Ok(false)
    }
}
