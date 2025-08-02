// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::move_vm_ext::{
    session::respawned_session::RespawnedSession, AptosMoveResolver, SessionId,
};
use aptos_crypto::hash::CryptoHash;
use aptos_types::{
    transaction::{
        scheduled_txn::ScheduledTransactionInfoWithKey,
        user_transaction_context::UserTransactionContext,
    },
    vm_status::VMStatus,
};
use aptos_vm_types::{change_set::VMChangeSet, storage::change_set_configs::ChangeSetConfigs};
use move_vm_runtime::ModuleStorage;

pub struct ScheduledTxnSession<'r> {
    pub session: RespawnedSession<'r>,
}

impl<'r> ScheduledTxnSession<'r> {
    pub fn spawn(
        vm: &crate::AptosVM,
        txn: &ScheduledTransactionInfoWithKey,
        chain_id: u8,
        resolver: &'r impl AptosMoveResolver,
        prologue_change_set: VMChangeSet,
    ) -> Self {
        let user_txn_ctx = UserTransactionContext::new(
            txn.sender_handle,
            vec![], // empty secondary signers
            txn.sender_handle,
            txn.max_gas_amount,
            txn.gas_unit_price,
            chain_id,
            None, // no script hash
            None, // no chain id of the source chain for this transaction
            true,
        );

        let session = RespawnedSession::spawn(
            vm,
            SessionId::scheduled_txn(txn.key.hash()),
            resolver,
            prologue_change_set,
            Some(user_txn_ctx),
        );

        Self { session }
    }

    pub fn finish(
        self,
        change_set_configs: &ChangeSetConfigs,
        module_storage: &impl ModuleStorage,
    ) -> Result<VMChangeSet, VMStatus> {
        self.session
            .finish_with_squashed_change_set(change_set_configs, module_storage, false)
    }
}
