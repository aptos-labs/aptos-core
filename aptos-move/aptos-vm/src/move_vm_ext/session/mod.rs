// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{
    data_cache::get_resource_group_member_from_metadata,
    move_vm_ext::{
        resource_state_key, write_op_converter::WriteOpConverter, AptosMoveResolver, SessionId,
    },
};
use aptos_framework::natives::{
    aggregator_natives::{AggregatorChangeSet, AggregatorChangeV1, NativeAggregatorContext},
    code::{NativeCodeContext, PublishRequest},
    cryptography::{algebra::AlgebraContext, ristretto255_point::NativeRistrettoPointContext},
    event::NativeEventContext,
    object::NativeObjectContext,
    randomness::RandomnessContext,
    state_storage::NativeStateStorageContext,
    transaction_context::NativeTransactionContext,
};
use aptos_table_natives::{NativeTableContext, TableChangeSet};
use aptos_types::{
    chain_id::ChainId, contract_event::ContractEvent, on_chain_config::Features,
    state_store::state_key::StateKey,
    transaction::user_transaction_context::UserTransactionContext,
};
use aptos_vm_types::{
    change_set::VMChangeSet, module_write_set::ModuleWriteSet,
    storage::change_set_configs::ChangeSetConfigs,
};
use bytes::Bytes;
use move_binary_format::errors::{Location, PartialVMError, PartialVMResult, VMResult};
use move_core_types::{
    effects::{AccountChanges, Changes, Op as MoveStorageOp},
    language_storage::StructTag,
    value::MoveTypeLayout,
    vm_status::StatusCode,
};
use move_vm_runtime::{
    move_vm::MoveVM, native_extensions::NativeContextExtensions, session::Session,
};
use move_vm_types::{value_serde::serialize_and_allow_delayed_values, values::Value};
use std::{
    collections::BTreeMap,
    ops::{Deref, DerefMut},
    sync::Arc,
};

pub mod respawned_session;
pub mod session_id;
pub(crate) mod user_transaction_sessions;
pub mod view_with_change_set;

pub(crate) enum ResourceGroupChangeSet {
    // Merged resource groups op.
    V0(BTreeMap<StateKey, MoveStorageOp<BytesWithResourceLayout>>),
    // Granular ops to individual resources within a group.
    V1(BTreeMap<StateKey, BTreeMap<StructTag, MoveStorageOp<BytesWithResourceLayout>>>),
}
type AccountChangeSet = AccountChanges<Bytes, BytesWithResourceLayout>;
type ChangeSet = Changes<Bytes, BytesWithResourceLayout>;
pub type BytesWithResourceLayout = (Bytes, Option<Arc<MoveTypeLayout>>);

pub struct SessionExt<'r, 'l> {
    inner: Session<'r, 'l>,
    resolver: &'r dyn AptosMoveResolver,
    is_storage_slot_metadata_enabled: bool,
}

impl<'r, 'l> SessionExt<'r, 'l> {
    pub(crate) fn new<R: AptosMoveResolver>(
        session_id: SessionId,
        move_vm: &'l MoveVM,
        chain_id: ChainId,
        features: &Features,
        maybe_user_transaction_context: Option<UserTransactionContext>,
        resolver: &'r R,
    ) -> Self {
        let mut extensions = NativeContextExtensions::default();
        let txn_hash: [u8; 32] = session_id
            .as_uuid()
            .to_vec()
            .try_into()
            .expect("HashValue should convert to [u8; 32]");

        extensions.add(NativeTableContext::new(txn_hash, resolver));
        extensions.add(NativeRistrettoPointContext::new());
        extensions.add(AlgebraContext::new());
        extensions.add(NativeAggregatorContext::new(
            txn_hash,
            resolver,
            move_vm.vm_config().delayed_field_optimization_enabled,
            resolver,
        ));
        extensions.add(RandomnessContext::new());
        extensions.add(NativeTransactionContext::new(
            txn_hash.to_vec(),
            session_id.into_script_hash(),
            chain_id.id(),
            maybe_user_transaction_context,
        ));
        extensions.add(NativeCodeContext::default());
        extensions.add(NativeStateStorageContext::new(resolver));
        extensions.add(NativeEventContext::default());
        extensions.add(NativeObjectContext::default());

        // The VM code loader has bugs around module upgrade. After a module upgrade, the internal
        // cache needs to be flushed to work around those bugs.
        move_vm.flush_loader_cache_if_invalidated();

        let is_storage_slot_metadata_enabled = features.is_storage_slot_metadata_enabled();
        Self {
            inner: move_vm.new_session_with_extensions(resolver, extensions),
            resolver,
            is_storage_slot_metadata_enabled,
        }
    }

