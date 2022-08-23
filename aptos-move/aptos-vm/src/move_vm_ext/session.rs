// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use crate::{
    access_path_cache::AccessPathCache, move_vm_ext::MoveResolverExt,
    transaction_metadata::TransactionMetadata,
};
use aptos_aggregator::{
    aggregator_extension::AggregatorID,
    delta_change_set::{serialize, DeltaChangeSet},
    transaction::ChangeSetExt,
};
use aptos_crypto::{hash::CryptoHash, HashValue};
use aptos_crypto_derive::{BCSCryptoHash, CryptoHasher};
use aptos_types::{
    block_metadata::BlockMetadata,
    contract_event::ContractEvent,
    state_store::{state_key::StateKey, table::TableHandle},
    transaction::{ChangeSet, SignatureCheckedTransaction},
    write_set::{WriteOp, WriteSetMut},
};
use framework::natives::{
    aggregator_natives::{AggregatorChange, AggregatorChangeSet, NativeAggregatorContext},
    code::{NativeCodeContext, PublishRequest},
};
use move_deps::{
    move_binary_format::errors::{Location, VMResult},
    move_core_types::{
        account_address::AccountAddress,
        effects::{ChangeSet as MoveChangeSet, Event as MoveEvent, Op as MoveStorageOp},
        language_storage::ModuleId,
        vm_status::{StatusCode, VMStatus},
    },
    move_table_extension::{NativeTableContext, TableChange, TableChangeSet},
    move_vm_runtime::session::Session,
};
use serde::{Deserialize, Serialize};
use std::ops::{Deref, DerefMut};

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
}

impl<'r, 'l, S> SessionExt<'r, 'l, S>
where
    S: MoveResolverExt,
{
    pub fn new(inner: Session<'r, 'l, S>) -> Self {
        Self { inner }
    }

    pub fn finish(self) -> VMResult<SessionOutput> {
        let (change_set, events, mut extensions) = self.inner.finish_with_extensions()?;
        let table_context: NativeTableContext = extensions.remove();
        let table_change_set = table_context
            .into_change_set()
            .map_err(|e| e.finish(Location::Undefined))?;

        let aggregator_context: NativeAggregatorContext = extensions.remove();
        let aggregator_change_set = aggregator_context.into_change_set();

        Ok(SessionOutput {
            change_set,
            events,
            table_change_set,
            aggregator_change_set,
        })
    }

    pub fn extract_publish_request(&mut self) -> Option<PublishRequest> {
        let ctx = self.get_native_extensions().get_mut::<NativeCodeContext>();
        ctx.requested_module_bundle.take()
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

pub struct SessionOutput {
    pub change_set: MoveChangeSet,
    pub events: Vec<MoveEvent>,
    pub table_change_set: TableChangeSet,
    pub aggregator_change_set: AggregatorChangeSet,
}

// TODO: Move this into the Move repo.
fn squash_table_change_sets(
    base: &mut TableChangeSet,
    other: TableChangeSet,
) -> Result<(), VMStatus> {
    base.new_tables.extend(other.new_tables);
    for removed_table in &base.removed_tables {
        base.new_tables.remove(removed_table);
    }
    // There's chance that a table is added in `self`, and an item is added to that table in
    // `self`, and later the item is deleted in `other`, netting to a NOOP for that item,
    // but this is an tricky edge case that we don't expect to happen too much, it doesn't hurt
    // too much to just keep the deletion. It's safe as long as we do it that way consistently.
    base.removed_tables.extend(other.removed_tables.into_iter());
    for (handle, changes) in other.changes.into_iter() {
        let my_changes = base.changes.entry(handle).or_insert(TableChange {
            entries: Default::default(),
        });
        my_changes.entries.extend(changes.entries.into_iter());
    }
    Ok(())
}

impl SessionOutput {
    pub fn into_change_set<C: AccessPathCache>(
        self,
        ap_cache: &mut C,
    ) -> Result<ChangeSetExt, VMStatus> {
        use MoveStorageOp::*;
        let Self {
            change_set,
            events,
            table_change_set,
            aggregator_change_set,
        } = self;

        let mut write_set_mut = WriteSetMut::new(Vec::new());
        let mut delta_change_set = DeltaChangeSet::empty();

        for (addr, account_changeset) in change_set.into_inner() {
            let (modules, resources) = account_changeset.into_inner();
            for (struct_tag, blob_op) in resources {
                let ap = ap_cache.get_resource_path(addr, struct_tag);
                let op = match blob_op {
                    Delete => WriteOp::Deletion,
                    New(blob) | Modify(blob) => WriteOp::Modification(blob),
                };
                write_set_mut.insert((StateKey::AccessPath(ap), op))
            }

            for (name, blob_op) in modules {
                let ap = ap_cache.get_module_path(ModuleId::new(addr, name));
                let op = match blob_op {
                    Delete => WriteOp::Deletion,
                    New(blob) => WriteOp::Creation(blob),
                    Modify(blob) => WriteOp::Modification(blob),
                };

                write_set_mut.insert((StateKey::AccessPath(ap), op))
            }
        }

        for (handle, change) in table_change_set.changes {
            for (key, value_op) in change.entries {
                let state_key = StateKey::table_item(handle.into(), key);
                match value_op {
                    Delete => write_set_mut.insert((state_key, WriteOp::Deletion)),
                    New(bytes) => write_set_mut.insert((state_key, WriteOp::Creation(bytes))),
                    Modify(bytes) => {
                        write_set_mut.insert((state_key, WriteOp::Modification(bytes)))
                    }
                }
            }
        }

        for (id, change) in aggregator_change_set.changes {
            let AggregatorID { handle, key } = id;
            let key_bytes = key.0.to_vec();
            let state_key = StateKey::table_item(TableHandle::from(handle), key_bytes);

            match change {
                AggregatorChange::Write(value) => {
                    let write_op = WriteOp::Modification(serialize(&value));
                    write_set_mut.insert((state_key, write_op));
                }
                AggregatorChange::Merge(delta_op) => delta_change_set.insert((state_key, delta_op)),
                AggregatorChange::Delete => {
                    let write_op = WriteOp::Deletion;
                    write_set_mut.insert((state_key, write_op));
                }
            }
        }

        let write_set = write_set_mut
            .freeze()
            .map_err(|_| VMStatus::Error(StatusCode::DATA_FORMAT_ERROR))?;

        let events = events
            .into_iter()
            .map(|(guid, seq_num, ty_tag, blob)| {
                let key = bcs::from_bytes(guid.as_slice())
                    .map_err(|_| VMStatus::Error(StatusCode::EVENT_KEY_MISMATCH))?;
                Ok(ContractEvent::new(key, seq_num, ty_tag, blob))
            })
            .collect::<Result<Vec<_>, VMStatus>>()?;

        let change_set = ChangeSet::new(write_set, events);
        Ok(ChangeSetExt::new(delta_change_set, change_set))
    }

    pub fn squash(&mut self, other: Self) -> Result<(), VMStatus> {
        self.change_set
            .squash(other.change_set)
            .map_err(|_| VMStatus::Error(StatusCode::DATA_FORMAT_ERROR))?;
        self.events.extend(other.events.into_iter());

        // Squash the table changes.
        squash_table_change_sets(&mut self.table_change_set, other.table_change_set)?;

        // Squash aggregator changes.
        self.aggregator_change_set
            .squash(other.aggregator_change_set)?;

        Ok(())
    }
}
