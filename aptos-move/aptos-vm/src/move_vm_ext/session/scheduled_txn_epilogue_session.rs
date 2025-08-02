// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{
    move_vm_ext::{
        session::{
            respawned_session::RespawnedSession,
            user_transaction_sessions::session_change_sets::UserSessionChangeSet,
        },
        AptosMoveResolver, SessionId,
    },
    AptosVM,
};
use aptos_crypto::hash::CryptoHash;
use aptos_types::{
    fee_statement::FeeStatement,
    transaction::{
        scheduled_txn::ScheduledTransactionInfoWithKey,
        user_transaction_context::UserTransactionContext, TransactionStatus,
    },
};
use aptos_vm_types::{
    change_set::VMChangeSet, module_and_script_storage::module_storage::AptosModuleStorage,
    module_write_set::ModuleWriteSet, output::VMOutput,
    storage::change_set_configs::ChangeSetConfigs,
};
use move_core_types::vm_status::VMStatus;

pub struct ScheduledTxnEpilogueSession<'r> {
    pub session: RespawnedSession<'r>,
    change_set_configs: &'r ChangeSetConfigs,
    fee_statement: FeeStatement,
    txn_status: TransactionStatus,
}

impl<'r> ScheduledTxnEpilogueSession<'r> {
    pub fn spawn(
        vm: &AptosVM,
        txn: &ScheduledTransactionInfoWithKey,
        chain_id: u8,
        resolver: &'r impl AptosMoveResolver,
        prev_session_change_set: VMChangeSet,
        change_set_configs: &'r ChangeSetConfigs,
        fee_statement: FeeStatement,
        txn_status: TransactionStatus,
    ) -> Self {
        let user_txn_ctx = UserTransactionContext::new(
            txn.sender_handle,
            [].to_vec(),
            txn.sender_handle,
            txn.max_gas_amount,
            txn.gas_unit_price,
            chain_id,
            None,
            None,
            true,
        );

        let session = RespawnedSession::spawn(
            vm,
            SessionId::scheduled_txn_epilogue(txn.key.hash()),
            resolver,
            prev_session_change_set,
            Some(user_txn_ctx),
        );

        Self {
            session,
            change_set_configs,
            fee_statement,
            txn_status,
        }
    }

    pub fn finish(self, module_storage: &impl AptosModuleStorage) -> Result<VMOutput, VMStatus> {
        let change_set = self.session.finish_with_squashed_change_set(
            self.change_set_configs,
            module_storage,
            true,
        )?;

        let epilogue_session_change_set = UserSessionChangeSet::new(
            change_set,
            ModuleWriteSet::empty(),
            self.change_set_configs,
        )?;

        let (change_set, module_write_set) = epilogue_session_change_set.unpack();
        Ok(VMOutput::new(
            change_set,
            module_write_set,
            self.fee_statement,
            self.txn_status,
        ))
    }
}
