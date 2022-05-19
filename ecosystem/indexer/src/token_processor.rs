// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use crate::{
    database::{execute_with_better_error, PgDbPool, PgPoolConnection},
    indexer::{
        errors::TransactionProcessingError, processing_result::ProcessingResult,
        transaction_processor::TransactionProcessor,
    },
    models::{
        collection::Collection,
        events::EventModel,
        ownership::Ownership,
        token::{CreateCollectionEventType, CreationEventType, MintEventType, Token, TokenEvent},
        transactions::{TransactionModel, UserTransaction},
    },
    schema,
    schema::{
        ownerships::dsl::{
            amount as ownership_amount, owner as ownership_owner, token_id as ownership_token_id,
        },
        tokens::dsl::{last_minted_at, supply, tokens},
    },
};
use aptos_rest_client::Transaction;
use async_trait::async_trait;
use diesel::{Connection, ExpressionMethods, QueryDsl, RunQueryDsl};
use futures::future::Either;
use std::{fmt::Debug, sync::Arc};

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

fn update_mint_token(conn: &PgPoolConnection, event_data: MintEventType, txn: &UserTransaction) {
    let last_mint_time = txn.timestamp;
    let query = diesel::update(tokens.find(event_data.id.to_string())).set((
        supply.eq(supply + event_data.amount),
        last_minted_at.eq(last_mint_time),
    ));
    query.execute(conn).expect("Error updating row in token");
}

fn insert_token(conn: &PgPoolConnection, event_data: CreationEventType, txn: &UserTransaction) {
    let token = Token {
        token_id: event_data.id.to_string(),
        creator: event_data.id.creator,
        collection: event_data.id.collection,
        name: event_data.id.name,
        description: event_data.token_data.description,
        max_amount: event_data.token_data.maximum.value,
        supply: 1, //TODO add initial balance to event
        uri: event_data.token_data.uri,
        minted_at: txn.timestamp,
        inserted_at: chrono::Utc::now().naive_utc(),
        last_minted_at: txn.timestamp,
    };
    execute_with_better_error(
        conn,
        diesel::insert_into(schema::tokens::table)
            .values(&token)
            .on_conflict_do_nothing(),
    )
    .expect("Error inserting row into token");
}

fn update_token_ownership(
    conn: &PgPoolConnection,
    token_id: String,
    txn: &UserTransaction,
    amount_update: i64,
) {
    let ownership = Ownership {
        token_id,
        owner: txn.sender.clone(),
        amount: amount_update,
        updated_at: txn.timestamp,
        inserted_at: chrono::Utc::now().naive_utc(),
    };
    execute_with_better_error(
        conn,
        diesel::insert_into(schema::ownerships::table)
            .values(&ownership)
            .on_conflict((ownership_token_id, ownership_owner))
            .do_update()
            .set(ownership_amount.eq(ownership_amount + ownership.amount)),
    )
    .expect("Error update token ownership");
}

fn insert_collection(
    conn: &PgPoolConnection,
    event_data: CreateCollectionEventType,
    txn: &UserTransaction,
) {
    let collection = Collection {
        creator: event_data.creator,
        name: event_data.collection_name,
        description: event_data.description,
        max_amount: event_data.maximum.value,
        uri: event_data.uri,
        created_at: txn.timestamp,
        inserted_at: chrono::Utc::now().naive_utc(),
    };
    execute_with_better_error(
        conn,
        diesel::insert_into(schema::collections::table)
            .values(&collection)
            .on_conflict_do_nothing(),
    )
    .expect("Error inserting row into collections");
}

fn process_token(conn: &PgPoolConnection, events: &[EventModel], txn: &UserTransaction) {
    // filter events to only keep token events
    let token_events = events
        .iter()
        .map(TokenEvent::from_event)
        .filter(|e| e.is_some())
        .collect::<Vec<Option<TokenEvent>>>();
    // for create token event, insert a new token to token table,
    // if token exists, increase the supply
    for event in token_events {
        match event.unwrap() {
            TokenEvent::CreationEvent(event_data) => {
                insert_token(conn, event_data, txn);
            }
            TokenEvent::MintEvent(event_data) => {
                update_mint_token(conn, event_data, txn);
            }
            TokenEvent::CollectionCreationEvent(event_data) => {
                insert_collection(conn, event_data, txn);
            }
            TokenEvent::DepositEvent(event_data) => {
                update_token_ownership(conn, event_data.id.to_string(), txn, event_data.amount);
            }
            TokenEvent::WithdrawEvent(event_data) => {
                update_token_ownership(conn, event_data.id.to_string(), txn, -event_data.amount);
            }
            _ => (),
        }
    }
}

#[async_trait]
impl TransactionProcessor for TokenTransactionProcessor {
    fn name(&self) -> &'static str {
        "token_processor"
    }

    async fn process_transaction(
        &self,
        transaction: Arc<Transaction>,
    ) -> Result<ProcessingResult, TransactionProcessingError> {
        let version = transaction.version().unwrap_or(0);

        let (_, maybe_details_model, maybe_events, _) =
            TransactionModel::from_transaction(&transaction);

        let conn = self.get_conn();

        let tx_result = conn.transaction::<(), diesel::result::Error, _>(|| {
            if let Some(Either::Left(user_txn)) = maybe_details_model {
                if let Some(events) = maybe_events {
                    process_token(&conn, &events, &user_txn);
                }
            }
            Ok(())
        });

        match tx_result {
            Ok(_) => Ok(ProcessingResult::new(self.name(), version)),
            Err(err) => Err(TransactionProcessingError::TransactionCommitError((
                anyhow::Error::from(err),
                version,
                self.name(),
            ))),
        }
    }

    fn connection_pool(&self) -> &PgDbPool {
        &self.connection_pool
    }
}
