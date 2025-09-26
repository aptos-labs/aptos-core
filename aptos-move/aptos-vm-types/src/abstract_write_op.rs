// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::resolver::{ExecutorView, ResourceGroupSize};
use aptos_types::{
    state_store::{state_key::StateKey, state_value::StateValueMetadata},
    write_set::{TransactionWrite, WriteOp, WriteOpSize},
};
use move_binary_format::errors::PartialVMResult;
use move_core_types::{language_storage::StructTag, value::MoveTypeLayout};
use std::collections::BTreeMap;
use triomphe::Arc as TriompheArc;

#[derive(PartialEq, Eq, Clone, Debug)]
pub enum AbstractResourceWriteOp {
    Write(WriteOp),
    WriteWithDelayedFields(WriteWithDelayedFieldsOp),
    // Prior to adding a dedicated write-set for resource groups, all resource group
    // updates are merged into a single WriteOp included in the resource_write_set.
    WriteResourceGroup(GroupWrite),
    // No writes in the resource, except for delayed field changes.
    InPlaceDelayedFieldChange(InPlaceDelayedFieldChangeOp),
    // No writes in the resource group, except for delayed field changes.
    ResourceGroupInPlaceDelayedFieldChange(ResourceGroupInPlaceDelayedFieldChangeOp),
}

impl AbstractResourceWriteOp {
    pub fn try_as_concrete_write(&self) -> Option<&WriteOp> {
        if let AbstractResourceWriteOp::Write(write_op) = self {
            Some(write_op)
        } else {
            None
        }
    }

    pub fn try_into_concrete_write(self) -> Option<WriteOp> {
        if let AbstractResourceWriteOp::Write(write_op) = self {
            Some(write_op)
        } else {
            None
        }
    }

    pub fn materialized_size(&self) -> WriteOpSize {
        use AbstractResourceWriteOp::*;
        match self {
            Write(write) => write.write_op_size(),
            WriteWithDelayedFields(WriteWithDelayedFieldsOp {
                write_op,
                materialized_size,
                ..
            }) => write_op.project_write_op_size(|| *materialized_size),
            WriteResourceGroup(GroupWrite {
                metadata_op: write_op,
                maybe_group_op_size,
                ..
            }) => write_op.project_write_op_size(|| maybe_group_op_size.map(|x| x.get())),
            InPlaceDelayedFieldChange(InPlaceDelayedFieldChangeOp {
                materialized_size, ..
            })
            | ResourceGroupInPlaceDelayedFieldChange(ResourceGroupInPlaceDelayedFieldChangeOp {
                materialized_size,
                ..
            }) => WriteOpSize::Modification {
                write_len: *materialized_size,
            },
        }
    }

    pub fn prev_materialized_size(
        &self,
        state_key: &StateKey,
        executor_view: &dyn ExecutorView,
        fix_prev_materialized_size: bool,
    ) -> PartialVMResult<u64> {
        use AbstractResourceWriteOp::*;
        let size = if fix_prev_materialized_size {
            match self {
                Write(_) | WriteWithDelayedFields(_) => {
                    executor_view.get_resource_state_value_size(state_key)?
                },
                InPlaceDelayedFieldChange(InPlaceDelayedFieldChangeOp {
                    materialized_size,
                    ..
                }) => *materialized_size,
                ResourceGroupInPlaceDelayedFieldChange(
                    ResourceGroupInPlaceDelayedFieldChangeOp {
                        materialized_size, ..
                    },
                ) => *materialized_size,
                WriteResourceGroup(GroupWrite {
                    prev_group_size, ..
                }) => *prev_group_size,
            }
        } else {
            match self {
                Write(_)
                | WriteWithDelayedFields(WriteWithDelayedFieldsOp { .. })
                | InPlaceDelayedFieldChange(_)
                | ResourceGroupInPlaceDelayedFieldChange(_) => {
                    executor_view.get_resource_state_value_size(state_key)?
                },
                WriteResourceGroup(GroupWrite {
                    prev_group_size, ..
                }) => *prev_group_size,
            }
        };
        Ok(size)
    }

    /// Deposit amount is inserted into metadata at a different time than the WriteOp is created.
    /// So this method is needed to be able to update metadata generically across different variants.
    pub fn metadata_mut(&mut self) -> &mut StateValueMetadata {
        use AbstractResourceWriteOp::*;
        match self {
            Write(write_op)
            | WriteWithDelayedFields(WriteWithDelayedFieldsOp { write_op, .. })
            | WriteResourceGroup(GroupWrite {
                metadata_op: write_op,
                ..
            }) => write_op.metadata_mut(),
            InPlaceDelayedFieldChange(InPlaceDelayedFieldChangeOp { metadata, .. })
            | ResourceGroupInPlaceDelayedFieldChange(ResourceGroupInPlaceDelayedFieldChangeOp {
                metadata,
                ..
            }) => metadata,
        }
    }

