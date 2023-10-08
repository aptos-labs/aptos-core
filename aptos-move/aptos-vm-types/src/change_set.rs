// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::check_change_set::CheckChangeSet;
use aptos_aggregator::{
    delta_change_set::{serialize, DeltaOp},
    resolver::{AggregatorReadMode, AggregatorResolver},
};
use aptos_types::{
    contract_event::ContractEvent,
    state_store::state_key::{StateKey, StateKeyInner},
    transaction::ChangeSet as StorageChangeSet,
    write_set::{TransactionWrite, WriteOp, WriteSetMut},
};
use claims::assert_none;
use move_binary_format::errors::Location;
use move_core_types::{
    language_storage::StructTag,
    vm_status::{err_msg, StatusCode, VMStatus},
};
use std::{collections::HashMap, hash::Hash};

#[derive(PartialEq, Eq, Clone, Debug)]
/// Describes an update to a resource group granularly, with WriteOps to affected
/// member resources of the group, as well as a separate WriteOp for metadata and size.
pub struct GroupWrite {
    /// Op of the correct kind (creation / modification / deletion) and metadata, and
    /// the size of the group after the updates encoded in the bytes (no bytes for
    /// deletion). Relevant during block execution, where the information read to
    /// derive metadata_op will be validated during parallel execution to make sure
    /// it is correct, and the bytes will be replaced after the transaction is committed
    /// with correct serialized group update to obtain storage WriteOp.
    metadata_op: WriteOp,
    /// Updates to individual group members. WriteOps are 'legacy', i.e. no metadata.
    /// If the metadata_op is a deletion, all (correct) inner_ops should be deletions,
    /// and if metadata_op is a creation, then there may not be a creation inner op.
    /// Not vice versa, e.g. for deleted inner ops, other untouched resources may still
    /// exist in the group. Note: During parallel block execution, due to speculative
    /// reads, this invariant may be violated (and lead to speculation error if observed)
    /// but guaranteed to fail validation and lead to correct re-execution in that case.
    inner_ops: HashMap<StructTag, WriteOp>,
}

impl GroupWrite {
    /// Creates a group write and ensures that the format is correct: in particular,
    /// sets the bytes of a non-deletion metadata_op by serializing the provided size,
    /// and ensures inner ops do not contain any metadata.
    pub fn new(
        mut metadata_op: WriteOp,
        group_size: u64,
        inner_ops: HashMap<StructTag, WriteOp>,
    ) -> Self {
        for v in inner_ops.values() {
            assert_none!(v.metadata());
        }

        let encoded_group_size = bcs::to_bytes(&group_size)
            .expect("Must serialize u64 successfully")
            .into();
        metadata_op.set_bytes(encoded_group_size);

        Self {
            metadata_op,
            inner_ops,
        }
    }

    /// Utility method that extracts the serialized group size from metadata_op. Returns
    /// None if group is being deleted, otherwise asserts on deserializing the size.
    pub fn encoded_group_size(&self) -> Option<u64> {
        self.metadata_op
            .bytes()
            .map(|b| bcs::from_bytes::<u64>(b).expect("Must be serialized group size"))
    }

    // TODO: refactor storage fee & refund interfaces to operate on metadata directly,
    // as that would avoid providing &mut to the whole metadata op in here, including
    // bytes that are not raw bytes (encoding group size) and must not be modified.
    pub fn metadata_op_mut(&mut self) -> &mut WriteOp {
        &mut self.metadata_op
    }

    pub fn metadata_op(&self) -> &WriteOp {
        &self.metadata_op
    }

    pub fn inner_ops(&self) -> &HashMap<StructTag, WriteOp> {
        &self.inner_ops
    }
}

