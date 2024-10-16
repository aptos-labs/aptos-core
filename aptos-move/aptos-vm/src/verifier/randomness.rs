// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::move_vm_ext::{AptosMoveResolver, SessionExt};
use aptos_framework::{KnownAttribute, RandomnessAnnotation};
use aptos_types::transaction::EntryFunction;
use move_binary_format::errors::VMResult;

pub(crate) fn get_randomness_annotation(
    resolver: &impl AptosMoveResolver,
    session: &mut SessionExt,
    entry_fn: &EntryFunction,
) -> VMResult<Option<RandomnessAnnotation>> {
    let module = session
        .get_move_vm()
        .load_module(entry_fn.module(), resolver)?;
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
