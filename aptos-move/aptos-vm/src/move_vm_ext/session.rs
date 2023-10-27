// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{
    access_path_cache::AccessPathCache,
    data_cache::get_resource_group_from_metadata,
    move_vm_ext::{write_op_converter::WriteOpConverter, AptosMoveResolver},
    transaction_metadata::TransactionMetadata,
};
use aptos_crypto::{hash::CryptoHash, HashValue};
use aptos_crypto_derive::{BCSCryptoHash, CryptoHasher};
use aptos_framework::natives::{
    aggregator_natives::{AggregatorChangeSet, AggregatorChangeV1, NativeAggregatorContext},
    code::{NativeCodeContext, PublishRequest},
    event::NativeEventContext,
};
use aptos_table_natives::{NativeTableContext, TableChangeSet};
use aptos_types::{
    block_metadata::BlockMetadata,
    contract_event::ContractEvent,
    on_chain_config::Features,
    state_store::state_key::StateKey,
    transaction::{SignatureCheckedTransaction, SignedTransaction},
};
use aptos_vm_types::{change_set::VMChangeSet, storage::ChangeSetConfigs};
use bytes::Bytes;
use claims::assert_none;
use move_binary_format::errors::{Location, PartialVMError, PartialVMResult, VMResult};
use move_core_types::{
    account_address::AccountAddress,
    effects::{AccountChanges, Changes, Op as MoveStorageOp},
    language_storage::{ModuleId, StructTag},
    value::MoveTypeLayout,
    vm_status::{err_msg, StatusCode, VMStatus},
};
use move_vm_runtime::{move_vm::MoveVM, session::Session};
use move_vm_types::values::Value;
use serde::{Deserialize, Serialize};
use std::{
    collections::{BTreeMap, HashMap},
    ops::{Deref, DerefMut},
    sync::Arc,
};

type AccountChangeSet = AccountChanges<Bytes, BytesWithResourceLayout>;
type ChangeSet = Changes<Bytes, BytesWithResourceLayout>;
pub type BytesWithResourceLayout = (Bytes, Option<Arc<MoveTypeLayout>>);

pub(crate) struct ResourceGroupChangeSet {
    maybe_released_cache: Option<HashMap<StateKey, BTreeMap<StructTag, Bytes>>>,
    charge_as_sum: bool,

    // Merged resource groups op.
    combined_changes: BTreeMap<StateKey, MoveStorageOp<BytesWithResourceLayout>>,

    // For V1 changes or AsSum gas charging, ops to individual resources in a group.
    granular_changes:
        BTreeMap<StateKey, BTreeMap<StructTag, MoveStorageOp<BytesWithResourceLayout>>>,
}

impl ResourceGroupChangeSet {
    fn new(
        maybe_released_cache: Option<HashMap<StateKey, BTreeMap<StructTag, Bytes>>>,
        charge_as_sum: bool,
    ) -> Self {
        Self {
            maybe_released_cache,
            charge_as_sum,
            combined_changes: BTreeMap::new(),
            granular_changes: BTreeMap::new(),
        }
    }

    fn need_granular(&self) -> bool {
        self.maybe_released_cache.is_none() || self.charge_as_sum
    }

    fn into(
        self,
    ) -> (
        Option<BTreeMap<StateKey, MoveStorageOp<BytesWithResourceLayout>>>,
        Option<BTreeMap<StateKey, BTreeMap<StructTag, MoveStorageOp<BytesWithResourceLayout>>>>,
    ) {
        let need_granular = self.need_granular();

        (
            // If released cache was set, then VM output requires combined changes.
            self.maybe_released_cache.map(|_| self.combined_changes),
            // If granular output is needed (as VM output or gas charging), provide granular changes.
            need_granular.then_some(self.granular_changes),
        )
    }