/// A change set produced by the VM.
///
/// **WARNING**: Just like VMOutput, this type should only be used inside the
/// VM. For storage backends, use `ChangeSet`.
#[derive(Debug, Clone, Eq, PartialEq)]
pub struct VMChangeSet {
    resource_write_set: HashMap<StateKey, WriteOp>,
    // Prior to adding a dedicated write-set for resource groups, all resource group
    // updates are merged into a single WriteOp included in the resource_write_set.
    resource_group_write_set: HashMap<StateKey, GroupWrite>,
    module_write_set: HashMap<StateKey, WriteOp>,
    aggregator_write_set: HashMap<StateKey, WriteOp>,
    aggregator_delta_set: HashMap<StateKey, DeltaOp>,
    events: Vec<ContractEvent>,
}

macro_rules! squash_writes_pair {
    ($write_entry:ident, $additional_write:ident) => {
        // Squashing creation and deletion is a no-op. In that case, we
        // have to remove the old write op from the write set.
        let noop = !WriteOp::squash($write_entry.get_mut(), $additional_write).map_err(|e| {
            VMStatus::error(
                StatusCode::UNKNOWN_INVARIANT_VIOLATION_ERROR,
                err_msg(format!("Error while squashing two write ops: {}.", e)),
            )
        })?;
        if noop {
            $write_entry.remove();
        }
    };
}

impl VMChangeSet {
    pub fn empty() -> Self {
        Self {
            resource_write_set: HashMap::new(),
            resource_group_write_set: HashMap::new(),
            module_write_set: HashMap::new(),
            aggregator_write_set: HashMap::new(),
            aggregator_delta_set: HashMap::new(),
            events: vec![],
        }
    }

    pub fn new(
        resource_write_set: HashMap<StateKey, WriteOp>,
        resource_group_write_set: HashMap<StateKey, GroupWrite>,
        module_write_set: HashMap<StateKey, WriteOp>,
        aggregator_write_set: HashMap<StateKey, WriteOp>,
        aggregator_delta_set: HashMap<StateKey, DeltaOp>,
        events: Vec<ContractEvent>,
        checker: &dyn CheckChangeSet,
    ) -> anyhow::Result<Self, VMStatus> {
        let change_set = Self {
            resource_write_set,
            resource_group_write_set,
            module_write_set,
            aggregator_write_set,
            aggregator_delta_set,
            events,
        };

        // Returns an error if structure of the change set is not valid,
        // e.g. the size in bytes is too large.
        checker.check_change_set(&change_set)?;
        Ok(change_set)
    }

    /// Builds a new change set from the storage representation.
    ///
    /// **WARNING**: Has complexity O(#write_ops) because we need to iterate
    /// over blobs and split them into resources or modules. Only used to
    /// support transactions with write-set payload.
    ///
    /// Note: does not separate out individual resource group updates.
    pub fn try_from_storage_change_set(
        change_set: StorageChangeSet,
        checker: &dyn CheckChangeSet,
    ) -> anyhow::Result<Self, VMStatus> {
        let (write_set, events) = change_set.into_inner();

        // There should be no aggregator writes if we have a change set from
        // storage.
        let mut resource_write_set = HashMap::new();
        let mut module_write_set = HashMap::new();

        for (state_key, write_op) in write_set {
            if matches!(state_key.inner(), StateKeyInner::AccessPath(ap) if ap.is_code()) {
                module_write_set.insert(state_key, write_op);
            } else {
                // TODO(aggregator) While everything else must be a resource, first
                // version of aggregators is implemented as a table item. Revisit when
                // we split MVHashMap into data and aggregators.
                resource_write_set.insert(state_key, write_op);
            }
        }

        let change_set = Self {
            resource_write_set,
            resource_group_write_set: HashMap::new(),
            module_write_set,
            aggregator_write_set: HashMap::new(),
            aggregator_delta_set: HashMap::new(),
            events,
        };
        checker.check_change_set(&change_set)?;
        Ok(change_set)
    }

