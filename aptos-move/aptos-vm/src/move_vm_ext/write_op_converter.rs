// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::move_vm_ext::{session::BytesWithResourceLayout, AptosMoveResolver};
use aptos_aggregator::delta_change_set::serialize;
use aptos_types::{
    on_chain_config::{CurrentTimeMicroseconds, OnChainConfig},
    state_store::{state_key::StateKey, state_value::StateValueMetadata},
    write_set::{WriteOp, WriteOpSize},
};
use aptos_vm_types::{
    abstract_write_op::{AbstractResourceWriteOp, GroupWrite},
    module_and_script_storage::module_storage::AptosModuleStorage,
    module_write_set::ModuleWrite,
    resource_group_adapter::{
        check_size_and_existence_match, decrement_size_for_remove_tag, group_tagged_resource_size,
        increment_size_for_add_tag,
    },
    write_info_builder::WriteOpInfoBuilder,
};
use bytes::Bytes;
use move_binary_format::errors::{PartialVMError, PartialVMResult};
use move_core_types::{
    effects::{Op as MoveStorageOp, Op},
    language_storage::{ModuleId, StructTag},
    value::MoveTypeLayout,
    vm_status::StatusCode,
};
use move_vm_runtime::Loader;
use move_vm_types::{
    delayed_values::delayed_field_id::DelayedFieldID,
    value_serde::{FunctionValueExtension, ValueSerDeContext},
    values::Value,
};
use std::{
    collections::{BTreeMap, HashSet},
};
use triomphe::Arc as TriompheArc;

pub(crate) struct LegacyWriteOpConverter<'r> {
    remote: &'r dyn AptosMoveResolver,
    new_slot_metadata: Option<StateValueMetadata>,
}

macro_rules! convert_impl {
    ($convert_func_name:ident, $get_metadata_callback:ident) => {
        pub(crate) fn $convert_func_name(
            &self,
            state_key: &StateKey,
            move_storage_op: MoveStorageOp<Bytes>,
            legacy_creation_as_modification: bool,
        ) -> PartialVMResult<WriteOp> {
            let state_value_metadata = self
                .remote
                .as_executor_view()
                .$get_metadata_callback(state_key)?;
            self.convert(
                state_value_metadata,
                move_storage_op,
                legacy_creation_as_modification,
            )
        }
    };
}

impl<'r> LegacyWriteOpConverter<'r> {
    convert_impl!(convert_aggregator, get_aggregator_v1_state_value_metadata);

    pub(crate) fn new(
        remote: &'r dyn AptosMoveResolver,
        is_storage_slot_metadata_enabled: bool,
    ) -> Self {
        let mut new_slot_metadata: Option<StateValueMetadata> = None;
        if is_storage_slot_metadata_enabled {
            if let Some(current_time) = CurrentTimeMicroseconds::fetch_config(remote) {
                // The deposit on the metadata is a placeholder (0), it will be updated later when
                // storage fee is charged.
                new_slot_metadata = Some(StateValueMetadata::placeholder(&current_time));
            }
        }

        Self {
            remote,
            new_slot_metadata,
        }
    }

    pub(crate) fn convert_modules_into_write_ops(
        &self,
        module_storage: &impl AptosModuleStorage,
        verified_module_bundle: impl Iterator<Item = (ModuleId, Bytes)>,
    ) -> PartialVMResult<BTreeMap<StateKey, ModuleWrite<WriteOp>>> {
        let mut writes = BTreeMap::new();
        for (module_id, bytes) in verified_module_bundle {
            let addr = module_id.address();
            let name = module_id.name();

            // INVARIANT:
            //   No need to charge for module metadata access because the write of a module must
            //   have been already charged for when processing module bundle. Here, it is used for
            //   conversion into a write op - if the metadata exists, it is a modification.
            let state_value_metadata =
                module_storage.unmetered_get_module_state_value_metadata(addr, name)?;
            let op = if state_value_metadata.is_some() {
                Op::Modify(bytes)
            } else {
                Op::New(bytes)
            };

            let write_op = self.convert(
                state_value_metadata,
                op,
                // For modules, creation is never a modification.
                false,
            )?;

            let state_key = StateKey::module_id(&module_id);

            // Enforce read-before-write:
            //   Modules can live in global cache, and so the DB may not see a module read even
            //   when it gets republished. This violates read-before-write property. Here, we on
            //   purpose enforce this by registering a read to the DB directly.
            //   Note that we also do it here so that in case of storage errors, only a  single
            //   transaction fails (e.g., if doing this read before commit in block executor we
            //   have no way to alter the transaction outputs at that point).
            self.remote.read_state_value(&state_key).map_err(|err| {
                let msg = format!(
                    "Error when enforcing read-before-write for module {}::{}: {:?}",
                    addr, name, err
                );
                PartialVMError::new(StatusCode::STORAGE_ERROR).with_message(msg)
            })?;

            writes.insert(state_key, ModuleWrite::new(module_id, write_op));
        }
        Ok(writes)
    }

