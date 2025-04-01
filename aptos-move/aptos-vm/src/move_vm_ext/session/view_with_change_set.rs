// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use aptos_aggregator::{
    bounded_math::{BoundedMath, SignedU128},
    delayed_change::{ApplyBase, DelayedApplyChange, DelayedChange},
    delta_change_set::DeltaWithMax,
    resolver::{TAggregatorV1View, TDelayedFieldView},
    types::{DelayedFieldValue, DelayedFieldsSpeculativeError},
};
use aptos_types::{
    error::{code_invariant_error, expect_ok, PanicError, PanicOr},
    state_store::{
        errors::StateViewError,
        state_key::StateKey,
        state_storage_usage::StateStorageUsage,
        state_value::{StateValue, StateValueMetadata},
        StateViewId,
    },
    write_set::TransactionWrite,
};
use aptos_vm_types::{
    abstract_write_op::{AbstractResourceWriteOp, WriteWithDelayedFieldsOp},
    change_set::{randomly_check_layout_matches, VMChangeSet},
    resolver::{
        ExecutorView, ResourceGroupSize, ResourceGroupView, StateStorageView, TResourceGroupView,
        TResourceView,
    },
};
use bytes::Bytes;
use move_binary_format::errors::PartialVMResult;
use move_core_types::{language_storage::StructTag, value::MoveTypeLayout};
use move_vm_types::delayed_values::delayed_field_id::DelayedFieldID;
use std::{
    collections::{BTreeMap, HashMap, HashSet},
    sync::Arc,
};

/// Adapter to allow resolving the calls to `ExecutorView` via change set.
pub struct ExecutorViewWithChangeSet<'r> {
    base_executor_view: &'r dyn ExecutorView,
    base_resource_group_view: &'r dyn ResourceGroupView,
    pub(crate) change_set: VMChangeSet,
}

impl<'r> ExecutorViewWithChangeSet<'r> {
    pub(crate) fn new(
        base_executor_view: &'r dyn ExecutorView,
        base_resource_group_view: &'r dyn ResourceGroupView,
        change_set: VMChangeSet,
    ) -> Self {
        Self {
            base_executor_view,
            base_resource_group_view,
            change_set,
        }
    }
}

impl<'r> TAggregatorV1View for ExecutorViewWithChangeSet<'r> {
    type Identifier = StateKey;

    fn get_aggregator_v1_state_value(
        &self,
        id: &Self::Identifier,
    ) -> PartialVMResult<Option<StateValue>> {
        match self.change_set.aggregator_v1_delta_set().get(id) {
            Some(delta_op) => Ok(self
                .base_executor_view
                .try_convert_aggregator_v1_delta_into_write_op(id, delta_op)?
                .as_state_value()),
            None => match self.change_set.aggregator_v1_write_set().get(id) {
                Some(write_op) => Ok(write_op.as_state_value()),
                None => self.base_executor_view.get_aggregator_v1_state_value(id),
            },
        }
    }
}

impl<'r> TDelayedFieldView for ExecutorViewWithChangeSet<'r> {
    type Identifier = DelayedFieldID;
    type ResourceGroupTag = StructTag;
    type ResourceKey = StateKey;

    fn get_delayed_field_value(
        &self,
        id: &Self::Identifier,
    ) -> Result<DelayedFieldValue, PanicOr<DelayedFieldsSpeculativeError>> {
        use DelayedChange::*;

        match self.change_set.delayed_field_change_set().get(id) {
            Some(Create(value)) => Ok(value.clone()),
            Some(Apply(apply)) => {
                let base_value = match apply.get_apply_base_id(id) {
                    ApplyBase::Previous(base_id) => {
                        self.base_executor_view.get_delayed_field_value(&base_id)?
                    },
                    // For Current, call on self to include current change!
                    ApplyBase::Current(base_id) => {
                        // avoid infinite loop
                        if &base_id == id {
                            return Err(code_invariant_error(format!(
                                "Base id is Current(self) for {:?} : Apply({:?})",
                                id, apply
                            ))
                            .into());
                        }
                        self.get_delayed_field_value(&base_id)?
                    },
                };
                Ok(apply.apply_to_base(base_value)?)
            },
            None => self.base_executor_view.get_delayed_field_value(id),
        }
    }

