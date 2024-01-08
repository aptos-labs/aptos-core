// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use aptos_types::{
    state_store::state_value::StateValueMetadata,
    write_set::{TransactionWrite, WriteOp, WriteOpSize},
};
use move_core_types::{language_storage::StructTag, value::MoveTypeLayout};
use std::{collections::BTreeMap, sync::Arc};

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
            Write(write) => write.into(),
            WriteWithDelayedFields(WriteWithDelayedFieldsOp {
                write_op,
                materialized_size,
                ..
            })
            | WriteResourceGroup(GroupWrite {
                metadata_op: write_op,
                maybe_group_op_size: materialized_size,
                ..
            }) => {
                use WriteOp::*;
                match write_op {
                    Creation { .. } => WriteOpSize::Creation {
                        write_len: materialized_size.expect("Creation must have size"),
                    },
                    Modification { .. } => WriteOpSize::Modification {
                        write_len: materialized_size.expect("Modification must have size"),
                    },
                    Deletion { .. } => WriteOpSize::Deletion,
                }
            },
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

    /// Deposit amount is inserted into metadata at a different time than the WriteOp is created.
    /// So this method is needed to be able to update metadata generically across different variants.
    pub fn get_metadata_mut(&mut self) -> &mut StateValueMetadata {
        use AbstractResourceWriteOp::*;
        match self {
            Write(write_op)
            | WriteWithDelayedFields(WriteWithDelayedFieldsOp { write_op, .. })
            | WriteResourceGroup(GroupWrite {
                metadata_op: write_op,
                ..
            })
            | ResourceGroupInPlaceDelayedFieldChange(ResourceGroupInPlaceDelayedFieldChangeOp {
                metadata_op: write_op,
                ..
            }) => write_op.get_metadata_mut(),
            InPlaceDelayedFieldChange(InPlaceDelayedFieldChangeOp { metadata, .. }) => metadata,
        }
    }

    pub fn from_resource_write_with_maybe_layout(
        write_op: WriteOp,
        maybe_layout: Option<Arc<MoveTypeLayout>>,
    ) -> Self {
        if let Some(layout) = maybe_layout {
            let materialized_size = WriteOpSize::from(&write_op).write_len();
            Self::WriteWithDelayedFields(WriteWithDelayedFieldsOp {
                write_op,
                layout,
                materialized_size,
            })
        } else {
            Self::Write(write_op)
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
    pub(crate) inner_ops: BTreeMap<StructTag, (WriteOp, Option<Arc<MoveTypeLayout>>)>,
    /// Group size as used for gas charging, None if (metadata_)op is Deletion.
    pub(crate) maybe_group_op_size: Option<u64>,
}

impl GroupWrite {
    /// Creates a group write and ensures that the format is correct: in particular,
    /// sets the bytes of a non-deletion metadata_op by serializing the provided size,
    /// and ensures inner ops do not contain any metadata.
    pub fn new(
        metadata_op: WriteOp,
        inner_ops: BTreeMap<StructTag, (WriteOp, Option<Arc<MoveTypeLayout>>)>,
        group_size: u64,
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
        }
    }

    /// Utility method that extracts the serialized group size from metadata_op. Returns
    /// None if group is being deleted, otherwise asserts on deserializing the size.
    pub fn maybe_group_op_size(&self) -> Option<u64> {
        self.maybe_group_op_size
    }

    pub fn metadata_op(&self) -> &WriteOp {
        &self.metadata_op
    }

    pub fn inner_ops(&self) -> &BTreeMap<StructTag, (WriteOp, Option<Arc<MoveTypeLayout>>)> {
        &self.inner_ops
    }
}

#[derive(PartialEq, Eq, Clone, Debug)]
pub struct WriteWithDelayedFieldsOp {
    pub write_op: WriteOp,
    pub layout: Arc<MoveTypeLayout>,
    pub materialized_size: Option<u64>,
}

/// Actual information on which delayed fields were read is unnecessary
/// in the current implementation, as we need to materialize the whole value anyways.
///
/// If future implementation needs those - they can be added.
#[derive(PartialEq, Eq, Clone, Debug)]
pub struct InPlaceDelayedFieldChangeOp {
    pub layout: Arc<MoveTypeLayout>,
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
    pub metadata_op: WriteOp,
    pub materialized_size: u64,
}
