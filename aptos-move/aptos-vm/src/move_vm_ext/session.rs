// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{
    access_path_cache::AccessPathCache, data_cache::MoveResolverWithVMMetadata,
    move_vm_ext::MoveResolverExt, transaction_metadata::TransactionMetadata,
};
use aptos_aggregator::aggregator_extension::AggregatorID;
use aptos_crypto::{hash::CryptoHash, HashValue};
use aptos_crypto_derive::{BCSCryptoHash, CryptoHasher};
use aptos_framework::natives::{
    aggregator_natives::{AggregatorChange, AggregatorChangeSet, NativeAggregatorContext},
    code::{NativeCodeContext, PublishRequest},
};
use aptos_gas::ChangeSetConfigs;
use aptos_types::{
    block_metadata::BlockMetadata,
    contract_event::ContractEvent,
    state_store::{state_key::StateKey, table::TableHandle},
    transaction::SignatureCheckedTransaction,
};
use aptos_vm_types::{
    change_set::{AptosChangeSet, DeltaChangeSet, WriteChangeSet},
    write::WriteOp,
};
use move_binary_format::errors::{Location, PartialVMError, VMResult};
use move_core_types::{
    account_address::AccountAddress,
    effects::{Event as MoveEvent, Op as MoveStorageOp},
    language_storage::{ModuleId, StructTag},
    vm_status::{StatusCode, VMStatus},
};
use move_table_extension::{NativeTableContext, TableChangeSet};
use move_vm_runtime::{move_vm::MoveVM, session::Session};
use move_vm_types::{
    effects::{AccountChangeSet, ChangeSet as MoveChangeSet},
    resolver::{Module, Resource},
};
use serde::{Deserialize, Serialize};
use std::{
    collections::BTreeMap,
    ops::{Deref, DerefMut},
};
use aptos_vm_types::effects::Op;
use aptos_vm_types::write::{AptosModule, AptosResource};

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
    // For those runs that are not a transaction and the output of which won't be committed.
    Void,
}