    fn delayed_field_try_add_delta_outcome(
        &self,
        id: &Self::Identifier,
        base_delta: &SignedU128,
        delta: &SignedU128,
        max_value: u128,
    ) -> Result<bool, PanicOr<DelayedFieldsSpeculativeError>> {
        use DelayedChange::*;

        let math = BoundedMath::new(max_value);
        match self.change_set.delayed_field_change_set().get(id) {
            Some(Create(value)) => {
                let prev_value = expect_ok(math.unsigned_add_delta(value.clone().into_aggregator_value()?, base_delta))?;
                Ok(math.unsigned_add_delta(prev_value, delta).is_ok())
            }
            Some(Apply(DelayedApplyChange::AggregatorDelta { delta: change_delta })) => {
                let merged = &DeltaWithMax::create_merged_delta(
                    &DeltaWithMax::new(*base_delta, max_value),
                    change_delta)?;
                self.base_executor_view.delayed_field_try_add_delta_outcome(
                    id,
                    &merged.get_update(),
                    delta,
                    max_value)
            },
            Some(Apply(_)) => Err(code_invariant_error(
                "Cannot call delayed_field_try_add_delta_outcome on non-AggregatorDelta Apply change",
            ).into()),
            None => self.base_executor_view.delayed_field_try_add_delta_outcome(id, base_delta, delta, max_value)
        }
    }

    fn generate_delayed_field_id(&self, width: u32) -> Self::Identifier {
        self.base_executor_view.generate_delayed_field_id(width)
    }

    fn validate_delayed_field_id(&self, id: &Self::Identifier) -> Result<(), PanicError> {
        self.base_executor_view.validate_delayed_field_id(id)
    }

    fn get_reads_needing_exchange(
        &self,
        delayed_write_set_keys: &HashSet<Self::Identifier>,
        skip: &HashSet<Self::ResourceKey>,
    ) -> Result<
        BTreeMap<Self::ResourceKey, (StateValueMetadata, u64, Arc<MoveTypeLayout>)>,
        PanicError,
    > {
        self.base_executor_view
            .get_reads_needing_exchange(delayed_write_set_keys, skip)
    }

    fn get_group_reads_needing_exchange(
        &self,
        delayed_write_set_keys: &HashSet<Self::Identifier>,
        skip: &HashSet<Self::ResourceKey>,
    ) -> PartialVMResult<BTreeMap<Self::ResourceKey, (StateValueMetadata, u64)>> {
        self.base_executor_view
            .get_group_reads_needing_exchange(delayed_write_set_keys, skip)
    }
}

impl<'r> TResourceView for ExecutorViewWithChangeSet<'r> {
    type Key = StateKey;
    type Layout = MoveTypeLayout;

    fn get_resource_state_value(
        &self,
        state_key: &Self::Key,
        maybe_layout: Option<&Self::Layout>,
    ) -> PartialVMResult<Option<StateValue>> {
        match self.change_set.resource_write_set().get(state_key) {
            Some(
                AbstractResourceWriteOp::Write(write_op)
                | AbstractResourceWriteOp::WriteWithDelayedFields(WriteWithDelayedFieldsOp {
                    write_op,
                    ..
                }),
            ) => Ok(write_op.as_state_value()),
            // We could either return from the read, or do the base read again.
            Some(AbstractResourceWriteOp::InPlaceDelayedFieldChange(_)) | None => self
                .base_executor_view
                .get_resource_state_value(state_key, maybe_layout),
            Some(AbstractResourceWriteOp::WriteResourceGroup(_))
            | Some(AbstractResourceWriteOp::ResourceGroupInPlaceDelayedFieldChange(_)) => {
                // In case this is a resource group, and feature is enabled that creates these ops,
                // this should never be called.
                // Call to metadata should go through get_resource_state_value_metadata(), and
                // calls to individual tagged resources should go through their trait.
                unreachable!("get_resource_state_value should never be called for resource group");
            },
        }
    }

