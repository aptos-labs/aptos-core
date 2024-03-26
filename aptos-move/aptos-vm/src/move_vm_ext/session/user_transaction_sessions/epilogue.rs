// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{
    move_vm_ext::{session::respawned_session::RespawnedSession, AptosMoveResolver, SessionId},
    transaction_metadata::TransactionMetadata,
    AptosVM,
};
use aptos_gas_algebra::Fee;
use aptos_vm_types::{change_set::VMChangeSet, storage::change_set_configs::ChangeSetConfigs};
use derive_more::{Deref, DerefMut};
use move_core_types::vm_status::VMStatus;

#[derive(Deref, DerefMut)]
pub struct EpilogueSession<'r, 'l> {
    #[deref]
    #[deref_mut]
    session: RespawnedSession<'r, 'l>,
    storage_refund: Fee,
}

impl<'r, 'l> EpilogueSession<'r, 'l> {
    pub fn new(
        vm: &'l AptosVM,
        txn_meta: &'l TransactionMetadata,
        resolver: &'r impl AptosMoveResolver,
        previous_session_change_set: VMChangeSet,
        storage_refund: Fee,
    ) -> Result<Self, VMStatus> {
        let session_id = SessionId::epilogue_meta(txn_meta);
        let session = RespawnedSession::spawn(
            vm,
            session_id,
            resolver,
            previous_session_change_set,
            Some(txn_meta.as_user_transaction_context()),
        )?;

        Ok(Self {
            session,
            storage_refund,
        })
    }

    pub fn get_storage_fee_refund(&self) -> Fee {
        self.storage_refund
    }

    pub fn finish(self, change_set_configs: &ChangeSetConfigs) -> Result<VMChangeSet, VMStatus> {
        let Self {
            session,
            storage_refund: _,
        } = self;
        session.finish_with_squashed_change_set(change_set_configs, true)
    }
}