    pub(crate) fn into_storage_change_set_unchecked(self) -> StorageChangeSet {
        let Self {
            resource_write_set,
            resource_group_write_set: _,
            module_write_set,
            aggregator_write_set,
            aggregator_delta_set: _,
            events,
        } = self;

        let mut write_set_mut = WriteSetMut::default();
        write_set_mut.extend(resource_write_set);
        write_set_mut.extend(module_write_set);
        write_set_mut.extend(aggregator_write_set);

        let write_set = write_set_mut
            .freeze()
            .expect("Freezing a WriteSet does not fail.");
        StorageChangeSet::new(write_set, events)
    }

    /// Converts VM-native change set into its storage representation with fully
    /// serialized changes. The conversion fails if:
    /// - deltas are not materialized.
    /// - resource group writes are not (combined &) converted to resource writes.
    pub fn try_into_storage_change_set(self) -> anyhow::Result<StorageChangeSet, VMStatus> {
        if !self.aggregator_delta_set.is_empty() {
            return Err(VMStatus::error(
                StatusCode::DATA_FORMAT_ERROR,
                err_msg(
                    "Cannot convert from VMChangeSet with non-materialized deltas to ChangeSet.",
                ),
            ));
        }
        if !self.resource_group_write_set.is_empty() {
            return Err(VMStatus::error(
                StatusCode::DATA_FORMAT_ERROR,
                err_msg(
                    "Cannot convert from VMChangeSet with non-combined resource group changes.",
                ),
            ));
        }
        Ok(self.into_storage_change_set_unchecked())
    }

    pub fn write_set_iter(&self) -> impl Iterator<Item = (&StateKey, &WriteOp)> {
        self.resource_write_set()
            .iter()
            .chain(self.module_write_set().iter())
            .chain(self.aggregator_v1_write_set().iter())
    }

    pub fn num_write_ops(&self) -> usize {
        self.resource_write_set().len()
            + self.module_write_set().len()
            + self.aggregator_v1_write_set().len()
    }

    pub fn write_set_iter_mut(&mut self) -> impl Iterator<Item = (&StateKey, &mut WriteOp)> {
        self.resource_write_set
            .iter_mut()
            .chain(self.module_write_set.iter_mut())
            .chain(self.aggregator_write_set.iter_mut())
    }

    pub fn group_write_set_iter_mut(
        &mut self,
    ) -> impl Iterator<Item = (&StateKey, &mut GroupWrite)> {
        self.resource_group_write_set.iter_mut()
    }

    pub fn resource_write_set(&self) -> &HashMap<StateKey, WriteOp> {
        &self.resource_write_set
    }

    pub fn resource_group_write_set(&self) -> &HashMap<StateKey, GroupWrite> {
        &self.resource_group_write_set
    }

    pub fn module_write_set(&self) -> &HashMap<StateKey, WriteOp> {
        &self.module_write_set
    }

    // Called by `try_into_transaction_output_with_materialized_writes` only.
    pub(crate) fn extend_aggregator_write_set(
        &mut self,
        additional_aggregator_writes: impl Iterator<Item = (StateKey, WriteOp)>,
    ) {
        self.aggregator_write_set
            .extend(additional_aggregator_writes)
    }

    pub fn aggregator_v1_write_set(&self) -> &HashMap<StateKey, WriteOp> {
        &self.aggregator_write_set
    }

    pub fn aggregator_v1_delta_set(&self) -> &HashMap<StateKey, DeltaOp> {
        &self.aggregator_delta_set
    }

    pub fn events(&self) -> &[ContractEvent] {
        &self.events
    }

