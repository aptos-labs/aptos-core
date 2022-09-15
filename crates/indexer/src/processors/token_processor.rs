// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use crate::{
    database::{execute_with_better_error, get_chunks, PgDbPool, PgPoolConnection},
    indexer::{
        errors::TransactionProcessingError, processing_result::ProcessingResult,
        transaction_processor::TransactionProcessor,
    },
    models::{
        collection_datas::CollectionData,
        token_datas::TokenData,
        tokens::{Token, TokenOwnership},
    },
    schema,
};
use aptos_api_types::Transaction;
use async_trait::async_trait;
use diesel::result::Error;
use field_count::FieldCount;
use std::fmt::Debug;

pub const NAME: &str = "token_processor";
pub struct TokenTransactionProcessor {
    connection_pool: PgDbPool,
}

impl TokenTransactionProcessor {
    pub fn new(connection_pool: PgDbPool) -> Self {
        Self { connection_pool }
    }
}

impl Debug for TokenTransactionProcessor {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let state = &self.connection_pool.state();
        write!(
            f,
            "TokenTransactionProcessor {{ connections: {:?}  idle_connections: {:?} }}",
            state.connections, state.idle_connections
        )
    }
}

fn insert_to_db(
    conn: &PgPoolConnection,
    name: &'static str,
    start_version: i64,
    end_version: i64,
    tokens: Vec<Token>,
    token_ownerships: Vec<TokenOwnership>,
    token_datas: Vec<TokenData>,
    collection_datas: Vec<CollectionData>,
) -> Result<(), diesel::result::Error> {
    aptos_logger::trace!(
        name = name,
        start_version = start_version,
        end_version = end_version,
        "Inserting to db",
    );
    conn.build_transaction()
        .read_write()
        .run::<_, Error, _>(|| {
            insert_tokens(conn, &tokens);
            insert_token_datas(conn, &token_datas);
            insert_token_ownerships(conn, &token_ownerships);
            insert_collection_datas(conn, &collection_datas);
            Ok(())
        })
}

fn insert_tokens(conn: &PgPoolConnection, tokens_to_insert: &[Token]) {
    use schema::tokens::dsl::*;

    let chunks = get_chunks(tokens_to_insert.len(), Token::field_count());
    for (start_ind, end_ind) in chunks {
        execute_with_better_error(
            conn,
            diesel::insert_into(schema::tokens::table)
                .values(&tokens_to_insert[start_ind..end_ind])
                .on_conflict((
                    creator_address,
                    collection_name_hash,
                    name_hash,
                    property_version,
                    transaction_version,
                ))
                .do_nothing(),
        )
        .expect("Error inserting tokens into database");
    }
}

fn insert_token_ownerships(conn: &PgPoolConnection, token_ownerships_to_insert: &[TokenOwnership]) {
    use schema::token_ownerships::dsl::*;

    let chunks = get_chunks(
        token_ownerships_to_insert.len(),
        TokenOwnership::field_count(),
    );
    for (start_ind, end_ind) in chunks {
        execute_with_better_error(
            conn,
            diesel::insert_into(schema::token_ownerships::table)
                .values(&token_ownerships_to_insert[start_ind..end_ind])
                .on_conflict((
                    creator_address,
                    collection_name_hash,
                    name_hash,
                    property_version,
                    transaction_version,
                    table_handle,
                ))
                .do_nothing(),
        )
        .expect("Error inserting token_ownerships into database");
    }
}

fn insert_token_datas(conn: &PgPoolConnection, token_datas_to_insert: &[TokenData]) {
    use schema::token_datas::dsl::*;

    let chunks = get_chunks(token_datas_to_insert.len(), TokenData::field_count());
    for (start_ind, end_ind) in chunks {
        execute_with_better_error(
            conn,
            diesel::insert_into(schema::token_datas::table)
                .values(&token_datas_to_insert[start_ind..end_ind])
                .on_conflict((
                    creator_address,
                    collection_name_hash,
                    name_hash,
                    transaction_version,
                ))
                .do_nothing(),
        )
        .expect("Error inserting token_datas into database");
    }
}

fn insert_collection_datas(conn: &PgPoolConnection, collection_datas_to_insert: &[CollectionData]) {
    use schema::collection_datas::dsl::*;

    let chunks = get_chunks(
        collection_datas_to_insert.len(),
        CollectionData::field_count(),
    );
    for (start_ind, end_ind) in chunks {
        execute_with_better_error(
            conn,
            diesel::insert_into(schema::collection_datas::table)
                .values(&collection_datas_to_insert[start_ind..end_ind])
                .on_conflict((creator_address, collection_name_hash, transaction_version))
                .do_nothing(),
        )
        .expect("Error inserting collection_datas into database");
    }
}

#[async_trait]
impl TransactionProcessor for TokenTransactionProcessor {
    fn name(&self) -> &'static str {
        NAME
    }

    async fn process_transactions(
        &self,
        transactions: Vec<Transaction>,
        start_version: i64,
        end_version: i64,
    ) -> Result<ProcessingResult, TransactionProcessingError> {
        let mut all_tokens = vec![];
        let mut all_token_ownerships = vec![];
        let mut all_token_datas = vec![];
        let mut all_collection_datas = vec![];
        for txn in transactions {
            let (mut tokens, mut token_ownerships, mut token_datas, mut collection_datas) =
                Token::from_transaction(&txn);
            all_tokens.append(&mut tokens);
            all_token_ownerships.append(&mut token_ownerships);
            all_token_datas.append(&mut token_datas);
            all_collection_datas.append(&mut collection_datas);
        }

        let conn = self.get_conn();
        let tx_result = insert_to_db(
            &conn,
            self.name(),
            start_version,
            end_version,
            all_tokens,
            all_token_ownerships,
            all_token_datas,
            all_collection_datas,
        );
        match tx_result {
            Ok(_) => Ok(ProcessingResult::new(
                self.name(),
                start_version,
                end_version,
            )),
            Err(err) => Err(TransactionProcessingError::TransactionCommitError((
                anyhow::Error::from(err),
                start_version,
                end_version,
                self.name(),
            ))),
        }
    }

    fn connection_pool(&self) -> &PgDbPool {
        &self.connection_pool
    }
}
