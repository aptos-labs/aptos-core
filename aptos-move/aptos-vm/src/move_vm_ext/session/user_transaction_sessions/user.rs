// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{
    move_vm_ext::{session::respawned_session::RespawnedSession, AptosMoveResolver, SessionId},
    transaction_metadata::TransactionMetadata,
    AptosVM,
};
use aptos_types::{
    contract_event::ContractEvent,
    state_store::state_key::StateKey,
    write_set::{TransactionWrite, WriteOp, WriteOpSize},
};
use aptos_vm_types::{
    change_set::{ChangeSetInterface, VMChangeSet, WriteOpInfo},
    resolver::ExecutorView,
    storage::change_set_configs::ChangeSetConfigs,
};
use derive_more::{Deref, DerefMut};
use move_binary_format::errors::PartialVMResult;
use move_core_types::{account_address::AccountAddress, ident_str, vm_status::VMStatus};
use std::collections::BTreeMap;

pub struct UserSessionChangeSet {
    change_set: VMChangeSet,
    module_write_set: BTreeMap<StateKey, WriteOp>,
}

impl UserSessionChangeSet {
    pub(crate) fn unpack(self) -> (VMChangeSet, BTreeMap<StateKey, WriteOp>) {
        (self.change_set, self.module_write_set)
    }
}

impl ChangeSetInterface for UserSessionChangeSet {
    fn num_write_ops(&self) -> usize {
        self.change_set.num_write_ops() + self.module_write_set.len()
    }

    fn write_set_size_iter(&self) -> impl Iterator<Item = (&StateKey, WriteOpSize)> {
        self.change_set.write_set_size_iter().chain(
            self.module_write_set
                .iter()
                .map(|(k, v)| (k, v.write_op_size())),
        )
    }

    fn write_op_info_iter_mut(
        &mut self,
        executor_view: &dyn ExecutorView,
    ) -> impl Iterator<Item = PartialVMResult<WriteOpInfo>> {
        self.change_set.write_op_info_iter_mut(executor_view).chain(
            self.module_write_set.iter_mut().map(|(key, op)| {
                // FIXME(George): Switch to different keys for modules!
                let tmp_addr = AccountAddress::ONE;
                let module_name = ident_str!("tmp_foo");

                let module_exists = executor_view.check_module_exists(&tmp_addr, module_name)?;
                let prev_size = if module_exists {
                    executor_view.fetch_module_size_in_bytes(&tmp_addr, module_name)? as u64
                } else {
                    0
                };

                Ok(WriteOpInfo {
                    key,
                    op_size: op.write_op_size(),
                    prev_size,
                    metadata_mut: op.get_metadata_mut(),
                })
            }),
        )
    }

    fn events_iter(&self) -> impl Iterator<Item = &ContractEvent> {
        self.change_set.events_iter()
    }
}

#[derive(Deref, DerefMut)]
pub struct UserSession<'r, 'l> {
    #[deref]
    #[deref_mut]
    pub session: RespawnedSession<'r, 'l>,
}

impl<'r, 'l> UserSession<'r, 'l> {
    pub fn new(
        vm: &'l AptosVM,
        txn_meta: &'l TransactionMetadata,
        resolver: &'r impl AptosMoveResolver,
        prologue_change_set: VMChangeSet,
    ) -> Self {
        let session_id = SessionId::txn_meta(txn_meta);

        let session = RespawnedSession::spawn(
            vm,
            session_id,
            resolver,
            prologue_change_set,
            Some(txn_meta.as_user_transaction_context()),
        );

        Self { session }
    }

    pub fn legacy_inherit_prologue_session(prologue_session: RespawnedSession<'r, 'l>) -> Self {
        Self {
            session: prologue_session,
        }
    }

    pub fn finish(
        self,
        change_set_configs: &ChangeSetConfigs,
    ) -> Result<UserSessionChangeSet, VMStatus> {
        let Self { session } = self;
        let (change_set, module_write_set) =
            session.finish_with_squashed_change_set(change_set_configs, false)?;
        let user_session_change_set = UserSessionChangeSet {
            change_set,
            module_write_set,
        };

        change_set_configs.check_change_set(&user_session_change_set)?;
        Ok(user_session_change_set)
    }
}