    fn populate_combined(
        &mut self,
        state_key: &StateKey,
        resources: &BTreeMap<StructTag, MoveStorageOp<BytesWithResourceLayout>>,
    ) -> VMResult<()> {
        let mut source_data = match &mut self.maybe_released_cache {
            Some(ref mut released_cache) => released_cache.remove(state_key).unwrap_or_default(),
            None => {
                // No need to populate combined group ops.
                return Ok(());
            },
        };

        let common_error = || {
            PartialVMError::new(StatusCode::UNKNOWN_INVARIANT_VIOLATION_ERROR)
                .with_message("populate v0 resource group change set error".to_string())
                .finish(Location::Undefined)
        };

        let create = source_data.is_empty();

        for (struct_tag, current_op) in resources.clone() {
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
        self.combined_changes.insert(state_key.clone(), op);
        Ok(())
    }
}

#[derive(BCSCryptoHash, CryptoHasher, Deserialize, Serialize)]
pub enum SessionId {
    Txn {
        sender: AccountAddress,
        sequence_number: u64,
        script_hash: Vec<u8>,
    },
    BlockMeta {
        // block id
        id: HashValue,
    },
    Genesis {
        // id to identify this specific genesis build
        id: HashValue,
    },
    Prologue {
        sender: AccountAddress,
        sequence_number: u64,
        script_hash: Vec<u8>,
    },
    Epilogue {
        sender: AccountAddress,
        sequence_number: u64,
        script_hash: Vec<u8>,
    },
    // For those runs that are not a transaction and the output of which won't be committed.
    Void,
}

impl SessionId {
    pub fn txn(txn: &SignedTransaction) -> Self {
        Self::txn_meta(&TransactionMetadata::new(&txn.clone()))
    }

    pub fn txn_meta(txn_data: &TransactionMetadata) -> Self {
        Self::Txn {
            sender: txn_data.sender,
            sequence_number: txn_data.sequence_number,
            script_hash: txn_data.script_hash.clone(),
        }
    }

    pub fn genesis(id: HashValue) -> Self {
        Self::Genesis { id }
    }

    pub fn block_meta(block_meta: &BlockMetadata) -> Self {
        Self::BlockMeta {
            id: block_meta.id(),
        }
    }

    pub fn prologue(txn: &SignedTransaction) -> Self {
        Self::prologue_meta(&TransactionMetadata::new(&txn.clone()))
    }

    pub fn prologue_meta(txn_data: &TransactionMetadata) -> Self {
        Self::Prologue {
            sender: txn_data.sender,
            sequence_number: txn_data.sequence_number,
            script_hash: txn_data.script_hash.clone(),
        }
    }

    pub fn epilogue(txn: &SignatureCheckedTransaction) -> Self {
        Self::epilogue_meta(&TransactionMetadata::new(&txn.clone().into_inner()))
    }

    pub fn epilogue_meta(txn_data: &TransactionMetadata) -> Self {
        Self::Epilogue {
            sender: txn_data.sender,
            sequence_number: txn_data.sequence_number,
            script_hash: txn_data.script_hash.clone(),
        }
    }

    pub fn void() -> Self {
        Self::Void
    }

    pub fn as_uuid(&self) -> HashValue {
        self.hash()
    }
}

pub struct SessionExt<'r, 'l> {
    inner: Session<'r, 'l>,
    remote: &'r dyn AptosMoveResolver,
    features: Arc<Features>,
}

impl<'r, 'l> SessionExt<'r, 'l> {
    pub fn new(
        inner: Session<'r, 'l>,
        remote: &'r dyn AptosMoveResolver,
        features: Arc<Features>,
    ) -> Self {
        Self {
            inner,
            remote,
            features,
        }
    }

