// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{
    state_store::{state_key::StateKey, StateView},
    vm::{
        gas::get_gas_feature_version,
        resource_groups::{GroupSizeKind, ResourceGroupSize},
    },
};
use bytes::Bytes;
use move_binary_format::errors::{PartialVMError, PartialVMResult};
use move_core_types::{language_storage::StructTag, value::MoveTypeLayout, vm_status::StatusCode};
use move_table_extension::{TableHandle, TableResolver};
use std::{
    cell::RefCell,
    collections::{BTreeMap, HashMap},
};

enum StateViewAdapter<'s, S> {
    Borrowed(&'s S),
    Owned(S),
}

/// Used for non-block execution context where there is no Block-STM to convert [StateView] into
/// other traits, e.g., that can handle Move extensions and resource groups.
pub struct ExecutorViewAdapter<'s, S> {
    /// Holds the underlying state for which we fetch bytes and other data.
    state_view_adapter: StateViewAdapter<'s, S>,
    /// Specifies how group size is computed. The invariant is that it is set to either
    /// [GroupSizeKind::None] or [GroupSizeKind::AsBlob].
    group_size_kind: GroupSizeKind,
    /// Cache for resource groups, so that they can be resolved by Move runtime via adapter.
    group_cache: RefCell<HashMap<StateKey, (BTreeMap<StructTag, Bytes>, ResourceGroupSize)>>,
}

impl<'s, S> ExecutorViewAdapter<'s, S>
where
    S: StateView,
{
    /// Creates and adapter consuming the [StateView].
    pub fn owned(state_view: S) -> Self {
        let gas_feature_version = get_gas_feature_version(&state_view);
        let group_size_kind =
            GroupSizeKind::from_gas_feature_version_without_split(gas_feature_version);
        assert_ne!(group_size_kind, GroupSizeKind::AsSum);

        Self {
            state_view_adapter: StateViewAdapter::Owned(state_view),
            group_size_kind,
            group_cache: RefCell::new(HashMap::new()),
        }
    }

    /// Creates and adapter from the reference to the [StateView].
    pub fn borrowed(state_view: &'s S) -> Self {
        let gas_feature_version = get_gas_feature_version(state_view);
        let group_size_kind =
            GroupSizeKind::from_gas_feature_version_without_split(gas_feature_version);
        assert_ne!(group_size_kind, GroupSizeKind::AsSum);

        Self {
            state_view_adapter: StateViewAdapter::Borrowed(state_view),
            group_size_kind,
            group_cache: RefCell::new(HashMap::new()),
        }
    }

    /// Returns the reference to the [StateView].
    pub fn state_view(&self) -> &S {
        match &self.state_view_adapter {
            StateViewAdapter::Borrowed(state_view) => state_view,
            StateViewAdapter::Owned(state_view) => state_view,
        }
    }

    /// Returns the mutable reference to the [StateView], if it is owned. If borrowed, [None] is
    /// returned.
    pub fn state_view_mut(&mut self) -> Option<&mut S> {
        match &mut self.state_view_adapter {
            StateViewAdapter::Borrowed(_) => None,
            StateViewAdapter::Owned(state_view) => Some(state_view),
        }
    }

    pub fn group_size_kind(&self) -> GroupSizeKind {
        self.group_size_kind
    }

    pub fn group_cache(
        &self,
    ) -> &RefCell<HashMap<StateKey, (BTreeMap<StructTag, Bytes>, ResourceGroupSize)>> {
        &self.group_cache
    }

    /// Ensures that the resource group at state key is cached. Returns true if the group has been
    /// already cached, and false it just got cached.
    pub fn load_group_to_cache(&self, group_key: &StateKey) -> PartialVMResult<bool> {
        let already_cached = self.group_cache.borrow().contains_key(group_key);
        if already_cached {
            return Ok(true);
        }

        let group_data = self
            .state_view()
            .get_state_value_bytes(group_key)
            .map_err(|err| {
                PartialVMError::new(StatusCode::STORAGE_ERROR).with_message(format!(
                    "Unexpected storage error for resource group at {:?}: {:?}",
                    group_key, err
                ))
            })?;
        let (group_data, blob_len): (BTreeMap<StructTag, Bytes>, u64) = group_data.map_or_else(
            || Ok::<_, PartialVMError>((BTreeMap::new(), 0)),
            |group_data_blob| {
                let group_data = bcs::from_bytes(&group_data_blob).map_err(|err| {
                    PartialVMError::new(StatusCode::UNEXPECTED_DESERIALIZATION_ERROR).with_message(
                        format!(
                            "Failed to deserialize the resource group at {:?}: {:?}",
                            group_key, err
                        ),
                    )
                })?;
                Ok((group_data, group_data_blob.len() as u64))
            },
        )?;

        let group_size = match self.group_size_kind {
            GroupSizeKind::None => ResourceGroupSize::Concrete(0),
            GroupSizeKind::AsBlob => ResourceGroupSize::Concrete(blob_len),
            GroupSizeKind::AsSum => {
                unreachable!("Resource groups are not split for state view adapter")
            },
        };
        self.group_cache
            .borrow_mut()
            .insert(group_key.clone(), (group_data, group_size));
        Ok(false)
    }
}

impl<'s, S> TableResolver for ExecutorViewAdapter<'s, S>
where
    S: StateView,
{
    fn resolve_table_entry_bytes_with_layout(
        &self,
        handle: &TableHandle,
        key: &[u8],
        _maybe_layout: Option<&MoveTypeLayout>,
    ) -> Result<Option<Bytes>, PartialVMError> {
        let state_key = StateKey::table_item(&(*handle).into(), key);
        self.state_view()
            .get_state_value_bytes(&state_key)
            .map_err(|err| {
                PartialVMError::new(StatusCode::STORAGE_ERROR).with_message(format!(
                    "Unexpected storage error for table item at {:?}: {:?}",
                    state_key, err
                ))
            })
    }
}
