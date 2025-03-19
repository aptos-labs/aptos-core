// Copyright © Aptos Foundation
// Parts of the project are originally copyright © Meta Platforms, Inc.
// SPDX-License-Identifier: Apache-2.0

use move_core_types::{language_storage::StructTag, metadata::Metadata};

pub fn get_resource_group_member_from_metadata(
    struct_tag: &StructTag,
    metadata: &[Metadata],
) -> Option<StructTag> {
    let metadata = aptos_framework::get_metadata(metadata)?;
    metadata
        .struct_attributes
        .get(struct_tag.name.as_ident_str().as_str())?
        .iter()
        .find_map(|attr| attr.get_resource_group_member())
}

// #[cfg(test)]
// pub(crate) mod tests {
//     use super::*;
//     use aptos_types::vm::resource_groups::GroupSizeKind;
//
//     // Expose a method to create a storage adapter with a provided group size kind.
//     pub(crate) fn as_resolver_with_group_size_kind<E: ExecutorView>(
//         executor_view: &E,
//         group_size_kind: GroupSizeKind,
//     ) -> StorageAdapter<E> {
//         assert_ne!(group_size_kind, GroupSizeKind::AsSum, "not yet supported");
//
//         let (gas_feature_version, resource_groups_split_in_vm_change_set_enabled) =
//             match group_size_kind {
//                 GroupSizeKind::AsSum => (12, true),
//                 GroupSizeKind::AsBlob => (10, false),
//                 GroupSizeKind::None => (1, false),
//             };
//
//         let group_adapter = ResourceGroupAdapter::new(
//             // TODO[agg_v2](test) add a converter for StateView for tests that implements ResourceGroupView
//             None,
//             executor_view,
//             gas_feature_version,
//             resource_groups_split_in_vm_change_set_enabled,
//         );
//
//         StorageAdapter::new(executor_view, group_adapter)
//     }
// }
