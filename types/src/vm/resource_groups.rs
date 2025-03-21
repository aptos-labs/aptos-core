// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use move_binary_format::file_format_common::size_u32_as_uleb128;

/// Corresponding to different gas features, methods for counting the 'size' of a
/// resource group. None leads to 0, while AsBlob provides the group size as the
/// size of the serialized blob of the BTreeMap corresponding to the group.
/// For AsSum, the size is summed for each resource contained in the group (of
/// the resource blob, and its corresponding tag, when serialized)
#[derive(PartialEq, Eq, Copy, Clone, Debug)]
pub enum GroupSizeKind {
    None,
    AsBlob,
    AsSum,
}

impl GroupSizeKind {
    pub fn from_gas_feature_version(
        gas_feature_version: u64,
        resource_groups_split_in_vm_change_set_enabled: bool,
    ) -> Self {
        if resource_groups_split_in_vm_change_set_enabled {
            GroupSizeKind::AsSum
        } else {
            Self::from_gas_feature_version_without_split(gas_feature_version)
        }
    }

    pub(crate) fn from_gas_feature_version_without_split(gas_feature_version: u64) -> Self {
        if gas_feature_version >= 9 {
            // Keep old caching behavior for replay.
            GroupSizeKind::AsBlob
        } else {
            GroupSizeKind::None
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ResourceGroupSize {
    Concrete(u64),
    /// Combined represents what would the size be if we know individual
    /// parts that contribute to it. This is useful when individual parts
    /// are changing, and we want to know what the size of the group would be.
    ///
    /// Formula is based on how bcs serializes the BTreeMap:
    ///   varint encoding len(num_tagged_resources) + all_tagged_resources_size
    /// Also, if num_tagged_resources is 0, then the size is 0, because we will not store
    /// empty resource group in storage.
    Combined {
        num_tagged_resources: usize,
        all_tagged_resources_size: u64,
    },
}

impl ResourceGroupSize {
    pub fn zero_combined() -> Self {
        Self::Combined {
            num_tagged_resources: 0,
            all_tagged_resources_size: 0,
        }
    }

    pub fn zero_concrete() -> Self {
        Self::Concrete(0)
    }

    pub fn get(&self) -> u64 {
        match self {
            Self::Concrete(size) => *size,
            Self::Combined {
                num_tagged_resources,
                all_tagged_resources_size,
            } => {
                if *num_tagged_resources == 0 {
                    0
                } else {
                    size_u32_as_uleb128(*num_tagged_resources) as u64 + *all_tagged_resources_size
                }
            },
        }
    }
}