impl SessionId {
    pub fn txn(txn: &SignatureCheckedTransaction) -> Self {
        Self::txn_meta(&TransactionMetadata::new(&txn.clone().into_inner()))
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

    pub fn void() -> Self {
        Self::Void
    }

    pub fn as_uuid(&self) -> HashValue {
        self.hash()
    }
}

pub struct SessionExt<'r, 'l, S> {
    inner: Session<'r, 'l, S>,
    remote: MoveResolverWithVMMetadata<'r, 'l, S>,
}

impl<'r, 'l, S> SessionExt<'r, 'l, S>
where
    S: MoveResolverExt + 'r,
{
    pub fn new(inner: Session<'r, 'l, S>, move_vm: &'l MoveVM, remote: &'r S) -> Self {
        Self {
            inner,
            remote: MoveResolverWithVMMetadata::new(remote, move_vm),
        }
    }

    pub fn finish<C: AccessPathCache>(
        self,
        ap_cache: &mut C,
        configs: &ChangeSetConfigs,
    ) -> VMResult<AptosChangeSet> {
        let (change_set, events, mut extensions) = self.inner.pause_with_extensions()?;

        // TODO: Fix resource groups :(
        let (change_set, resource_group_change_set) = (change_set, MoveChangeSet::new());
        //    Self::split_and_merge_resource_groups(&self.remote, change_set)?;

        let table_context: NativeTableContext = extensions.remove();
        let table_change_set = table_context
            .into_change_set()
            .map_err(|e| e.finish(Location::Undefined))?;

        let aggregator_context: NativeAggregatorContext = extensions.remove();
        let aggregator_change_set = aggregator_context.into_change_set();

        Self::convert_change_set(
            change_set,
            resource_group_change_set,
            events,
            table_change_set,
            aggregator_change_set,
            ap_cache,
            configs,
        )
        .map_err(|status| PartialVMError::new(status.status_code()).finish(Location::Undefined))
    }

    pub fn extract_publish_request(&mut self) -> Option<PublishRequest> {
        let ctx = self.get_native_extensions().get_mut::<NativeCodeContext>();
        ctx.requested_module_bundle.take()
    }

    /// * Separate the resource groups from the non-resource groups
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
    // fn split_and_merge_resource_groups(
    //     remote: &MoveResolverWithVMMetadata<S>,
    //     change_set: MoveChangeSet,
    // ) -> VMResult<(MoveChangeSet, MoveChangeSet)> {
    //     // The use of this implies that we could theoretically call unwrap with no consequences,
    //     // but using unwrap means the code panics if someone can come up with an attack.
    //     let common_error = PartialVMError::new(StatusCode::UNKNOWN_INVARIANT_VIOLATION_ERROR)
    //         .finish(Location::Undefined);
    //     let mut change_set_filtered = MoveChangeSet::new();
    //     let mut resource_group_change_set = MoveChangeSet::new();
    //
    //     for (addr, account_changeset) in change_set.into_inner() {
    //         let mut resource_groups: BTreeMap<StructTag, AccountChangeSet> = BTreeMap::new();
    //         let (modules, resources) = account_changeset.into_inner();
    //
    //         for (struct_tag, resource_op) in resources {
    //             let resource_group_tag = remote
    //                 .get_resource_group(&struct_tag)
    //                 .map_err(|_| common_error.clone())?;
    //             if let Some(resource_group_tag) = resource_group_tag {
    //                 resource_groups
    //                     .entry(resource_group_tag)
    //                     .or_insert_with(AccountChangeSet::new)
    //                     .add_resource_op(struct_tag, resource_op)
    //                     .map_err(|_| common_error.clone())?;
    //             } else {
    //                 change_set_filtered
    //                     .add_resource_op(addr, struct_tag, resource_op)
    //                     .map_err(|_| common_error.clone())?;
    //             }
    //         }
    //
    //         for (name, blob_op) in modules {
    //             change_set_filtered
    //                 .add_module_op(ModuleId::new(addr, name), blob_op)
    //                 .map_err(|_| common_error.clone())?;
    //         }
    //
    //         for (resource_tag, resources) in resource_groups {
    //             let source_data = remote
    //                 .get_resource_group_data(&addr, &resource_tag)
    //                 .map_err(|_| common_error.clone())?;
    //             let (mut source_data, create) = if let Some(source_data) = source_data {
    //                 let source_data =
    //                     bcs::from_bytes(&source_data).map_err(|_| common_error.clone())?;
    //                 (source_data, false)
    //             } else {
    //                 (BTreeMap::new(), true)
    //             };
    //
    //             for (struct_tag, current_op) in resources.into_resources() {
    //                 match current_op {
    //                     MoveStorageOp::Delete => {
    //                         source_data
    //                             .remove(&struct_tag)
    //                             .ok_or_else(|| common_error.clone())?;
    //                     },
    //                     MoveStorageOp::Modify(new_data) => {
    //                         let data = source_data
    //                             .get_mut(&struct_tag)
    //                             .ok_or_else(|| common_error.clone())?;
    //                         *data = new_data;
    //                     },
    //                     MoveStorageOp::New(data) => {
    //                         let data = source_data.insert(struct_tag, data);
    //                         if data.is_some() {
    //                             return Err(common_error);
    //                         }
    //                     },
    //                 }
    //             }
    //
    //             let op = if source_data.is_empty() {
    //                 MoveStorageOp::Delete
    //             } else if create {
    //                 MoveStorageOp::New(
    //                     bcs::to_bytes(&source_data).map_err(|_| common_error.clone())?,
    //                 )
    //             } else {
    //                 MoveStorageOp::Modify(
    //                     bcs::to_bytes(&source_data).map_err(|_| common_error.clone())?,
    //                 )
    //             };
    //             resource_group_change_set
    //                 .add_resource_op(addr, resource_tag, op)
    //                 .map_err(|_| common_error.clone())?;
    //         }
    //     }
    //
    //     Ok((change_set_filtered, resource_group_change_set))
    // }

    fn convert_change_set<C: AccessPathCache>(
        change_set: MoveChangeSet,
        resource_group_change_set: MoveChangeSet,
        events: Vec<MoveEvent>,
        table_change_set: TableChangeSet,
        aggregator_change_set: AggregatorChangeSet,
        ap_cache: &mut C,
        configs: &ChangeSetConfigs,
    ) -> Result<AptosChangeSet, VMStatus> {
        let mut writes = WriteChangeSet::empty();
        let mut deltas = DeltaChangeSet::empty();

        for (addr, account_change_set) in change_set.into_inner() {
            let (modules, resources) = account_change_set.into_inner();
            for (struct_tag, resource_op) in resources {
                let state_key = StateKey::access_path(ap_cache.get_resource_path(addr, struct_tag));
                let op = Self::convert_resource_op(
                    resource_op,
                    configs.legacy_resource_creation_as_modification(),
                );
                writes.insert((state_key, op))
            }

            for (name, blob_op) in modules {
                let state_key =
                    StateKey::access_path(ap_cache.get_module_path(ModuleId::new(addr, name)));
                let op = Self::convert_module_op(blob_op, false);
                writes.insert((state_key, op))
            }
        }

        for (addr, account_change_set) in resource_group_change_set.into_inner() {
            let (_, resources) = account_change_set.into_inner();
            for (struct_tag, resource_op) in resources {
                let state_key =
                    StateKey::access_path(ap_cache.get_resource_group_path(addr, struct_tag));
                let op = Self::convert_resource_op(resource_op, false);
                writes.insert((state_key, op))
            }
        }

        for (handle, change) in table_change_set.changes {
            for (key, blob_op) in change.entries {
                let state_key = StateKey::table_item(handle.into(), key);
                let op = Self::convert_resource_op(blob_op.map(Resource::from_blob), false);
                writes.insert((state_key, op))
            }
        }

        for (id, change) in aggregator_change_set.changes {
            let AggregatorID { handle, key } = id;
            let key_bytes = key.0.to_vec();
            let state_key = StateKey::table_item(TableHandle::from(handle), key_bytes);

            match change {
                AggregatorChange::Write(value) => {
                    let op = WriteOp::ResourceWrite(Op::Modification(AptosResource::AggregatorValue(value)));
                    writes.insert((state_key, op));
                },
                AggregatorChange::Merge(delta_op) => deltas.insert((state_key, delta_op)),
                AggregatorChange::Delete => writes.insert((state_key, WriteOp::ResourceWrite(Op::Deletion))),
            }
        }

        let events = events
            .into_iter()
            .map(|(guid, seq_num, ty_tag, blob)| {
                let key = bcs::from_bytes(guid.as_slice())
                    .map_err(|_| VMStatus::Error(StatusCode::EVENT_KEY_MISMATCH, None))?;
                Ok(ContractEvent::new(key, seq_num, ty_tag, blob))
            })
            .collect::<Result<Vec<_>, VMStatus>>()?;

        // TODO: check change set using `configs` here.
        let change_set = AptosChangeSet::new(writes, deltas, events);
        Ok(change_set)
    }

    fn convert_resource_op(
        move_storage_op: MoveStorageOp<Resource>,
        creation_as_modification: bool,
    ) -> WriteOp {
        use MoveStorageOp::*;
        use Op::*;
        WriteOp::ResourceWrite(match move_storage_op {
            Delete => Deletion,
            New(resource) => {
                if creation_as_modification {
                    Modification(AptosResource::Standard(resource))
                } else {
                    Creation(AptosResource::Standard(resource))
                }
            },
            Modify(resource) => Modification(AptosResource::Standard(resource)),
        })
    }

    fn convert_module_op(
        move_storage_op: MoveStorageOp<Module>,
        creation_as_modification: bool,
    ) -> WriteOp {
        use MoveStorageOp::*;
        use Op::*;
        WriteOp::ModuleWrite(match move_storage_op {
            Delete => Deletion,
            New(m) => {
                if creation_as_modification {
                    Modification(AptosModule::new(m))
                } else {
                    Creation(AptosModule::new(m))
                }
            },
            Modify(m) => Modification(AptosModule::new(m)),
        })
    }
}

impl<'r, 'l, S> Deref for SessionExt<'r, 'l, S> {
    type Target = Session<'r, 'l, S>;

    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

impl<'r, 'l, S> DerefMut for SessionExt<'r, 'l, S> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.inner
    }
}
