// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use aptos_logger::info;
use aptos_types::state_store::StateView;
use aptos_types::transaction::scheduled_txn::{SCHEDULED_TRANSACTIONS_MODULE_INFO, ScheduledTransactionInfoWithKey, ScheduleMapKey};
use aptos_vm::AptosVM;
use move_core_types::ability::AbilitySet;
use move_core_types::function::MoveFunctionLayout;
use move_core_types::value::{MoveStruct, MoveStructLayout, MoveTypeLayout, MoveValue};

pub struct ScheduledTxnsHandler {
}

impl ScheduledTxnsHandler {
    pub fn get_ready_txns(state_view: &impl StateView, block_timestamp_ms: u64) -> Vec<ScheduledTransactionInfoWithKey> {
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
                info!("bytes_vec size {}", bytes_vec.len());
                if let Some(first_result) = bytes_vec.first() {
                    info!("Getting ready transactions");
                    for byte_arr in bytes_vec.iter() {
                        info!("byte_arr size {}", byte_arr.len());
                    }
                    //bcs::from_bytes::<Vec<ScheduledTransactionWithKey>>(first_result).unwrap()
                    let deserial_res = bcs::from_bytes::<Vec<ScheduledTransactionInfoWithKey>>(first_result);
                    let transactions: Vec<ScheduledTransactionInfoWithKey> = deserial_res.unwrap_or_else(|e| {
                        info!("Failed to deserialize transactions: {:?}", e);
                        Vec::new()
                    });
                    transactions
                } else {
                    info!("Getting zerooooooo ready transactions");
                    assert!(false, "Getting zerooooooo ready transactions");
                    Vec::new()
                }
            }
            Err(_) => {
                info!("Failed to get ready transactions");
                //assert!(false, "Failed to get ready transactions");
                Vec::new()
            }
        }
    }
}
