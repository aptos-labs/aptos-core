// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::move_vm_ext::AptosMoveResolver;
use aptos_framework::{KnownAttribute, RandomnessAnnotation};
use aptos_types::transaction::EntryFunction;
use move_binary_format::errors::{Location, VMResult};

pub(crate) fn get_randomness_annotation(
    resolver: &impl AptosMoveResolver,
    entry_fn: &EntryFunction,
) -> VMResult<Option<RandomnessAnnotation>> {
    let md = resolver
        .fetch_module_metadata(entry_fn.module().address(), entry_fn.module().name())
        .map_err(|e| e.finish(Location::Undefined))?;
    let metadata = aptos_framework::get_metadata(md);
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