    pub fn finish(self, configs: &ChangeSetConfigs) -> VMResult<(VMChangeSet, ModuleWriteSet)> {
        let move_vm = self.inner.get_move_vm();

        let resource_converter = |value: Value,
                                  layout: MoveTypeLayout,
                                  has_aggregator_lifting: bool|
         -> PartialVMResult<BytesWithResourceLayout> {
            let serialization_result = if has_aggregator_lifting {
                // We allow serialization of native values here because we want to
                // temporarily store native values (via encoding to ensure deterministic
                // gas charging) in block storage.
                serialize_and_allow_delayed_values(&value, &layout)?
                    .map(|bytes| (bytes.into(), Some(Arc::new(layout))))
            } else {
                // Otherwise, there should be no native values so ensure
                // serialization fails here if there are any.
                value
                    .simple_serialize(&layout)
                    .map(|bytes| (bytes.into(), None))
            };
            serialization_result.ok_or_else(|| {
                PartialVMError::new(StatusCode::INTERNAL_TYPE_ERROR)
                    .with_message(format!("Error when serializing resource {}.", value))
            })
        };

        let (change_set, mut extensions) = self
            .inner
            .finish_with_extensions_with_custom_effects(&resource_converter)?;

        let (change_set, resource_group_change_set) =
            Self::split_and_merge_resource_groups(move_vm, self.resolver, change_set)
                .map_err(|e| e.finish(Location::Undefined))?;

        let table_context: NativeTableContext = extensions.remove();
        let table_change_set = table_context
            .into_change_set()
            .map_err(|e| e.finish(Location::Undefined))?;

        let aggregator_context: NativeAggregatorContext = extensions.remove();
        let aggregator_change_set = aggregator_context
            .into_change_set()
            .map_err(|e| e.finish(Location::Undefined))?;

        let event_context: NativeEventContext = extensions.remove();
        let events = event_context.into_events();

        let woc = WriteOpConverter::new(self.resolver, self.is_storage_slot_metadata_enabled);

        let (change_set, module_write_set) = Self::convert_change_set(
            &woc,
            change_set,
            resource_group_change_set,
            events,
            table_change_set,
            aggregator_change_set,
            configs.legacy_resource_creation_as_modification(),
        )
        .map_err(|e| e.finish(Location::Undefined))?;

        Ok((change_set, module_write_set))
    }

    pub fn extract_publish_request(&mut self) -> Option<PublishRequest> {
        let ctx = self.get_native_extensions().get_mut::<NativeCodeContext>();
        ctx.requested_module_bundle.take()
    }

    fn populate_v0_resource_group_change_set(
        change_set: &mut BTreeMap<StateKey, MoveStorageOp<BytesWithResourceLayout>>,
        state_key: StateKey,
        mut source_data: BTreeMap<StructTag, Bytes>,
        resources: BTreeMap<StructTag, MoveStorageOp<BytesWithResourceLayout>>,
    ) -> PartialVMResult<()> {
        let common_error = || {
            PartialVMError::new(StatusCode::UNKNOWN_INVARIANT_VIOLATION_ERROR)
                .with_message("populate v0 resource group change set error".to_string())
        };

        let create = source_data.is_empty();

        for (struct_tag, current_op) in resources {
            match current_op {
                MoveStorageOp::Delete => {
                    source_data.remove(&struct_tag).ok_or_else(common_error)?;
                },
                MoveStorageOp::Modify((new_data, _)) => {
                    let data = source_data.get_mut(&struct_tag).ok_or_else(common_error)?;
                    *data = new_data;
                },
                MoveStorageOp::New((data, _)) => {
                    let data = source_data.insert(struct_tag, data);
                    if data.is_some() {
                        return Err(common_error());
                    }
                },
            }
        }

        let op = if source_data.is_empty() {
            MoveStorageOp::Delete
        } else if create {
            MoveStorageOp::New((
                bcs::to_bytes(&source_data)
                    .map_err(|_| common_error())?
                    .into(),
                None,
            ))
        } else {
            MoveStorageOp::Modify((
                bcs::to_bytes(&source_data)
                    .map_err(|_| common_error())?
                    .into(),
                None,
            ))
        };
        change_set.insert(state_key, op);
        Ok(())
    }