    pub fn finish<C: AccessPathCache>(
        self,
        ap_cache: &mut C,
        configs: &ChangeSetConfigs,
    ) -> VMResult<VMChangeSet> {
        let move_vm = self.inner.get_move_vm();

        let resource_converter = |value: Value,
                                  layout: MoveTypeLayout,
                                  has_aggregator_lifting: bool|
         -> PartialVMResult<BytesWithResourceLayout> {
            value
                .simple_serialize(&layout)
                .map(Into::into)
                .map(|bytes| (bytes, has_aggregator_lifting.then_some(Arc::new(layout))))
                .ok_or_else(|| {
                    PartialVMError::new(StatusCode::INTERNAL_TYPE_ERROR)
                        .with_message(format!("Error when serializing resource {}.", value))
                })
        };
        let (change_set, mut extensions) = self
            .inner
            .finish_with_extensions_with_custom_effects(&resource_converter)?;

        let (change_set, resource_group_change_set) =
            Self::split_and_merge_resource_groups(move_vm, self.remote, change_set, ap_cache)?;

        let table_context: NativeTableContext = extensions.remove();
        let table_change_set = table_context
            .into_change_set()
            .map_err(|e| e.finish(Location::Undefined))?;

        let aggregator_context: NativeAggregatorContext = extensions.remove();
        let aggregator_change_set = aggregator_context.into_change_set();

        let event_context: NativeEventContext = extensions.remove();
        let events = event_context.into_events();

        let woc = WriteOpConverter::new(
            self.remote,
            self.features.is_storage_slot_metadata_enabled(),
        );

        let change_set = Self::convert_change_set(
            &woc,
            change_set,
            resource_group_change_set,
            events,
            table_change_set,
            aggregator_change_set,
            ap_cache,
            configs,
        )
        .map_err(|status| PartialVMError::new(status.status_code()).finish(Location::Undefined))?;

        Ok(change_set)
    }

    pub fn extract_publish_request(&mut self) -> Option<PublishRequest> {
        let ctx = self.get_native_extensions().get_mut::<NativeCodeContext>();
        ctx.requested_module_bundle.take()
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
    ///   * If group or data does't exist, Unreachable
    ///   * If elements remain, Modify
    ///   * Otherwise delete
    ///
    /// V1 Resource group change set behavior keeps ops for individual resources separate, not
    /// merging them into the a single op corresponding to the whole resource group (V0).
    /// TODO[agg_v2](fix) Resource groups are currently not handled correctly in terms of propagating MoveTypeLayout
    fn split_and_merge_resource_groups<C: AccessPathCache>(
        runtime: &MoveVM,
        remote: &dyn AptosMoveResolver,
        change_set: ChangeSet,
        ap_cache: &mut C,
    ) -> VMResult<(ChangeSet, ResourceGroupChangeSet)> {
        // The use of this implies that we could theoretically call unwrap with no consequences,
        // but using unwrap means the code panics if someone can come up with an attack.
        let common_error = || {
            PartialVMError::new(StatusCode::UNKNOWN_INVARIANT_VIOLATION_ERROR)
                .with_message("split_and_merge_resource_groups error".to_string())
                .finish(Location::Undefined)
        };
        let mut change_set_filtered = ChangeSet::new();

        let (maybe_released_cache, charge_as_sum) = remote.release_resource_group_cache();
        let mut resource_group_change_set =
            ResourceGroupChangeSet::new(maybe_released_cache, charge_as_sum);

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
                        get_resource_group_from_metadata(&struct_tag, md)
                    });

                if let Some(resource_group_tag) = resource_group_tag {
                    if resource_groups
                        .entry(resource_group_tag)
                        .or_insert_with(BTreeMap::new)
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
                let state_key = StateKey::access_path(
                    ap_cache.get_resource_group_path(addr, resource_group_tag),
                );

                resource_group_change_set.populate_combined(&state_key, &resources)?;
                if resource_group_change_set.need_granular() {
                    // Maintain the behavior of failing the transaction on resource
                    // group member existence invariants.
                    for (struct_tag, current_op) in resources.iter() {
                        let exists = remote
                            .resource_exists_in_group(&state_key, struct_tag)
                            .map_err(|_| common_error())?;
                        if matches!(current_op, MoveStorageOp::New(_)) == exists {
                            // Deletion and Modification require resource to exist,
                            // while creation requires the resource to not exist.
                            return Err(common_error());
                        }
                    }
                    resource_group_change_set
                        .granular_changes
                        .insert(state_key, resources);
                }
            }
        }

