// Copyright (c) Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::abstract_write_op::AbstractResourceWriteOp;
use aptos_types::{
    state_store::{state_key::StateKey, state_value::StateValueMetadata},
    write_set::WriteOpSize,
};
use move_binary_format::errors::PartialVMResult;
use move_core_types::{effects::Op, value::MoveTypeLayout};
use move_vm_types::values::Value;
use std::sync::Arc;

/// Interface to get state value metadata (for patching refunds), size of the write ops, and the
/// write ops themselves.
pub trait WriteOpInfoBuilder {
    /// Given an operation on a value, returns its metadata and size.
    ///
    /// If `assert_no_creation` is set, returns an error if the current operation is
    /// a creation.
    fn get_resource_metadata_and_size(
        &self,
        key: &StateKey,
        op: Op<&Value>,
        layout: &MoveTypeLayout,
        contains_delayed_fields: bool,
        assert_no_creation: bool,
    ) -> PartialVMResult<(StateValueMetadata, WriteOpSize)>;

    /// Given an operation on a value, and its metadata, returns the storage write op.
    fn get_resource_write_op(
        &self,
        op: Op<&Value>,
        layout: Arc<MoveTypeLayout>,
        contains_delayed_fields: bool,
        metadata: StateValueMetadata,
    ) -> PartialVMResult<AbstractResourceWriteOp>;

    /// If the resource ead at the key needs to be included in the write-set (i.e., it contains
    /// delayed fields which also have been modified), returns its size and metadata. Otherwise,
    /// returns [None].
    fn get_resource_metadata_and_size_for_read_with_delayed_fields(
        &self,
        key: &StateKey,
    ) -> PartialVMResult<Option<(StateValueMetadata, WriteOpSize)>>;

    /// If the group read at the key needs to be included in the write-set (i.e., it contains
    /// a member with delayed fields which also have been modified), returns its size and metadata.
    /// Otherwise, returns [None].
    fn get_group_metadata_and_size_for_read_with_delayed_fields(
        &self,
        key: &StateKey,
    ) -> PartialVMResult<Option<(StateValueMetadata, WriteOpSize)>>;
}