    pub(crate) fn convert_resource(
        &self,
        state_key: &StateKey,
        move_storage_op: MoveStorageOp<BytesWithResourceLayout>,
        legacy_creation_as_modification: bool,
    ) -> PartialVMResult<(WriteOp, Option<TriompheArc<MoveTypeLayout>>)> {
        let state_value_metadata = self
            .remote
            .as_executor_view()
            .get_resource_state_value_metadata(state_key)?;
        let (move_storage_op, layout) = match move_storage_op {
            MoveStorageOp::New((data, layout)) => (MoveStorageOp::New(data), layout),
            MoveStorageOp::Modify((data, layout)) => (MoveStorageOp::Modify(data), layout),
            MoveStorageOp::Delete => (MoveStorageOp::Delete, None),
        };

        let write_op = self.convert(
            state_value_metadata,
            move_storage_op,
            legacy_creation_as_modification,
        )?;
        Ok((write_op, layout))
    }

    pub(crate) fn convert_resource_group_v1(
        &self,
        state_key: &StateKey,
        group_changes: BTreeMap<StructTag, MoveStorageOp<BytesWithResourceLayout>>,
    ) -> PartialVMResult<GroupWrite> {
        // Resource group metadata is stored at the group StateKey, and can be obtained via the
        // same interfaces at for a resource at a given StateKey.
        let state_value_metadata = self
            .remote
            .as_executor_view()
            .get_resource_state_value_metadata(state_key)?;
        // Currently, due to read-before-write and a gas charge on the first read that is based
        // on the group size, this should simply re-read a cached (speculative) group size.
        let pre_group_size = self.remote.resource_group_size(state_key)?;
        check_size_and_existence_match(&pre_group_size, state_value_metadata.is_some(), state_key)?;

        let mut inner_ops = BTreeMap::new();
        let mut post_group_size = pre_group_size;

        for (tag, current_op) in group_changes {
            // We take speculative group size prior to the transaction, and update it based on the change-set.
            // For each tagged resource in the change set, we subtract the previous size tagged resource size,
            // and then add new tagged resource size.
            //
            // The reason we do not instead get and add the sizes of the resources in the group,
            // but not in the change-set, is to avoid creating unnecessary R/W conflicts (the resources
            // in the change-set are already read, but the other resources are not).
            if !matches!(current_op, MoveStorageOp::New(_)) {
                let old_tagged_value_size = self.remote.resource_size_in_group(state_key, &tag)?;
                let old_size = group_tagged_resource_size(&tag, old_tagged_value_size)?;
                decrement_size_for_remove_tag(&mut post_group_size, old_size)?;
            }

            match &current_op {
                MoveStorageOp::Modify((data, _)) | MoveStorageOp::New((data, _)) => {
                    let new_size = group_tagged_resource_size(&tag, data.len())?;
                    increment_size_for_add_tag(&mut post_group_size, new_size)?;
                },
                MoveStorageOp::Delete => {},
            };

            let legacy_op = match current_op {
                MoveStorageOp::Delete => (WriteOp::legacy_deletion(), None),
                MoveStorageOp::Modify((data, maybe_layout)) => {
                    (WriteOp::legacy_modification(data), maybe_layout)
                },
                MoveStorageOp::New((data, maybe_layout)) => {
                    (WriteOp::legacy_creation(data), maybe_layout)
                },
            };
            inner_ops.insert(tag, legacy_op);
        }

        // Create an op to encode the proper kind for resource group operation.
        let metadata_op = if post_group_size.get() == 0 {
            MoveStorageOp::Delete
        } else if pre_group_size.get() == 0 {
            MoveStorageOp::New(Bytes::new())
        } else {
            MoveStorageOp::Modify(Bytes::new())
        };
        Ok(GroupWrite::new(
            self.convert(state_value_metadata, metadata_op, false)?,
            inner_ops,
            post_group_size,
            pre_group_size.get(),
        ))
    }

