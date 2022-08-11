// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

mod transaction_converter;

use aptos_protos::{
    block_output::v1::{
        transaction_output::TxnData as TxnDataOutput, BlockOutput, TransactionOutput,
    },
    extractor::v1::{
        transaction::TransactionType, transaction::TxnData as TxnDataInput, Event, Transaction,
    },
};

use substreams::{errors::Error, store};

#[substreams::handlers::map]
/// Input will be block soon but for now we're using transaction as block
fn block_to_block_output(input_txn: Transaction) -> Result<BlockOutput, Error> {
    let mut transactions: Vec<TransactionOutput> = vec![];

    // TODO: Add a for loop here for block
    let transaction_info;
    let write_set_changes;
    match &input_txn.info {
        None => {
            return Err(Error::Unexpected(String::from(
                "Transaction info missing from Transaction",
            )));
        }
        Some(info) => {
            transaction_info = transaction_converter::get_transaction_info_output(&input_txn, info)
                .map_err(|e| Error::Unexpected(e.to_string()))?;
            write_set_changes = transaction_converter::get_write_set_changes_output(&input_txn)
                .map_err(|e| Error::Unexpected(e.to_string()))?;
        }
    }
    let mut txn_data: Option<TxnDataOutput> = None;
    let mut events_input: Option<&Vec<Event>> = None;
    match &input_txn.txn_data {
        None => {
            return Err(Error::Unexpected(String::from(
                "Transaction info cannot be missing",
            )));
        }
        Some(TxnDataInput::BlockMetadata(bmt)) => {
            txn_data = Some(TxnDataOutput::BlockMetadata(
                transaction_converter::get_block_metadata_output(bmt, &transaction_info)
                    .map_err(|e| Error::Unexpected(e.to_string()))?,
            ));
            events_input = Some(&bmt.events);
        }
        Some(TxnDataInput::User(user_txn)) => {
            txn_data = Some(TxnDataOutput::User(
                transaction_converter::get_user_transaction_output(user_txn, &transaction_info)
                    .map_err(|e| Error::Unexpected(e.to_string()))?,
            ));
            events_input = Some(&user_txn.events);
        }
        Some(TxnDataInput::Genesis(genesis_txn)) => {
            events_input = Some(&genesis_txn.events);
        }
        Some(TxnDataInput::StateCheckpoint(_)) => {}
    };
    let events = match events_input {
        None => vec![],
        Some(e) => transaction_converter::get_events_output(e, &transaction_info)
            .map_err(|e| Error::Unexpected(e.to_string()))?,
    };

    transactions.push(TransactionOutput {
        transaction_info_output: Some(transaction_info),
        events,
        write_set_changes,
        txn_data,
    });
    Ok(BlockOutput { transactions })
}

#[substreams::handlers::store]
fn store_count(transaction: Transaction, store: store::StoreAddInt64) {
    store.add(transaction.version, generate_trx_key(), 1);
    store.add(
        transaction.version,
        generate_trx_type_key(transaction.r#type()),
        1,
    );
}

fn generate_trx_key() -> String {
    String::from("total")
}

fn generate_trx_type_key(trx_type: TransactionType) -> String {
    match trx_type {
        TransactionType::Genesis => "genesis",
        TransactionType::BlockMetadata => "block_metadata",
        TransactionType::StateCheckpoint => "state_checkpoint",
        TransactionType::User => "user",
    }
    .to_string()
}