    /// Materializes this change set: all deltas are converted into writes and
    /// are combined with existing aggregator writes.
    pub fn try_materialize(
        self,
        resolver: &impl AggregatorResolver,
    ) -> anyhow::Result<Self, VMStatus> {
        let Self {
            resource_write_set,
            resource_group_write_set,
            module_write_set,
            mut aggregator_write_set,
            aggregator_delta_set,
            events,
        } = self;

        let into_write =
            |(state_key, delta): (StateKey, DeltaOp)| -> anyhow::Result<(StateKey, WriteOp), VMStatus> {
                // Materialization is needed when committing a transaction, so
                // we need precise mode to compute the true value of an
                // aggregator.
                let write = resolver.try_convert_aggregator_v1_delta_into_write_op(&state_key, &delta, AggregatorReadMode::Precise)?;
                Ok((state_key, write))
            };

        let materialized_aggregator_delta_set =
            aggregator_delta_set
                .into_iter()
                .map(into_write)
                .collect::<anyhow::Result<HashMap<StateKey, WriteOp>, VMStatus>>()?;
        aggregator_write_set.extend(materialized_aggregator_delta_set);

        Ok(Self {
            resource_write_set,
            resource_group_write_set,
            module_write_set,
            aggregator_write_set,
            aggregator_delta_set: HashMap::new(),
            events,
        })
    }

    fn squash_additional_aggregator_changes(
        aggregator_write_set: &mut HashMap<StateKey, WriteOp>,
        aggregator_delta_set: &mut HashMap<StateKey, DeltaOp>,
        additional_aggregator_write_set: HashMap<StateKey, WriteOp>,
        additional_aggregator_delta_set: HashMap<StateKey, DeltaOp>,
    ) -> anyhow::Result<(), VMStatus> {
        use std::collections::hash_map::Entry::{Occupied, Vacant};
        use WriteOp::*;

        // First, squash deltas.
        for (state_key, additional_delta_op) in additional_aggregator_delta_set {
            if let Some(write_op) = aggregator_write_set.get_mut(&state_key) {
                // In this case, delta follows a write op.
                match write_op {
                    Creation(data)
                    | Modification(data)
                    | CreationWithMetadata { data, .. }
                    | ModificationWithMetadata { data, .. } => {
                        // Apply delta on top of creation or modification.
                        // TODO(aggregator): This will not be needed anymore once aggregator
                        // change sets carry non-serialized information.
                        let base: u128 = bcs::from_bytes(data)
                            .expect("Deserializing into an aggregator value always succeeds");
                        let value = additional_delta_op
                            .apply_to(base)
                            .map_err(|e| e.finish(Location::Undefined).into_vm_status())?;
                        *data = serialize(&value).into();
                    },
                    Deletion | DeletionWithMetadata { .. } => {
                        // This case (applying a delta to deleted item) should
                        // never happen. Let's still return an error instead of
                        // panicking.
                        return Err(VMStatus::error(
                            StatusCode::UNKNOWN_INVARIANT_VIOLATION_ERROR,
                            err_msg("Cannot squash delta which was already deleted."),
                        ));
                    },
                }
            } else {
                // Otherwise, this is a either a new delta or an additional delta
                // for the same state key.
                match aggregator_delta_set.entry(state_key) {
                    Occupied(entry) => {
                        // In this case, we need to merge the new incoming delta
                        // to the existing delta, ensuring the strict ordering.
                        entry
                            .into_mut()
                            .merge_with_next_delta(additional_delta_op)
                            .map_err(|e| e.finish(Location::Undefined).into_vm_status())?;
                    },
                    Vacant(entry) => {
                        // We see this delta for the first time, so simply add it
                        // to the set.
                        entry.insert(additional_delta_op);
                    },
                }
            }
        }

        // Next, squash write ops.
        for (state_key, additional_write_op) in additional_aggregator_write_set {
            match aggregator_write_set.entry(state_key) {
                Occupied(mut entry) => {
                    squash_writes_pair!(entry, additional_write_op);
                },
                Vacant(entry) => {
                    // This is a new write op. It can overwrite a delta so we
                    // have to make sure we remove such a delta from the set in
                    // this case.
                    let removed_delta = aggregator_delta_set.remove(entry.key());

                    // We cannot create after modification with a delta!
                    if removed_delta.is_some() && additional_write_op.is_creation() {
                        return Err(VMStatus::error(
                            StatusCode::UNKNOWN_INVARIANT_VIOLATION_ERROR,
                            err_msg("Cannot create a resource after modification with a delta."),
                        ));
                    }

                    entry.insert(additional_write_op);
                },
            }
        }

        Ok(())
    }

