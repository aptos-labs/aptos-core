// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use aptos_crypto::HashValue;
use aptos_logger::error;
use aptos_types::{
    state_store::StateView,
    transaction::scheduled_txn::{
        ScheduledTransactionInfoWithKey, SCHEDULED_TRANSACTIONS_MODULE_INFO,
    },
};
use aptos_vm::{move_vm_ext::SessionId, AptosVM};

pub struct ScheduledTxnsHandler {}

impl ScheduledTxnsHandler {
    pub fn handle_ready_txns_result(
        res: Result<Vec<Vec<u8>>, move_core_types::vm_status::VMStatus>,
    ) -> Vec<ScheduledTransactionInfoWithKey> {
        match res {
            Ok(bytes_vec) => {
                if let Some(first_result) = bytes_vec.first() {
                    let deserial_res =
                        bcs::from_bytes::<Vec<ScheduledTransactionInfoWithKey>>(first_result);
                    let transactions: Vec<ScheduledTransactionInfoWithKey> = deserial_res
                        .unwrap_or_else(|e| {
                            error!(
                                "[Scheduled txns] failed to deserialize transactions: {:?}",
                                e
                            );
                            Vec::new()
                        });
                    transactions
                } else {
                    error!(
                        "[Scheduled txns] wrong result format from get_ready_transactions(): {:?}",
                        bytes_vec
                    );
                    Vec::new()
                }
            },
            Err(err) => {
                error!(
                    "[Scheduled txns] failed to execute get_ready_transactions(): {:?}",
                    err
                );
                Vec::new()
            },
        }
    }

    pub fn get_ready_txns(
        state_view: &impl StateView,
        block_timestamp_ms: u64,
        block_id: HashValue,
    ) -> Vec<ScheduledTransactionInfoWithKey> {
        let res = AptosVM::execute_system_function_no_gas_meter(
            state_view,
            &SCHEDULED_TRANSACTIONS_MODULE_INFO.module_id(),
            &SCHEDULED_TRANSACTIONS_MODULE_INFO.get_ready_transactions_name,
            vec![],
            vec![bcs::to_bytes(&block_timestamp_ms).expect("Failed to serialize block timestamp")],
            SessionId::scheduled_txn_get_ready_txns(block_id),
        );
        Self::handle_ready_txns_result(res)
    }
}
