// Copyright (c) Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use aptos_protos::transaction::v1::{
    transaction::TxnData, transaction_payload::Payload, EntryFunctionId, EntryFunctionPayload,
    Event, MoveModuleId, Signature, Transaction, TransactionInfo, TransactionPayload,
    UserTransaction, UserTransactionRequest, WriteSetChange,
};

#[allow(dead_code)]
pub fn create_test_transaction(
    module_address: String,
    module_name: String,
    function_name: String,
) -> Transaction {
    Transaction {
        version: 1,
        txn_data: Some(TxnData::User(UserTransaction {
            request: Some(UserTransactionRequest {
                payload: Some(TransactionPayload {
                    r#type: 1,
                    payload: Some(Payload::EntryFunctionPayload(EntryFunctionPayload {
                        function: Some(EntryFunctionId {
                            module: Some(MoveModuleId {
                                address: module_address,
                                name: module_name,
                            }),
                            name: function_name,
                        }),
                        ..Default::default()
                    })),
                }),
                signature: Some(Signature::default()),
                ..Default::default()
            }),
            events: vec![Event::default()],
        })),
        info: Some(TransactionInfo {
            changes: vec![WriteSetChange::default()],
            ..Default::default()
        }),
        ..Default::default()
    }
}
