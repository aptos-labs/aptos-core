// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use crate::{
    access_path_cache::AccessPathCache,
    aptos_vm_impl::{convert_changeset_and_events_cached, convert_table_changeset},
    move_vm_ext::MoveResolverExt,
    transaction_metadata::TransactionMetadata,
};
use aptos_crypto::{hash::CryptoHash, HashValue};
use aptos_crypto_derive::{BCSCryptoHash, CryptoHasher};
use aptos_types::{
    block_metadata::BlockMetadata,
    transaction::{ChangeSet, SignatureCheckedTransaction},
    write_set::WriteSetMut,
};
use move_binary_format::errors::{Location, VMResult};
use move_core_types::{
    account_address::AccountAddress,
    effects::{ChangeSet as MoveChangeSet, Event as MoveEvent},
    vm_status::{StatusCode, VMStatus},
};
use move_table_extension::NativeTableContext;
use move_vm_runtime::{native_extensions::NativeContextExtensions, session::Session};
use serde::{Deserialize, Serialize};
use std::{
    convert::TryInto,
    ops::{Deref, DerefMut},
};

#[derive(BCSCryptoHash, CryptoHasher, Deserialize, Serialize)]
pub enum SessionId {
    Txn {
        sender: AccountAddress,
        sequence_number: u64,
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
        Self::Txn {
            sender: txn.sender(),
            sequence_number: txn.sequence_number(),
        }
    }

    pub fn txn_meta(txn_data: &TransactionMetadata) -> Self {
        Self::Txn {
            sender: txn_data.sender,
            sequence_number: txn_data.sequence_number,
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

    pub fn as_uuid(&self) -> u128 {
        u128::from_be_bytes(
            self.hash().as_ref()[..16]
                .try_into()
                .expect("Slice to array conversion failed."),
        )
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

    pub fn finish(self) -> VMResult<SessionOutput<'r>> {
        let (change_set, events, extensions) = self.inner.finish_with_extensions()?;
        Ok(SessionOutput {
            change_set,
            events,
            extensions,
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

pub struct SessionOutput<'r> {
    pub change_set: MoveChangeSet,
    pub events: Vec<MoveEvent>,
    pub extensions: NativeContextExtensions<'r>,
}

impl<'r> SessionOutput<'r> {
    pub fn into_change_set<C: AccessPathCache>(
        mut self,
        ap_cache: &mut C,
    ) -> Result<ChangeSet, VMStatus> {
        let mut out_write_set = WriteSetMut::new(Vec::new());
        let mut out_events = Vec::new();
        convert_changeset_and_events_cached(
            ap_cache,
            self.change_set,
            self.events,
            &mut out_write_set,
            &mut out_events,
        )?;

        let table_context: NativeTableContext = self.extensions.remove();
        let table_changeset = table_context
            .into_change_set()
            .map_err(|e| e.finish(Location::Undefined))?;
        convert_table_changeset(table_changeset, &mut out_write_set)?;

        let ws = out_write_set
            .freeze()
            .map_err(|_| VMStatus::Error(StatusCode::DATA_FORMAT_ERROR))?;
        Ok(ChangeSet::new(ws, out_events))
    }

    pub fn unpack(self) -> (MoveChangeSet, Vec<MoveEvent>, NativeContextExtensions<'r>) {
        (self.change_set, self.events, self.extensions)
    }
}