    fn squash_additional_writes<K: Hash + Eq + PartialEq>(
        write_set: &mut HashMap<K, WriteOp>,
        additional_write_set: HashMap<K, WriteOp>,
    ) -> anyhow::Result<(), VMStatus> {
        use std::collections::hash_map::Entry::{Occupied, Vacant};

        for (key, additional_write_op) in additional_write_set.into_iter() {
            match write_set.entry(key) {
                Occupied(mut entry) => {
                    squash_writes_pair!(entry, additional_write_op);
                },
                Vacant(entry) => {
                    entry.insert(additional_write_op);
                },
            }
        }
        Ok(())
    }

    fn squash_group_writes(
        write_set: &mut HashMap<StateKey, GroupWrite>,
        additional_write_set: HashMap<StateKey, GroupWrite>,
    ) -> anyhow::Result<(), VMStatus> {
        use std::collections::hash_map::Entry::{Occupied, Vacant};

        for (key, additional_update) in additional_write_set.into_iter() {
            match write_set.entry(key) {
                Occupied(mut group_entry) => {
                    let GroupWrite {
                        metadata_op: additional_metadata_op,
                        inner_ops: additional_inner_ops,
                    } = additional_update;

                    // Squashing creation and deletion is a no-op. In that case, we have to
                    // remove the old GroupWrite from the group write set.
                    let noop = !WriteOp::squash(
                        &mut group_entry.get_mut().metadata_op,
                        additional_metadata_op,
                    )
                    .map_err(|e| {
                        VMStatus::error(
                            StatusCode::UNKNOWN_INVARIANT_VIOLATION_ERROR,
                            err_msg(format!(
                                "Error while squashing two group write metadata ops: {}.",
                                e
                            )),
                        )
                    })?;
                    if noop {
                        group_entry.remove();
                    } else {
                        Self::squash_additional_writes(
                            &mut group_entry.get_mut().inner_ops,
                            additional_inner_ops,
                        )?;
                    }
                },
                Vacant(entry) => {
                    entry.insert(additional_update);
                },
            }
        }
        Ok(())
    }

    pub fn squash_additional_change_set(
        &mut self,
        additional_change_set: Self,
        checker: &dyn CheckChangeSet,
    ) -> anyhow::Result<(), VMStatus> {
        let Self {
            resource_write_set: additional_resource_write_set,
            resource_group_write_set: additional_resource_group_write_set,
            module_write_set: additional_module_write_set,
            aggregator_write_set: additional_aggregator_write_set,
            aggregator_delta_set: additional_aggregator_delta_set,
            events: additional_events,
        } = additional_change_set;

        Self::squash_additional_aggregator_changes(
            &mut self.aggregator_write_set,
            &mut self.aggregator_delta_set,
            additional_aggregator_write_set,
            additional_aggregator_delta_set,
        )?;
        Self::squash_additional_writes(
            &mut self.resource_write_set,
            additional_resource_write_set,
        )?;
        Self::squash_group_writes(
            &mut self.resource_group_write_set,
            additional_resource_group_write_set,
        )?;
        Self::squash_additional_writes(&mut self.module_write_set, additional_module_write_set)?;
        self.events.extend(additional_events);

        checker.check_change_set(self)
    }