        Ok((change_set_filtered, resource_group_change_set))
    }

    pub(crate) fn convert_change_set<C: AccessPathCache>(
        woc: &WriteOpConverter,
        change_set: ChangeSet,
        resource_group_change_set: ResourceGroupChangeSet,
        events: Vec<(ContractEvent, Option<MoveTypeLayout>)>,
        table_change_set: TableChangeSet,
        aggregator_change_set: AggregatorChangeSet,
        ap_cache: &mut C,
        configs: &ChangeSetConfigs,
    ) -> Result<VMChangeSet, VMStatus> {
        let mut resource_write_set = BTreeMap::new();
        let mut resource_group_write_set = BTreeMap::new();
        let mut module_write_set = BTreeMap::new();
        let mut aggregator_v1_write_set = BTreeMap::new();
        let mut aggregator_v1_delta_set = BTreeMap::new();
        let mut delayed_field_change_set = BTreeMap::new();

        for (addr, account_changeset) in change_set.into_inner() {
            let (modules, resources) = account_changeset.into_inner();
            for (struct_tag, blob_and_layout_op) in resources {
                let state_key = StateKey::access_path(ap_cache.get_resource_path(addr, struct_tag));
                let op = woc.convert_resource(
                    &state_key,
                    blob_and_layout_op,
                    configs.legacy_resource_creation_as_modification(),
                )?;

                resource_write_set.insert(state_key, op);
            }

            for (name, blob_op) in modules {
                let state_key =
                    StateKey::access_path(ap_cache.get_module_path(ModuleId::new(addr, name)));
                let op = woc.convert_module(&state_key, blob_op, false)?;
                module_write_set.insert(state_key, op);
            }
        }

        // Resource group handling.
        let (maybe_combined_changes, maybe_granular_changes) = resource_group_change_set.into();
        if let Some(granular_changes) = maybe_granular_changes {
            for (state_key, resources) in granular_changes {
                let maybe_combined_op = if let Some(combined_changes) = &maybe_combined_changes {
                    // Granular ops & GroupWrite is just for gas charging, need combined op for
                    // final VM change set.
                    combined_changes
                        .get(&state_key)
                        .ok_or_else(|| {
                            VMStatus::error(
                                StatusCode::UNKNOWN_INVARIANT_VIOLATION_ERROR,
                                err_msg("Resource group combined change must be populated"),
                            )
                        })
                        .map(|op| Some(op.clone()))?
                } else {
                    None
                };

                let maybe_combined_op = match maybe_combined_op {
                    Some(op) => {
                        let converted = woc.convert_resource(&state_key, op, false)?;
                        assert_none!(
                            converted.1,
                            "Resource group combined write may not have a layout"
                        );
                        Some(converted.0)
                    },
                    None => None,
                };

                let group_write =
                    woc.convert_resource_group_v1(&state_key, resources, maybe_combined_op)?;
                resource_group_write_set.insert(state_key, group_write);
            }
        } else {
            if let Some(combined_changes) = maybe_combined_changes {
                for (state_key, blob_op) in combined_changes {
                    let op = woc.convert_resource(&state_key, blob_op, false)?;
                    resource_write_set.insert(state_key, op);
                }
            } else {
                return Err(VMStatus::error(
                    StatusCode::UNKNOWN_INVARIANT_VIOLATION_ERROR,
                    err_msg("Must have either granular or combined resource group changes"),
                ));
            }
        }

        for (handle, change) in table_change_set.changes {
            for (key, value_op) in change.entries {
                let state_key = StateKey::table_item(handle.into(), key);
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

        for (id, change) in aggregator_change_set.delayed_field_changes {
            delayed_field_change_set.insert(id, change);
        }

        VMChangeSet::new(
            resource_write_set,
            resource_group_write_set,
            module_write_set,
            aggregator_v1_write_set,
            aggregator_v1_delta_set,
            delayed_field_change_set,
            events,
            configs,
        )
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
