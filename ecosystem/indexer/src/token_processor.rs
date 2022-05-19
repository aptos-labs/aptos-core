// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use crate::{
    database::{execute_with_better_error, PgDbPool, PgPoolConnection},
    indexer::{
        errors::TransactionProcessingError, processing_result::ProcessingResult,
        transaction_processor::TransactionProcessor,
    },
    models::{
        events::EventModel,
        token::{CreationEventType, MintEventType, Token, TokenEvent},
        transactions::{TransactionModel, UserTransaction},
    },
    schema,
    schema::tokens::{
        dsl::{supply, tokens},
        token_id,
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
            TokenEvent::CreationEvent(CreationEventType { id, token_data }) => {
                let token = Token {
                    token_id: id.to_string(),
                    creator: id.creator,
                    collection: id.collection,
                    name: id.name,
                    description: token_data.description,
                    max_amount: token_data.maximum.value,
                    supply: 1, //TODO add initial balance to event
                    uri: token_data.uri,
                    minted_at: txn.timestamp,
                    inserted_at: chrono::Utc::now().naive_utc(),
                };
                execute_with_better_error(
                    conn,
                    diesel::insert_into(schema::tokens::table)
                        .values(&token)
                        .on_conflict_do_nothing(),
                )
                .expect("Error inserting row into token");
            }
            TokenEvent::MintEvent(MintEventType { amount, id }) => {
                let result: Token = tokens
                    .filter(token_id.eq(id.to_string()))
                    .first(conn)
                    .expect("Error in loading Tokens");
                let new_supply = result.supply + amount;
                let query = diesel::update(tokens.find(id.to_string())).set(supply.eq(new_supply));
                query.execute(conn).expect("Error updating row in token");
            }
            _ => (),
        }
    }
    // TODO add a dedicated mint event when minting

    // TODO for withdraw/deposite update the ownership table

    // TODO add the information to token activity table
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