    pub fn has_creation(&self) -> bool {
        use WriteOp::*;
        self.write_set_iter()
            .any(|(_key, op)| matches!(op, Creation(..) | CreationWithMetadata { .. }))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tests::utils::{
        mock_tag_0, mock_tag_1, mock_tag_2, raw_metadata, write_op_with_metadata,
    };
    use bytes::Bytes;
    use claims::{assert_err, assert_ok, assert_some_eq};
    use test_case::test_case;

    macro_rules! assert_group_write_size {
        ($op:expr, $s:expr, $exp:expr) => {{
            let group_write = GroupWrite::new($op, $s, HashMap::new());
            assert_eq!(group_write.encoded_group_size(), $exp);
        }};
    }

    #[test]
    fn test_group_write_size() {
        // Deletions should lead to size 0.
        assert_group_write_size!(WriteOp::Deletion, 0, None);
        assert_group_write_size!(
            WriteOp::DeletionWithMetadata {
                metadata: raw_metadata(10)
            },
            0,
            None
        );

        let sizes = [20, 100, 45279432, 5];
        assert_group_write_size!(WriteOp::Creation(Bytes::new()), sizes[0], Some(sizes[0]));
        assert_group_write_size!(
            WriteOp::CreationWithMetadata {
                data: Bytes::new(),
                metadata: raw_metadata(20)
            },
            sizes[1],
            Some(sizes[1])
        );
        assert_group_write_size!(
            WriteOp::Modification(Bytes::new()),
            sizes[2],
            Some(sizes[2])
        );
        assert_group_write_size!(
            WriteOp::ModificationWithMetadata {
                data: Bytes::new(),
                metadata: raw_metadata(30)
            },
            sizes[3],
            Some(sizes[3])
        );
    }

    #[test]
    fn test_squash_groups_one_empty() {
        let key_1 = StateKey::raw(vec![1]);
        let key_2 = StateKey::raw(vec![2]);

        let mut base_update = HashMap::new();
        base_update.insert(key_1.clone(), GroupWrite {
            metadata_op: write_op_with_metadata(0, 100),
            inner_ops: HashMap::new(),
        });
        let mut additional_update = HashMap::new();
        additional_update.insert(key_2.clone(), GroupWrite {
            metadata_op: write_op_with_metadata(0, 200),
            inner_ops: HashMap::new(),
        });

        assert_ok!(VMChangeSet::squash_group_writes(
            &mut base_update,
            additional_update
        ));

        assert_eq!(base_update.len(), 2);
        assert_some_eq!(
            base_update.get(&key_1).unwrap().metadata_op.metadata(),
            &raw_metadata(100)
        );
        assert_some_eq!(
            base_update.get(&key_2).unwrap().metadata_op.metadata(),
            &raw_metadata(200)
        );
    }

    #[test_case(0, 1)] // create, modify
    #[test_case(1, 1)] // modify, modify
    #[test_case(1, 2)] // modify, delete
    #[test_case(2, 0)] // delete, create
    fn test_squash_groups_mergeable_metadata(base_type_idx: u8, additional_type_idx: u8) {
        let key = StateKey::raw(vec![0]);

        let mut base_update = HashMap::new();
        let mut additional_update = HashMap::new();
        base_update.insert(key.clone(), GroupWrite {
            metadata_op: write_op_with_metadata(base_type_idx, 100),
            inner_ops: HashMap::new(),
        });
        additional_update.insert(key.clone(), GroupWrite {
            metadata_op: write_op_with_metadata(additional_type_idx, 200),
            inner_ops: HashMap::new(),
        });

        assert_ok!(VMChangeSet::squash_group_writes(
            &mut base_update,
            additional_update
        ));

        assert_eq!(base_update.len(), 1);
        assert_some_eq!(
            base_update.get(&key).unwrap().metadata_op.metadata(),
            // take the original metadata
            &raw_metadata(100)
        );
    }

    #[test_case(0, 0)] // create, create
    #[test_case(1, 0)] // modify, create
    #[test_case(2, 1)] // delete, modify
    #[test_case(2, 2)] // delete, delete
    fn test_squash_groups_error(base_type_idx: u8, additional_type_idx: u8) {
        let key = StateKey::raw(vec![0]);

        let mut base_update = HashMap::new();
        let mut additional_update = HashMap::new();
        base_update.insert(key.clone(), GroupWrite {
            metadata_op: write_op_with_metadata(base_type_idx, 100),
            inner_ops: HashMap::new(),
        });
        additional_update.insert(key.clone(), GroupWrite {
            metadata_op: write_op_with_metadata(additional_type_idx, 200),
            inner_ops: HashMap::new(),
        });

        assert_err!(VMChangeSet::squash_group_writes(
            &mut base_update,
            additional_update
        ));
    }

    #[test]
    fn test_squash_groups_noop() {
        let key = StateKey::raw(vec![0]);

        let mut base_update = HashMap::new();
        let mut additional_update = HashMap::new();
        base_update.insert(key.clone(), GroupWrite {
            metadata_op: write_op_with_metadata(0, 100), // create
            inner_ops: HashMap::new(),
        });
        additional_update.insert(key.clone(), GroupWrite {
            metadata_op: write_op_with_metadata(2, 200), // delete
            inner_ops: HashMap::new(),
        });

        assert_ok!(VMChangeSet::squash_group_writes(
            &mut base_update,
            additional_update
        ));
        assert!(base_update.is_empty(), "Must become a no-op");
    }

    #[test]
    fn test_inner_ops() {
        let key_1 = StateKey::raw(vec![1]);
        let key_2 = StateKey::raw(vec![2]);

        let mut base_update = HashMap::new();
        let mut additional_update = HashMap::new();
        base_update.insert(key_1.clone(), GroupWrite {
            metadata_op: write_op_with_metadata(1, 100),
            inner_ops: HashMap::from([
                (mock_tag_0(), WriteOp::Creation(vec![100].into())),
                (mock_tag_2(), WriteOp::Modification(vec![2].into())),
            ]),
        });
        additional_update.insert(key_1.clone(), GroupWrite {
            metadata_op: write_op_with_metadata(1, 200),
            inner_ops: HashMap::from([
                (mock_tag_0(), WriteOp::Modification(vec![0].into())),
                (mock_tag_1(), WriteOp::Modification(vec![1].into())),
            ]),
        });

        base_update.insert(key_2.clone(), GroupWrite {
            metadata_op: write_op_with_metadata(1, 100),
            inner_ops: HashMap::from([
                (mock_tag_0(), WriteOp::Deletion),
                (mock_tag_1(), WriteOp::Modification(vec![2].into())),
                (mock_tag_2(), WriteOp::Creation(vec![2].into())),
            ]),
        });
        additional_update.insert(key_2.clone(), GroupWrite {
            metadata_op: write_op_with_metadata(1, 200),
            inner_ops: HashMap::from([
                (mock_tag_0(), WriteOp::Creation(vec![0].into())),
                (mock_tag_1(), WriteOp::Deletion),
                (mock_tag_2(), WriteOp::Deletion),
            ]),
        });

        assert_ok!(VMChangeSet::squash_group_writes(
            &mut base_update,
            additional_update
        ));
        assert_eq!(base_update.len(), 2);
        let inner_ops_1 = &base_update.get(&key_1).unwrap().inner_ops;
        assert_eq!(inner_ops_1.len(), 3);
        assert_some_eq!(
            inner_ops_1.get(&mock_tag_0()),
            &WriteOp::Creation(vec![0].into())
        );
        assert_some_eq!(
            inner_ops_1.get(&mock_tag_1()),
            &WriteOp::Modification(vec![1].into())
        );
        assert_some_eq!(
            inner_ops_1.get(&mock_tag_2()),
            &WriteOp::Modification(vec![2].into())
        );
        let inner_ops_2 = &base_update.get(&key_2).unwrap().inner_ops;
        assert_eq!(inner_ops_2.len(), 2);
        assert_some_eq!(
            inner_ops_2.get(&mock_tag_0()),
            &WriteOp::Modification(vec![0].into())
        );
        assert_some_eq!(inner_ops_2.get(&mock_tag_1()), &WriteOp::Deletion);

        let additional_update = HashMap::from([(key_2.clone(), GroupWrite {
            metadata_op: write_op_with_metadata(1, 200),
            inner_ops: HashMap::from([(mock_tag_1(), WriteOp::Deletion)]),
        })]);
        assert_err!(VMChangeSet::squash_group_writes(
            &mut base_update,
            additional_update
        ));
    }
}