    fn convert(
        &self,
        state_value_metadata: Option<StateValueMetadata>,
        move_storage_op: MoveStorageOp<Bytes>,
        legacy_creation_as_modification: bool,
    ) -> PartialVMResult<WriteOp> {
        use MoveStorageOp::*;
        let write_op = match (state_value_metadata, move_storage_op) {
            (None, Modify(_) | Delete) => {
                // Possible under speculative execution, returning speculative error waiting for re-execution.
                return Err(
                    PartialVMError::new(StatusCode::SPECULATIVE_EXECUTION_ABORT_ERROR)
                        .with_message(
                            "When converting write op: updating non-existent value.".to_string(),
                        ),
                );
            },
            (Some(_), New(_)) => {
                // Possible under speculative execution, returning speculative error waiting for re-execution.
                return Err(
                    PartialVMError::new(StatusCode::SPECULATIVE_EXECUTION_ABORT_ERROR)
                        .with_message(
                            "When converting write op: Recreating existing value.".to_string(),
                        ),
                );
            },
            (None, New(data)) => match &self.new_slot_metadata {
                None => {
                    if legacy_creation_as_modification {
                        WriteOp::legacy_modification(data)
                    } else {
                        WriteOp::legacy_creation(data)
                    }
                },
                Some(metadata) => WriteOp::creation(data, metadata.clone()),
            },
            (Some(metadata), Modify(data)) => WriteOp::modification(data, metadata),
            (Some(metadata), Delete) => {
                // Inherit metadata even if the feature flags is turned off, for compatibility.
                WriteOp::deletion(metadata)
            },
        };
        Ok(write_op)
    }

    pub(crate) fn convert_aggregator_modification(
        &self,
        state_key: &StateKey,
        value: u128,
    ) -> PartialVMResult<WriteOp> {
        let maybe_existing_metadata = self
            .remote
            .get_aggregator_v1_state_value_metadata(state_key)?;
        let data = serialize(&value).into();

        let op = match maybe_existing_metadata {
            None => {
                match &self.new_slot_metadata {
                    // n.b. Aggregator writes historically did not distinguish Create vs Modify.
                    None => WriteOp::legacy_modification(data),
                    Some(metadata) => WriteOp::creation(data, metadata.clone()),
                }
            },
            Some(metadata) => WriteOp::modification(data, metadata),
        };

        Ok(op)
    }
}

/// Converts Move global storage operations into DB operations, computes their sizes and metadata.
/// Unlike [LegacyWriteOpConverter], does not support "creation as modification" (for gas versions
/// < 3), and disabled storage metadata.
#[allow(dead_code)]
pub(crate) struct WriteOpConverter<'a, LoaderImpl> {
    woc: LegacyWriteOpConverter<'a>,
    loader: &'a LoaderImpl,
    delayed_field_ids: &'a HashSet<DelayedFieldID>,
}

impl<'a, LoaderImpl> WriteOpConverter<'a, LoaderImpl>
where
    LoaderImpl: Loader,
{
    #[allow(dead_code)]
    pub(crate) fn new(
        data_view: &'a dyn AptosMoveResolver,
        loader: &'a LoaderImpl,
        delayed_field_ids: &'a HashSet<DelayedFieldID>,
    ) -> Self {
        Self {
            woc: LegacyWriteOpConverter::new(data_view, true),
            loader,
            delayed_field_ids,
        }
    }

    /// Serializes value into bytes.
    fn serialize(
        &self,
        value: &Value,
        layout: &MoveTypeLayout,
        contains_delayed_fields: bool,
    ) -> PartialVMResult<Bytes> {
        let function_value_extension = self.loader.as_function_value_extension();
        let mut ctx = ValueSerDeContext::new(function_value_extension.max_value_nest_depth())
            .with_func_args_deserialization(&function_value_extension);
        if contains_delayed_fields {
            ctx = ctx.with_delayed_fields_serde();
        }

        ctx.serialize(value, layout)?
            .map(Bytes::from)
            .ok_or_else(|| PartialVMError::new(StatusCode::VALUE_SERIALIZATION_ERROR))
    }

    /// Returns serialized size of a value.
    fn serialized_size(
        &self,
        value: &Value,
        layout: &MoveTypeLayout,
        contains_delayed_fields: bool,
    ) -> PartialVMResult<u64> {
        let function_value_extension = self.loader.as_function_value_extension();
        let mut ctx = ValueSerDeContext::new(function_value_extension.max_value_nest_depth())
            .with_func_args_deserialization(&function_value_extension);
        if contains_delayed_fields {
            ctx = ctx.with_delayed_fields_serde();
        }

        Ok(ctx.serialized_size(value, layout)? as u64)
    }
}

