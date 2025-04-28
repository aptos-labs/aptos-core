// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use once_cell::sync::Lazy;
use aptos_types::state_store::StateView;
use aptos_types::transaction::scheduled_txn::ScheduledTransactionWithKey;
use aptos_vm::AptosVM;
use move_core_types::account_address::AccountAddress;
use move_core_types::identifier::Identifier;
use move_core_types::language_storage::{CORE_CODE_ADDRESS, ModuleId};

pub static SCHEDULED_TRANSACTIONS_INFO: Lazy<ScheduledTxnsHandler> =
    Lazy::new(||
        ScheduledTxnsHandler {
            module_addr: CORE_CODE_ADDRESS,
            module_name: Identifier::new("scheduled_txns").unwrap(),
            get_ready_transactions_name: Identifier::new("get_ready_transactions").unwrap(),
        }
    );

pub struct ScheduledTxnsHandler {
    pub module_addr: AccountAddress,
    pub module_name: Identifier,
    pub get_ready_transactions_name: Identifier,
}

impl ScheduledTxnsHandler {
    pub fn module_id(&self) -> ModuleId {
        ModuleId::new(self.module_addr, self.module_name.clone())
    }

    pub fn get_ready_txns(state_view: &impl StateView, block_timestamp_ms: u64) -> Vec<ScheduledTransactionWithKey> {
        let res = AptosVM::execute_function(
            state_view,
            &SCHEDULED_TRANSACTIONS_INFO.module_id(),
            &SCHEDULED_TRANSACTIONS_INFO.get_ready_transactions_name,
            vec![],
            vec![
                bcs::to_bytes(&block_timestamp_ms).unwrap()
            ]
        );
        return bcs::from_bytes::<Vec<ScheduledTransactionWithKey>>(res.unwrap().as_ref()).unwrap();
    }
}
