// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use aptos_types::state_store::{state_key::StateKey, state_value::StateValueMetadata};
use move_core_types::language_storage::StructTag;

/// Allows to query storage metadata in the VM session. Needed for storage refunds.
pub trait StateValueMetadataResolver {
    /// Returns metadata for a given state value:
    ///   - None             if state value does not exist,
    ///   - Some(None)       if state value has no metadata,
    ///   - Some(Some(..))   otherwise.
    // TODO: Nested options are ugly, refactor.
    fn get_state_value_metadata(
        &self,
        state_key: &StateKey,
    ) -> anyhow::Result<Option<Option<StateValueMetadata>>>;
}

pub trait TResourceGroupResolver {
    type Key;
    type Tag;

    fn get_resource_from_group(
        &self,
        key: &Self::Key,
        resource_tag: &Self::Tag,
        return_group_size: bool,
    ) -> anyhow::Result<(Option<Vec<u8>>, Option<usize>)>;

    /// Needed for backwards compatibility with the additional safety mechanism for resource
    /// groups, where the violation of the following invariant causes transaction failure:
    /// - if a resource is modified or deleted it must already exist within a group,
    /// and if it is created, it must not previously exist.
    ///
    /// For normal resources, this is asserted, but for resource groups the behavior (that
    /// we maintain) is for the transaction to fail with INVARIANT_VIOLATION_ERROR.
    /// This ensures state does not change and blockchain does not halt while the underlying
    /// issue is addressed. In order to maintain the behavior we check for resource existence,
    /// which in the context of parallel execution does not cause a full R/W conflict.
    ///
    /// Note: If and when we start using the method in other use-cases, in particular, if it
    /// may access a resource group for the first time, we should also incorporate the size
    /// charge for such access.
    fn resource_exists_in_group(
        &self,
        key: &Self::Key,
        resource_tag: &Self::Tag,
    ) -> anyhow::Result<bool> {
        self.get_resource_from_group(key, resource_tag, false)
            .map(|(res, _)| res.is_some())
    }
}

pub trait ResourceGroupResolver: TResourceGroupResolver<Key = StateKey, Tag = StructTag> {}

impl<T: TResourceGroupResolver<Key = StateKey, Tag = StructTag>> ResourceGroupResolver for T {}