    /// * Separate the resource groups from the non-resource.
    /// * non-resource groups are kept as is
    /// * resource groups are merged into the correct format as deltas to the source data
    ///   * Remove resource group data from the deltas
    ///   * Attempt to read the existing resource group data or create a new empty container
    ///   * Apply the deltas to the resource group data
    /// The process for translating Move deltas of resource groups to resources is
    /// * Add -- insert element in container
    ///   * If entry exists, Unreachable
    ///   * If group exists, Modify
    ///   * If group doesn't exist, Add
    /// * Modify -- update element in container
    ///   * If group or data doesn't exist, Unreachable
    ///   * Otherwise modify
    /// * Delete -- remove element from container
    ///   * If group or data doesn't exist, Unreachable
    ///   * If elements remain, Modify
    ///   * Otherwise delete
    ///
    /// V1 Resource group change set behavior keeps ops for individual resources separate, not
    /// merging them into a single op corresponding to the whole resource group (V0).
    fn split_and_merge_resource_groups(
        runtime: &MoveVM,
        resolver: &dyn AptosMoveResolver,
        change_set: ChangeSet,
    ) -> PartialVMResult<(ChangeSet, ResourceGroupChangeSet)> {
        // The use of this implies that we could theoretically call unwrap with no consequences,
        // but using unwrap means the code panics if someone can come up with an attack.
        let common_error = || {
            PartialVMError::new(StatusCode::UNKNOWN_INVARIANT_VIOLATION_ERROR)
                .with_message("split_and_merge_resource_groups error".to_string())
        };
        let mut change_set_filtered = ChangeSet::new();

        let mut maybe_resource_group_cache = resolver.release_resource_group_cache().map(|v| {
            v.into_iter()
                .map(|(k, v)| (k, v.into_iter().collect::<BTreeMap<_, _>>()))
                .collect::<BTreeMap<_, _>>()
        });
        let mut resource_group_change_set = if maybe_resource_group_cache.is_some() {
            ResourceGroupChangeSet::V0(BTreeMap::new())
        } else {
            ResourceGroupChangeSet::V1(BTreeMap::new())
        };
        for (addr, account_changeset) in change_set.into_inner() {
            let mut resource_groups: BTreeMap<
                StructTag,
                BTreeMap<StructTag, MoveStorageOp<BytesWithResourceLayout>>,
            > = BTreeMap::new();
            let mut resources_filtered = BTreeMap::new();
            let (modules, resources) = account_changeset.into_inner();

            for (struct_tag, blob_op) in resources {
                let resource_group_tag = runtime
                    .with_module_metadata(&struct_tag.module_id(), |md| {
                        get_resource_group_member_from_metadata(&struct_tag, md)
                    });

                if let Some(resource_group_tag) = resource_group_tag {
                    if resource_groups
                        .entry(resource_group_tag)
                        .or_default()
                        .insert(struct_tag, blob_op)
                        .is_some()
                    {
                        return Err(common_error());
                    }
                } else {
                    resources_filtered.insert(struct_tag, blob_op);
                }
            }

            change_set_filtered
                .add_account_changeset(
                    addr,
                    AccountChangeSet::from_modules_resources(modules, resources_filtered),
                )
                .map_err(|_| common_error())?;

            for (resource_group_tag, resources) in resource_groups {
                let state_key = StateKey::resource_group(&addr, &resource_group_tag);
                match &mut resource_group_change_set {
                    ResourceGroupChangeSet::V0(v0_changes) => {
                        let source_data = maybe_resource_group_cache
                            .as_mut()
                            .expect("V0 cache must be set")
                            .remove(&state_key)
                            .unwrap_or_default();
                        Self::populate_v0_resource_group_change_set(
                            v0_changes,
                            state_key,
                            source_data,
                            resources,
                        )?;
                    },
                    ResourceGroupChangeSet::V1(v1_changes) => {
                        // Maintain the behavior of failing the transaction on resource
                        // group member existence invariants.
                        for (struct_tag, current_op) in resources.iter() {
                            let exists =
                                resolver.resource_exists_in_group(&state_key, struct_tag)?;
                            if matches!(current_op, MoveStorageOp::New(_)) == exists {
                                // Deletion and Modification require resource to exist,
                                // while creation requires the resource to not exist.
                                return Err(common_error());
                            }
                        }
                        v1_changes.insert(state_key, resources);
                    },
                }
            }
        }

        Ok((change_set_filtered, resource_group_change_set))
    }

