// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use aptos_types::state_store::StateView;
use aptos_types::transaction::scheduled_txn::{SCHEDULED_TRANSACTIONS_MODULE_INFO, ScheduledTransactionWithKey};
use aptos_vm::AptosVM;

pub struct ScheduledTxnsHandler {
}

impl ScheduledTxnsHandler {
    pub fn get_ready_txns(state_view: &impl StateView, block_timestamp_ms: u64) -> Vec<ScheduledTransactionWithKey> {
        let res = AptosVM::execute_function(
            state_view,
            &SCHEDULED_TRANSACTIONS_MODULE_INFO.module_id(),
            &SCHEDULED_TRANSACTIONS_MODULE_INFO.get_ready_transactions_name,
            vec![],
            vec![
                bcs::to_bytes(&block_timestamp_ms).unwrap()
            ]
        );
        match res {
            Ok(bytes_vec) => {
                if let Some(first_result) = bytes_vec.first() {
                    bcs::from_bytes::<Vec<ScheduledTransactionWithKey>>(first_result)
                        .unwrap_or_default()
                } else {
                    Vec::new()
                }
            }
            Err(_) => Vec::new()
        }
    }
}