    fn get_resource_state_value_metadata(
        &self,
        state_key: &Self::Key,
    ) -> PartialVMResult<Option<StateValueMetadata>> {
        match self.change_set.resource_write_set().get(state_key) {
            Some(
                AbstractResourceWriteOp::Write(write_op)
                | AbstractResourceWriteOp::WriteWithDelayedFields(WriteWithDelayedFieldsOp {
                    write_op,
                    ..
                }),
            ) => Ok(write_op.as_state_value_metadata()),
            Some(AbstractResourceWriteOp::WriteResourceGroup(write_op)) => {
                Ok(write_op.metadata_op().as_state_value_metadata())
            },
            // We could either return from the read, or do the base read again.
            Some(AbstractResourceWriteOp::InPlaceDelayedFieldChange(_))
            | Some(AbstractResourceWriteOp::ResourceGroupInPlaceDelayedFieldChange(_))
            | None => self
                .base_executor_view
                .get_resource_state_value_metadata(state_key),
        }
    }
}

impl<'r> TResourceGroupView for ExecutorViewWithChangeSet<'r> {
    type GroupKey = StateKey;
    type Layout = MoveTypeLayout;
    type ResourceTag = StructTag;

    fn resource_group_size(
        &self,
        group_key: &Self::GroupKey,
    ) -> PartialVMResult<ResourceGroupSize> {
        use AbstractResourceWriteOp::*;

        if let Some(size) = self
        .change_set
        .resource_write_set()
        .get(group_key)
        .and_then(|write| match write {
            WriteResourceGroup(group_write) => Some(Ok(group_write.maybe_group_op_size().unwrap_or(ResourceGroupSize::zero_combined()))),
            ResourceGroupInPlaceDelayedFieldChange(_) => None,
            Write(_) | WriteWithDelayedFields(_) | InPlaceDelayedFieldChange(_) => {
                // There should be no collisions, we cannot have group key refer to a resource.
                Some(Err(code_invariant_error(format!("Non-ResourceGroup write found for key in get_resource_from_group call for key {group_key:?}"))))
            },
        })
        .transpose()? {
            return Ok(size);
        }

        self.base_resource_group_view.resource_group_size(group_key)
    }

    fn get_resource_from_group(
        &self,
        group_key: &Self::GroupKey,
        resource_tag: &Self::ResourceTag,
        maybe_layout: Option<&Self::Layout>,
    ) -> PartialVMResult<Option<Bytes>> {
        use AbstractResourceWriteOp::*;

        if let Some((write_op, layout)) = self
            .change_set
            .resource_write_set()
            .get(group_key)
            .and_then(|write| match write {
                WriteResourceGroup(group_write) => Some(Ok(group_write)),
                ResourceGroupInPlaceDelayedFieldChange(_) => None,
                Write(_) | WriteWithDelayedFields(_) | InPlaceDelayedFieldChange(_) => {
                    // There should be no collisions, we cannot have group key refer to a resource.
                    Some(Err(code_invariant_error(format!("Non-ResourceGroup write found for key in get_resource_from_group call for key {group_key:?}"))))
                },
            })
            .transpose()?
            .and_then(|g| g.inner_ops().get(resource_tag))
        {
            randomly_check_layout_matches(maybe_layout, layout.as_deref())?;
            Ok(write_op.extract_raw_bytes())
        } else {
            self.base_resource_group_view.get_resource_from_group(
                group_key,
                resource_tag,
                maybe_layout,
            )
        }
    }

    fn release_group_cache(
        &self,
    ) -> Option<HashMap<Self::GroupKey, BTreeMap<Self::ResourceTag, Bytes>>> {
        unreachable!("Must not be called by RespawnedSession finish");
    }

