// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use aptos_types::{
    transaction::EntryFunction,
    vm::module_metadata::{
        get_metadata_from_compiled_module, KnownAttribute, RandomnessAnnotation,
    },
};
use aptos_vm_types::module_and_script_storage::module_storage::AptosModuleStorage;
use move_binary_format::errors::VMResult;

pub(crate) fn get_randomness_annotation(
    module_storage: &impl AptosModuleStorage,
    entry_fn: &EntryFunction,
) -> VMResult<Option<RandomnessAnnotation>> {
    // TODO(loader_v2): Enhance this further by querying RuntimeModuleMetadataV1 directly.
    let module = module_storage.fetch_existing_deserialized_module(
        entry_fn.module().address(),
        entry_fn.module().name(),
    )?;

    let metadata = get_metadata_from_compiled_module(&module);
    if let Some(metadata) = metadata {
        let maybe_annotation = metadata
            .fun_attributes
            .get(entry_fn.function().as_str())
            .map(|attrs| {
                attrs
                    .iter()
                    .filter_map(KnownAttribute::try_as_randomness_annotation)
                    .next()
            })
            .unwrap_or(None);
        Ok(maybe_annotation)
    } else {
        Ok(None)
    }
}