    pub fn from_resource_write_with_maybe_layout(
        write_op: WriteOp,
        maybe_layout: Option<TriompheArc<MoveTypeLayout>>,
    ) -> Self {
        match maybe_layout {
            Some(layout) => {
                let materialized_size = write_op.write_op_size().write_len();
                Self::WriteWithDelayedFields(WriteWithDelayedFieldsOp {
                    write_op,
                    layout,
                    materialized_size,
                })
            },
            None => Self::Write(write_op),
        }
    }
}

/// Describes an update to a resource group granularly, with WriteOps to affected
/// member resources of the group, as well as a separate WriteOp for metadata and size.
#[derive(PartialEq, Eq, Clone, Debug)]
pub struct GroupWrite {
    /// Op of the correct kind (creation / modification / deletion) and metadata, and
    /// the size of the group after the updates encoded in the bytes (no bytes for
    /// deletion). Relevant during block execution, where the information read to
    /// derive metadata_op will be validated during parallel execution to make sure
    /// it is correct, and the bytes will be replaced after the transaction is committed
    /// with correct serialized group update to obtain storage WriteOp.
    pub metadata_op: WriteOp,
    /// Updates to individual group members. WriteOps are 'legacy', i.e. no metadata.
    /// If the metadata_op is a deletion, all (correct) inner_ops should be deletions,
    /// and if metadata_op is a creation, then there may not be a creation inner op.
    /// Not vice versa, e.g. for deleted inner ops, other untouched resources may still
    /// exist in the group. Note: During parallel block execution, due to speculative
    /// reads, this invariant may be violated (and lead to speculation error if observed)
    /// but guaranteed to fail validation and lead to correct re-execution in that case.
    pub(crate) inner_ops: BTreeMap<StructTag, (WriteOp, Option<TriompheArc<MoveTypeLayout>>)>,
    /// Group size as used for gas charging, None if (metadata_)op is Deletion.
    pub(crate) maybe_group_op_size: Option<ResourceGroupSize>,
    // TODO: consider Option<u64> to be able to represent a previously non-existent group,
    //       if useful
    pub(crate) prev_group_size: u64,
}

impl GroupWrite {
    /// Creates a group write and ensures that the format is correct: in particular,
    /// sets the bytes of a non-deletion metadata_op by serializing the provided size,
    /// and ensures inner ops do not contain any metadata.
    pub fn new(
        metadata_op: WriteOp,
        inner_ops: BTreeMap<StructTag, (WriteOp, Option<TriompheArc<MoveTypeLayout>>)>,
        group_size: ResourceGroupSize,
        prev_group_size: u64,
    ) -> Self {
        assert!(
            metadata_op.bytes().is_none() || metadata_op.bytes().unwrap().is_empty(),
            "Metadata op should have empty bytes. metadata_op: {:?}",
            metadata_op
        );
        for (v, _layout) in inner_ops.values() {
            assert!(
                v.metadata().is_none(),
                "Group inner ops must have no metadata",
            );
        }

        let maybe_group_op_size = (!metadata_op.is_deletion()).then_some(group_size);

        Self {
            metadata_op,
            inner_ops,
            maybe_group_op_size,
            prev_group_size,
        }
    }

    /// Utility method that extracts the serialized group size from metadata_op. Returns
    /// None if group is being deleted, otherwise asserts on deserializing the size.
    pub fn maybe_group_op_size(&self) -> Option<ResourceGroupSize> {
        self.maybe_group_op_size
    }

    pub fn prev_group_size(&self) -> u64 {
        self.prev_group_size
    }

    pub fn metadata_op(&self) -> &WriteOp {
        &self.metadata_op
    }

    pub fn inner_ops(
        &self,
    ) -> &BTreeMap<StructTag, (WriteOp, Option<TriompheArc<MoveTypeLayout>>)> {
        &self.inner_ops
    }
}

#[derive(PartialEq, Eq, Clone, Debug)]
/// Note that write_op can be a Deletion, as long as the Move type layout contains
/// a delayed field. This simplifies squashing session outputs, in particular.
pub struct WriteWithDelayedFieldsOp {
    pub write_op: WriteOp,
    pub layout: TriompheArc<MoveTypeLayout>,
    pub materialized_size: Option<u64>,
}

/// Actual information on which delayed fields were read is unnecessary
/// in the current implementation, as we need to materialize the whole value anyways.
///
/// If future implementation needs those - they can be added.
#[derive(PartialEq, Eq, Clone, Debug)]
pub struct InPlaceDelayedFieldChangeOp {
    pub layout: TriompheArc<MoveTypeLayout>,
    pub materialized_size: u64,
    pub metadata: StateValueMetadata,
}

/// Actual information of which individual tag has delayed fields was read,
/// or what those fields are unnecessary in the current implementation.
/// That is the case, because we need to traverse and materialize all tags anyways.
///
/// If future implementation needs those - they can be added.
#[derive(PartialEq, Eq, Clone, Debug)]
pub struct ResourceGroupInPlaceDelayedFieldChangeOp {
    pub materialized_size: u64,
    pub metadata: StateValueMetadata,
}
