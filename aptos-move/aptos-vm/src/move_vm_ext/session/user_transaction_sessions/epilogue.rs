// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{
    move_vm_ext::{
        session::{
            respawned_session::RespawnedSession,
            user_transaction_sessions::user::UserSessionChangeSet,
        },
        AptosMoveResolver, SessionId,
    },
    transaction_metadata::TransactionMetadata,
    AptosVM,
};
use aptos_gas_algebra::Fee;
use aptos_types::{state_store::state_key::StateKey, write_set::WriteOp};
use aptos_vm_types::{change_set::VMChangeSet, storage::change_set_configs::ChangeSetConfigs};
use derive_more::{Deref, DerefMut};
use move_core_types::vm_status::VMStatus;
use std::collections::BTreeMap;

#[derive(Deref, DerefMut)]
pub struct EpilogueSession<'r, 'l> {
    #[deref]
    #[deref_mut]
    session: RespawnedSession<'r, 'l>,
    storage_refund: Fee,
    #[allow(dead_code)]
    module_write_set: BTreeMap<StateKey, WriteOp>,
}

impl<'r, 'l> EpilogueSession<'r, 'l> {
    pub fn on_user_session_success(
        vm: &'l AptosVM,
        txn_meta: &'l TransactionMetadata,
        resolver: &'r impl AptosMoveResolver,
        user_session_change_set: UserSessionChangeSet,
        storage_refund: Fee,
    ) -> Self {
        let (change_set, module_write_set) = user_session_change_set.unpack();
        Self::new(
            vm,
            txn_meta,
            resolver,
            change_set,
            module_write_set,
            storage_refund,
        )
    }

    pub fn on_user_session_failure(
        vm: &'l AptosVM,
        txn_meta: &'l TransactionMetadata,
        resolver: &'r impl AptosMoveResolver,
        previous_session_change_set: VMChangeSet,
    ) -> Self {
        Self::new(
            vm,
            txn_meta,
            resolver,
            previous_session_change_set,
            BTreeMap::new(),
            0.into(),
        )
    }

    fn new(
        vm: &'l AptosVM,
        txn_meta: &'l TransactionMetadata,
        resolver: &'r impl AptosMoveResolver,
        previous_session_change_set: VMChangeSet,
        module_write_set: BTreeMap<StateKey, WriteOp>,
        storage_refund: Fee,
    ) -> Self {
        let session_id = SessionId::epilogue_meta(txn_meta);
        let session = RespawnedSession::spawn(
            vm,
            session_id,
            resolver,
            previous_session_change_set,
            Some(txn_meta.as_user_transaction_context()),
        );

        Self {
            session,
            storage_refund,
            module_write_set,
        }
    }

    pub fn get_storage_fee_refund(&self) -> Fee {
        self.storage_refund
    }

    pub fn finish(self, change_set_configs: &ChangeSetConfigs) -> Result<VMChangeSet, VMStatus> {
        let Self {
            session,
            storage_refund: _,
            module_write_set: _,
        } = self;
        session.finish_with_squashed_change_set(change_set_configs, true)
    }
}
