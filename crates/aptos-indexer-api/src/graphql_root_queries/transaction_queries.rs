use crate::graphql_root_queries::AptosTransaction;
use aptos_indexer::{
    database::PgPoolConnection,
    models::{
        transactions::{Transaction},
    },
};
use async_graphql::{FieldResult};

pub fn get_transaction_by_version(
    version: u64,
    conn: &PgPoolConnection,
) -> FieldResult<AptosTransaction> {
    let (txn, maybe_user_txn, maybe_block_metadata_txn, events, writesets) =
        Transaction::get_by_version(version, conn).unwrap();
    Ok(AptosTransaction {
        transaction_info: txn,
        block_metadata_transaction: maybe_block_metadata_txn,
        user_transaction: maybe_user_txn,
        events,
        writesets,
    })
}

pub fn get_transactions_by_start_version(
    start_version: i64,
    limit: i64,
    conn: &PgPoolConnection,
) -> FieldResult<Vec<AptosTransaction>> {
    let aptos_transactions = Transaction::get_many_by_version(start_version, limit, &conn)
        .unwrap()
        .into_iter()
        .map(|res| {
            let (txn, maybe_user_txn, maybe_block_metadata_txn, events, writesets) = res;
            AptosTransaction {
                transaction_info: txn,
                block_metadata_transaction: maybe_block_metadata_txn,
                user_transaction: maybe_user_txn,
                events,
                writesets,
            }
        })
        .rev()
        .collect();
    Ok(aptos_transactions)
}

pub fn get_transactions_by_block(
    block_height: i64,
    conn: &PgPoolConnection,
) -> FieldResult<Vec<AptosTransaction>> {
    unimplemented!();
}