    fn is_resource_groups_split_in_change_set_capable(&self) -> bool {
        self.base_resource_group_view
            .is_resource_groups_split_in_change_set_capable()
    }
}

impl<'r> StateStorageView for ExecutorViewWithChangeSet<'r> {
    type Key = StateKey;

    fn id(&self) -> StateViewId {
        self.base_executor_view.id()
    }

    fn read_state_value(&self, state_key: &Self::Key) -> Result<(), StateViewError> {
        self.base_executor_view.read_state_value(state_key)
    }

    fn get_usage(&self) -> Result<StateStorageUsage, StateViewError> {
        Err(StateViewError::Other(
            "Unexpected access to get_usage()".to_string(),
        ))
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::{
        data_cache::AsMoveResolver,
        move_vm_ext::resolver::{AsExecutorView, AsResourceGroupView},
    };
    use aptos_aggregator::delta_change_set::{delta_add, serialize};
    use aptos_transaction_simulation::{InMemoryStateStore, SimulationStateStore};
    use aptos_types::{account_address::AccountAddress, write_set::WriteOp};
    use aptos_vm_types::abstract_write_op::GroupWrite;
    use move_core_types::{
        identifier::Identifier,
        language_storage::{StructTag, TypeTag},
    };

    fn key(s: impl ToString) -> StateKey {
        StateKey::raw(s.to_string().as_bytes())
    }

    fn write(v: u128) -> WriteOp {
        WriteOp::legacy_modification(serialize(&v).into())
    }

    fn read_resource(view: &ExecutorViewWithChangeSet, s: impl ToString) -> u128 {
        bcs::from_bytes(&view.get_resource_bytes(&key(s), None).unwrap().unwrap()).unwrap()
    }

    fn read_aggregator(view: &ExecutorViewWithChangeSet, s: impl ToString) -> u128 {
        view.get_aggregator_v1_value(&key(s)).unwrap().unwrap()
    }

    fn read_resource_from_group(
        view: &ExecutorViewWithChangeSet,
        s: impl ToString,
        tag: &StructTag,
    ) -> u128 {
        bcs::from_bytes(
            &view
                .get_resource_from_group(&key(s), tag, None)
                .unwrap()
                .unwrap(),
        )
        .unwrap()
    }

    fn mock_tag_0() -> StructTag {
        StructTag {
            address: AccountAddress::ONE,
            module: Identifier::new("a").unwrap(),
            name: Identifier::new("a").unwrap(),
            type_args: vec![TypeTag::U8],
        }
    }

    fn mock_tag_1() -> StructTag {
        StructTag {
            address: AccountAddress::ONE,
            module: Identifier::new("abcde").unwrap(),
            name: Identifier::new("fgh").unwrap(),
            type_args: vec![TypeTag::U64],
        }
    }

    fn mock_tag_2() -> StructTag {
        StructTag {
            address: AccountAddress::ONE,
            module: Identifier::new("abcdex").unwrap(),
            name: Identifier::new("fghx").unwrap(),
            type_args: vec![TypeTag::U128],
        }
    }

    #[test]
    fn test_change_set_state_view() {
        let state_view = InMemoryStateStore::new();

        state_view
            .set_state_value(
                key("resource_base"),
                StateValue::new_legacy(serialize(&30).into()),
            )
            .unwrap();
        state_view
            .set_state_value(
                key("resource_both"),
                StateValue::new_legacy(serialize(&40).into()),
            )
            .unwrap();

        state_view
            .set_state_value(
                key("aggregator_base"),
                StateValue::new_legacy(serialize(&50).into()),
            )
            .unwrap();
        state_view
            .set_state_value(
                key("aggregator_both"),
                StateValue::new_legacy(serialize(&60).into()),
            )
            .unwrap();
        state_view
            .set_state_value(
                key("aggregator_delta_set"),
                StateValue::new_legacy(serialize(&70).into()),
            )
            .unwrap();

        let tree: BTreeMap<StructTag, Bytes> = BTreeMap::from([
            (mock_tag_0(), serialize(&100).into()),
            (mock_tag_1(), serialize(&200).into()),
        ]);
        state_view
            .set_state_value(
                key("resource_group_base"),
                StateValue::new_legacy(bcs::to_bytes(&tree).unwrap().into()),
            )
            .unwrap();
        state_view
            .set_state_value(
                key("resource_group_both"),
                StateValue::new_legacy(bcs::to_bytes(&tree).unwrap().into()),
            )
            .unwrap();

        let resource_write_set = BTreeMap::from([
            (key("resource_both"), (write(80), None)),
            (key("resource_write_set"), (write(90), None)),
        ]);

        let aggregator_v1_write_set = BTreeMap::from([
            (key("aggregator_both"), write(120)),
            (key("aggregator_write_set"), write(130)),
        ]);

        let aggregator_v1_delta_set =
            BTreeMap::from([(key("aggregator_delta_set"), delta_add(1, 1000))]);

        // TODO[agg_v2](test): Layout hardcoded to None. Test with layout = Some(..)
        let resource_group_write_set = BTreeMap::from([
            (
                key("resource_group_both"),
                GroupWrite::new(
                    WriteOp::legacy_deletion(),
                    BTreeMap::from([
                        (
                            mock_tag_0(),
                            (WriteOp::legacy_modification(serialize(&1000).into()), None),
                        ),
                        (
                            mock_tag_2(),
                            (WriteOp::legacy_modification(serialize(&300).into()), None),
                        ),
                    ]),
                    ResourceGroupSize::zero_combined(),
                    0,
                ),
            ),
            (
                key("resource_group_write_set"),
                GroupWrite::new(
                    WriteOp::legacy_deletion(),
                    BTreeMap::from([(
                        mock_tag_1(),
                        (WriteOp::legacy_modification(serialize(&5000).into()), None),
                    )]),
                    ResourceGroupSize::zero_combined(),
                    0,
                ),
            ),
        ]);

        let change_set = VMChangeSet::new_expanded(
            resource_write_set,
            resource_group_write_set,
            aggregator_v1_write_set,
            aggregator_v1_delta_set,
            BTreeMap::new(),
            BTreeMap::new(),
            BTreeMap::new(),
            vec![],
        )
        .unwrap();

        let resolver = state_view.as_move_resolver();
        let view = ExecutorViewWithChangeSet::new(
            resolver.as_executor_view(),
            resolver.as_resource_group_view(),
            change_set,
        );

        assert_eq!(read_resource(&view, "resource_base"), 30);
        assert_eq!(read_resource(&view, "resource_both"), 80);
        assert_eq!(read_resource(&view, "resource_write_set"), 90);

        assert_eq!(read_aggregator(&view, "aggregator_base"), 50);
        assert_eq!(read_aggregator(&view, "aggregator_both"), 120);
        assert_eq!(read_aggregator(&view, "aggregator_write_set"), 130);
        assert_eq!(read_aggregator(&view, "aggregator_delta_set"), 71);

        assert_eq!(
            read_resource_from_group(&view, "resource_group_base", &mock_tag_0()),
            100
        );
        assert_eq!(
            read_resource_from_group(&view, "resource_group_base", &mock_tag_1()),
            200
        );
        assert_eq!(
            read_resource_from_group(&view, "resource_group_both", &mock_tag_0()),
            1000
        );
        assert_eq!(
            read_resource_from_group(&view, "resource_group_both", &mock_tag_1()),
            200
        );
        assert_eq!(
            read_resource_from_group(&view, "resource_group_both", &mock_tag_2()),
            300
        );
        assert_eq!(
            read_resource_from_group(&view, "resource_group_write_set", &mock_tag_1()),
            5000
        );
    }

    // TODO[agg_v2](test) add delayed field tests
}