impl<'a, LoaderImpl> WriteOpInfoBuilder for WriteOpConverter<'a, LoaderImpl>
where
    LoaderImpl: Loader,
{
    fn get_resource_metadata_and_size(
        &self,
        key: &StateKey,
        op: MoveStorageOp<&Value>,
        layout: &MoveTypeLayout,
        contains_delayed_fields: bool,
        assert_no_creation: bool,
    ) -> PartialVMResult<(StateValueMetadata, WriteOpSize)> {
        use MoveStorageOp::*;

        let prev_metadata = self
            .woc
            .remote
            .as_executor_view()
            .get_resource_state_value_metadata(key)?;

        Ok(match (prev_metadata, op) {
            (None, New(v)) => {
                if assert_no_creation {
                    return Err(PartialVMError::new_invariant_violation(format!(
                        "Creation for {:?} found where it is not allowed",
                        key
                    )));
                }

                let metadata = match self.woc.new_slot_metadata.as_ref() {
                    // If new slot metadata is not set, it means we were not able to fetch current
                    // time, and need to use the legacy flow.
                    None => StateValueMetadata::none(),
                    Some(metadata) => metadata.clone(),
                };
                let write_len = self.serialized_size(v, layout, contains_delayed_fields)?;
                (metadata, WriteOpSize::Creation { write_len })
            },
            (Some(metadata), Modify(v)) => {
                let write_len = self.serialized_size(v, layout, contains_delayed_fields)?;
                (metadata, WriteOpSize::Creation { write_len })
            },
            (Some(metadata), Delete) => (metadata, WriteOpSize::Deletion),
            (None, Modify(_) | Delete) | (Some(_), New(_)) => {
                // Possible under speculative execution, returning speculative error waiting for
                // re-execution.
                return Err(PartialVMError::new(
                    StatusCode::SPECULATIVE_EXECUTION_ABORT_ERROR,
                ));
            },
        })
    }

    fn get_resource_write_op(
        &self,
        op: MoveStorageOp<&Value>,
        layout: Arc<MoveTypeLayout>,
        contains_delayed_fields: bool,
        metadata: StateValueMetadata,
    ) -> PartialVMResult<AbstractResourceWriteOp> {
        let write_op = match op {
            Op::New(v) => WriteOp::creation(
                self.serialize(v, &layout, contains_delayed_fields)?,
                metadata,
            ),
            Op::Modify(v) => WriteOp::modification(
                self.serialize(v, &layout, contains_delayed_fields)?,
                metadata,
            ),
            Op::Delete => WriteOp::deletion(metadata),
        };
        let layout = contains_delayed_fields.then_some(layout);
        Ok(AbstractResourceWriteOp::from_resource_write_with_maybe_layout(write_op, layout))
    }

    fn get_resource_metadata_and_size_for_read_with_delayed_fields(
        &self,
        key: &StateKey,
    ) -> PartialVMResult<Option<(StateValueMetadata, WriteOpSize)>> {
        // TODO(sessions): optimize this by accessing a single key instead!
        let mut need_exchange = self
            .woc
            .remote
            .as_executor_view()
            .get_reads_needing_exchange(self.delayed_field_ids, &HashSet::new())?;
        Ok(need_exchange
            .remove(key)
            .map(|(metadata, write_len, _)| (metadata, WriteOpSize::Modification { write_len })))
    }

    fn get_group_metadata_and_size_for_read_with_delayed_fields(
        &self,
        key: &StateKey,
    ) -> PartialVMResult<Option<(StateValueMetadata, WriteOpSize)>> {
        // TODO(sessions): optimize this by accessing a single key instead!
        let mut need_exchange = self
            .woc
            .remote
            .as_executor_view()
            .get_group_reads_needing_exchange(self.delayed_field_ids, &HashSet::new())?;
        Ok(need_exchange
            .remove(key)
            .map(|(metadata, write_len)| (metadata, WriteOpSize::Modification { write_len })))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        data_cache::{tests::as_resolver_with_group_size_kind, AsMoveResolver},
        move_vm_ext::resolver::ResourceGroupResolver,
    };
    use aptos_types::{
        account_address::AccountAddress,
        state_store::{state_value::StateValue, MockStateView},
        write_set::TransactionWrite,
    };
    use aptos_vm_environment::environment::AptosEnvironment;
    use aptos_vm_types::{
        module_and_script_storage::AsAptosCodeStorage,
        resource_group_adapter::{group_size_as_sum, GroupSizeKind},
    };
    use claims::{assert_none, assert_ok, assert_some, assert_some_eq};
    use move_binary_format::{
        file_format::empty_module_with_dependencies_and_friends, CompiledModule,
    };
    use move_core_types::{
        identifier::Identifier,
        language_storage::{StructTag, TypeTag},
    };
    use std::collections::HashMap;

    fn raw_metadata(v: u64) -> StateValueMetadata {
        StateValueMetadata::legacy(v, &CurrentTimeMicroseconds { microseconds: v })
    }

    // TODO: Can re-use some of these testing definitions with aptos-vm-types.
    pub(crate) fn mock_tag_0() -> StructTag {
        StructTag {
            address: AccountAddress::ONE,
            module: Identifier::new("a").unwrap(),
            name: Identifier::new("a").unwrap(),
            type_args: vec![TypeTag::U8],
        }
    }

    pub(crate) fn mock_tag_1() -> StructTag {
        StructTag {
            address: AccountAddress::ONE,
            module: Identifier::new("abcde").unwrap(),
            name: Identifier::new("fgh").unwrap(),
            type_args: vec![TypeTag::U64],
        }
    }

    pub(crate) fn mock_tag_2() -> StructTag {
        StructTag {
            address: AccountAddress::ONE,
            module: Identifier::new("abcdex").unwrap(),
            name: Identifier::new("fghx").unwrap(),
            type_args: vec![TypeTag::U128],
        }
    }

    fn module(name: &str) -> (StateKey, Bytes, CompiledModule) {
        let module = empty_module_with_dependencies_and_friends(name, vec![], vec![]);
        let state_key = StateKey::module(module.self_addr(), module.self_name());
        let mut module_bytes = vec![];
        assert_ok!(module.serialize(&mut module_bytes));
        (state_key, module_bytes.into(), module)
    }

    #[test]
    fn test_convert_modules_into_write_ops() {
        // Create a state value with no metadata.
        let (a_state_key, a_bytes, a) = module("a");
        let a_state_value = StateValue::new_legacy(a_bytes.clone());

        // Create a state value with legacy metadata.
        let (b_state_key, b_bytes, b) = module("b");
        let b_state_value = StateValue::new_with_metadata(
            b_bytes.clone(),
            StateValueMetadata::legacy(10, &CurrentTimeMicroseconds { microseconds: 100 }),
        );

        // Create a state value with non-legacy metadata.
        let (c_state_key, c_bytes, c) = module("c");
        let c_state_value = StateValue::new_with_metadata(
            c_bytes.clone(),
            StateValueMetadata::new(20, 30, &CurrentTimeMicroseconds { microseconds: 200 }),
        );

        // Module that does not yet exist.
        let (d_state_key, d_bytes, d) = module("d");

        // Create the configuration time resource in the state as well;
        let current_time = CurrentTimeMicroseconds { microseconds: 300 };
        let state_key = assert_ok!(StateKey::resource(
            CurrentTimeMicroseconds::address(),
            &CurrentTimeMicroseconds::struct_tag()
        ));
        let bytes = assert_ok!(bcs::to_bytes(&current_time));
        let state_value = StateValue::new_legacy(bytes.into());

        // Setting up the state.
        let state_view = MockStateView::new(HashMap::from([
            (state_key, state_value),
            (a_state_key.clone(), a_state_value.clone()),
            (b_state_key.clone(), b_state_value.clone()),
            (c_state_key.clone(), c_state_value.clone()),
        ]));
        let resolver = state_view.as_move_resolver();
        let env = AptosEnvironment::new(&state_view);
        let code_storage = state_view.as_aptos_code_storage(&env);
        // Storage slot metadata is enabled on the mainnet.
        let woc = LegacyWriteOpConverter::new(&resolver, true);

        let modules = vec![
            (a.self_id(), a_bytes.clone()),
            (b.self_id(), b_bytes.clone()),
            (c.self_id(), c_bytes.clone()),
            (d.self_id(), d_bytes.clone()),
        ];

        let results =
            assert_ok!(woc.convert_modules_into_write_ops(&code_storage, modules.into_iter()));
        assert_eq!(results.len(), 4);

        // For `a`, `b`, and `c`, since they exist, metadata is inherited
        // the write op is a creation.

        let a_write = assert_some!(results.get(&a_state_key));
        assert!(a_write.write_op().is_modification());
        assert_eq!(assert_some!(a_write.write_op().bytes()), &a_bytes);
        assert_eq!(
            a_write.write_op().metadata(),
            &a_state_value.into_metadata()
        );

        let b_write = assert_some!(results.get(&b_state_key));
        assert!(b_write.write_op().is_modification());
        assert_eq!(assert_some!(b_write.write_op().bytes()), &b_bytes);
        assert_eq!(
            b_write.write_op().metadata(),
            &b_state_value.into_metadata()
        );

        let c_write = assert_some!(results.get(&c_state_key));
        assert!(c_write.write_op().is_modification());
        assert_eq!(assert_some!(c_write.write_op().bytes()), &c_bytes);
        assert_eq!(
            c_write.write_op().metadata(),
            &c_state_value.into_metadata()
        );

        // Since `d` does not exist, its metadata is a placeholder.
        let d_write = assert_some!(results.get(&d_state_key));
        assert!(d_write.write_op().is_creation());
        assert_eq!(assert_some!(d_write.write_op().bytes()), &d_bytes);
        assert_eq!(
            d_write.write_op().metadata(),
            &StateValueMetadata::placeholder(&current_time)
        )
    }

    // TODO[agg_v2](test) make as_resolver_with_group_size_kind support AsSum
    // #[test]
    #[allow(unused)]
    fn size_computation_delete_modify_ops() {
        let group: BTreeMap<StructTag, Bytes> = BTreeMap::from([
            (mock_tag_0(), vec![1].into()),
            (mock_tag_1(), vec![2, 2].into()),
            (mock_tag_2(), vec![3, 3, 3].into()),
        ]);
        let metadata = raw_metadata(100);
        let key = StateKey::raw(&[0]);

        let data = HashMap::from([(
            key.clone(),
            StateValue::new_with_metadata(bcs::to_bytes(&group).unwrap().into(), metadata.clone()),
        )]);

        let expected_size = group_size_as_sum(
            vec![(&mock_tag_0(), 1), (&mock_tag_1(), 2), (&mock_tag_2(), 3)].into_iter(),
        )
        .unwrap();

        let s = MockStateView::new(data);
        let resolver = as_resolver_with_group_size_kind(&s, GroupSizeKind::AsSum);

        assert_eq!(resolver.resource_group_size(&key).unwrap(), expected_size);
        // TODO[agg_v2](test): Layout hardcoded to None. Test with layout = Some(..)
        let group_changes = BTreeMap::from([
            (mock_tag_0(), MoveStorageOp::Delete),
            (
                mock_tag_2(),
                MoveStorageOp::Modify((vec![5, 5, 5, 5, 5].into(), None)),
            ),
        ]);
        let converter = LegacyWriteOpConverter::new(&resolver, false);
        let group_write = converter
            .convert_resource_group_v1(&key, group_changes)
            .unwrap();

        assert_eq!(group_write.metadata_op().metadata(), &metadata);
        let expected_new_size =
            group_size_as_sum(vec![(&mock_tag_1(), 2), (&mock_tag_2(), 5)].into_iter()).unwrap();
        assert_some_eq!(group_write.maybe_group_op_size(), expected_new_size);
        assert_eq!(group_write.inner_ops().len(), 2);
        assert_some_eq!(
            group_write.inner_ops().get(&mock_tag_0()),
            &(WriteOp::legacy_deletion(), None)
        );
        assert_some_eq!(
            group_write.inner_ops().get(&mock_tag_2()),
            &(
                WriteOp::legacy_modification(vec![5, 5, 5, 5, 5].into()),
                None
            )
        );
    }

    // TODO[agg_v2](test) make as_resolver_with_group_size_kind support AsSum
    // #[test]
    #[allow(unused)]
    fn size_computation_new_op() {
        let group: BTreeMap<StructTag, Bytes> = BTreeMap::from([
            (mock_tag_0(), vec![1].into()),
            (mock_tag_1(), vec![2, 2].into()),
        ]);
        let metadata = raw_metadata(100);
        let key = StateKey::raw(&[0]);

        let data = HashMap::from([(
            key.clone(),
            StateValue::new_with_metadata(bcs::to_bytes(&group).unwrap().into(), metadata.clone()),
        )]);

        let s = MockStateView::new(data);
        let resolver = as_resolver_with_group_size_kind(&s, GroupSizeKind::AsSum);

        let group_changes = BTreeMap::from([(
            mock_tag_2(),
            MoveStorageOp::New((vec![3, 3, 3].into(), None)),
        )]);
        let converter = LegacyWriteOpConverter::new(&resolver, true);
        let group_write = converter
            .convert_resource_group_v1(&key, group_changes)
            .unwrap();

        assert_eq!(group_write.metadata_op().metadata(), &metadata);
        let expected_new_size = group_size_as_sum(
            vec![(&mock_tag_0(), 1), (&mock_tag_1(), 2), (&mock_tag_2(), 3)].into_iter(),
        )
        .unwrap();
        assert_some_eq!(group_write.maybe_group_op_size(), expected_new_size);
        assert_eq!(group_write.inner_ops().len(), 1);
        assert_some_eq!(
            group_write.inner_ops().get(&mock_tag_2()),
            &(WriteOp::legacy_creation(vec![3, 3, 3].into()), None)
        );
    }

    // TODO[agg_v2](test) make as_resolver_with_group_size_kind support AsSum
    // #[test]
    #[allow(unused)]
    fn size_computation_new_group() {
        let s = MockStateView::empty();
        let resolver = as_resolver_with_group_size_kind(&s, GroupSizeKind::AsSum);

        // TODO[agg_v2](test): Layout hardcoded to None. Test with layout = Some(..)
        let group_changes =
            BTreeMap::from([(mock_tag_1(), MoveStorageOp::New((vec![2, 2].into(), None)))]);
        let key = StateKey::raw(&[0]);
        let converter = LegacyWriteOpConverter::new(&resolver, true);
        let group_write = converter
            .convert_resource_group_v1(&key, group_changes)
            .unwrap();

        assert!(group_write.metadata_op().metadata().is_none());
        let expected_new_size = group_size_as_sum(vec![(&mock_tag_1(), 2)].into_iter()).unwrap();
        assert_some_eq!(group_write.maybe_group_op_size(), expected_new_size);
        assert_eq!(group_write.inner_ops().len(), 1);
        assert_some_eq!(
            group_write.inner_ops().get(&mock_tag_1()),
            &(WriteOp::legacy_creation(vec![2, 2].into()), None)
        );
    }

    // TODO[agg_v2](test) make as_resolver_with_group_size_kind support AsSum
    // #[test]
    #[allow(unused)]
    fn size_computation_delete_group() {
        let group: BTreeMap<StructTag, Bytes> = BTreeMap::from([
            (mock_tag_0(), vec![1].into()),
            (mock_tag_1(), vec![2, 2].into()),
        ]);
        let metadata = raw_metadata(100);
        let key = StateKey::raw(&[0]);

        let data = HashMap::from([(
            key.clone(),
            StateValue::new_with_metadata(bcs::to_bytes(&group).unwrap().into(), metadata.clone()),
        )]);

        let s = MockStateView::new(data);
        let resolver = as_resolver_with_group_size_kind(&s, GroupSizeKind::AsSum);
        let group_changes = BTreeMap::from([
            (mock_tag_0(), MoveStorageOp::Delete),
            (mock_tag_1(), MoveStorageOp::Delete),
        ]);
        let converter = LegacyWriteOpConverter::new(&resolver, true);
        let group_write = converter
            .convert_resource_group_v1(&key, group_changes)
            .unwrap();

        // Deletion should still contain the metadata - for storage refunds.
        assert_eq!(group_write.metadata_op().metadata(), &metadata);
        assert_eq!(group_write.metadata_op(), &WriteOp::deletion(metadata));
        assert_none!(group_write.metadata_op().bytes());
    }
}