    fn convert_change_set(
        woc: &WriteOpConverter,
        change_set: ChangeSet,
        resource_group_change_set: ResourceGroupChangeSet,
        events: Vec<(ContractEvent, Option<MoveTypeLayout>)>,
        table_change_set: TableChangeSet,
        aggregator_change_set: AggregatorChangeSet,
        legacy_resource_creation_as_modification: bool,
    ) -> PartialVMResult<(VMChangeSet, ModuleWriteSet)> {
        let mut resource_write_set = BTreeMap::new();
        let mut resource_group_write_set = BTreeMap::new();

        let mut has_modules_published_to_special_address = false;
        let mut module_write_ops = BTreeMap::new();

        let mut aggregator_v1_write_set = BTreeMap::new();
        let mut aggregator_v1_delta_set = BTreeMap::new();

        for (addr, account_changeset) in change_set.into_inner() {
            let (modules, resources) = account_changeset.into_inner();
            for (struct_tag, blob_and_layout_op) in resources {
                let state_key = resource_state_key(&addr, &struct_tag)?;
                let op = woc.convert_resource(
                    &state_key,
                    blob_and_layout_op,
                    legacy_resource_creation_as_modification,
                )?;

                resource_write_set.insert(state_key, op);
            }

            for (name, blob_op) in modules {
                if addr.is_special() {
                    has_modules_published_to_special_address = true;
                }
                let state_key = StateKey::module(&addr, &name);
                let op = woc.convert_module(&state_key, blob_op, false)?;
                module_write_ops.insert(state_key, op);
            }
        }

        match resource_group_change_set {
            ResourceGroupChangeSet::V0(v0_changes) => {
                for (state_key, blob_op) in v0_changes {
                    let op = woc.convert_resource(&state_key, blob_op, false)?;
                    resource_write_set.insert(state_key, op);
                }
            },
            ResourceGroupChangeSet::V1(v1_changes) => {
                for (state_key, resources) in v1_changes {
                    let group_write = woc.convert_resource_group_v1(&state_key, resources)?;
                    resource_group_write_set.insert(state_key, group_write);
                }
            },
        }

        for (handle, change) in table_change_set.changes {
            for (key, value_op) in change.entries {
                let state_key = StateKey::table_item(&handle.into(), &key);
                let op = woc.convert_resource(&state_key, value_op, false)?;
                resource_write_set.insert(state_key, op);
            }
        }

        for (state_key, change) in aggregator_change_set.aggregator_v1_changes {
            match change {
                AggregatorChangeV1::Write(value) => {
                    let write_op = woc.convert_aggregator_modification(&state_key, value)?;
                    aggregator_v1_write_set.insert(state_key, write_op);
                },
                AggregatorChangeV1::Merge(delta_op) => {
                    aggregator_v1_delta_set.insert(state_key, delta_op);
                },
                AggregatorChangeV1::Delete => {
                    let write_op =
                        woc.convert_aggregator(&state_key, MoveStorageOp::Delete, false)?;
                    aggregator_v1_write_set.insert(state_key, write_op);
                },
            }
        }

        // We need to remove values that are already in the writes.
        let reads_needing_exchange = aggregator_change_set
            .reads_needing_exchange
            .into_iter()
            .filter(|(state_key, _)| !resource_write_set.contains_key(state_key))
            .collect();

        let group_reads_needing_change = aggregator_change_set
            .group_reads_needing_exchange
            .into_iter()
            .filter(|(state_key, _)| !resource_group_write_set.contains_key(state_key))
            .collect();

        let change_set = VMChangeSet::new_expanded(
            resource_write_set,
            resource_group_write_set,
            aggregator_v1_write_set,
            aggregator_v1_delta_set,
            aggregator_change_set.delayed_field_changes,
            reads_needing_exchange,
            group_reads_needing_change,
            events,
        )?;
        let module_write_set =
            ModuleWriteSet::new(has_modules_published_to_special_address, module_write_ops);

        Ok((change_set, module_write_set))
    }
}

impl<'r, 'l> Deref for SessionExt<'r, 'l> {
    type Target = Session<'r, 'l>;

    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

impl<'r, 'l> DerefMut for SessionExt<'r, 'l> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.inner
    }
}
