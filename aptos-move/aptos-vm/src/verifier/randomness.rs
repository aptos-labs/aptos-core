// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::move_vm_ext::{AptosMoveResolver, SessionExt};
use aptos_framework::{KnownAttribute, RandomnessAnnotation};
use aptos_types::transaction::EntryFunction;
use aptos_vm_types::module_and_script_storage::module_storage::AptosModuleStorage;
use move_binary_format::errors::VMResult;
use move_vm_runtime::module_linker_error;

pub(crate) fn get_randomness_annotation(
    resolver: &impl AptosMoveResolver,
    module_storage: &impl AptosModuleStorage,
    session: &mut SessionExt,
    entry_fn: &EntryFunction,
    use_loader_v2: bool,
) -> VMResult<Option<RandomnessAnnotation>> {
    let module = if use_loader_v2 {
        let addr = entry_fn.module().address();
        let name = entry_fn.module().name();

        // TODO(loader_v2): Enhance this further by querying RuntimeModuleMetadataV1 directly.
        module_storage
            .fetch_deserialized_module(addr, name)?
            .ok_or_else(|| module_linker_error!(addr, name))?
    } else {
        #[allow(deprecated)]
        session
            .get_move_vm()
            .load_module(entry_fn.module(), resolver)?
    };

    let metadata = aptos_framework::get_metadata_from_compiled_module(&module);
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
