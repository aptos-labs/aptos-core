// Copyright © Aptos Foundation

// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

// This is required because a diesel macro makes clippy sad
#![allow(clippy::extra_unused_lifetimes)]
#![allow(clippy::unused_unit)]

use super::{
    signatures::Signature,
    transactions::{Transaction, TransactionQuery},
};
use crate::{
    schema::user_transactions,
    util::{parse_timestamp, u64_to_bigdecimal},
};
use aptos_protos::transaction::v1::{
    transaction_payload::Payload as ProtoPayloadEnum, UserTransaction as ProtoUserTransaction,
};
use bigdecimal::BigDecimal;
use field_count::FieldCount;
use serde::{Deserialize, Serialize};

#[derive(
    Associations, Clone, Deserialize, Debug, FieldCount, Identifiable, Insertable, Serialize,
)]
#[diesel(belongs_to(Transaction, foreign_key = version))]
#[diesel(primary_key(version))]
#[diesel(table_name = user_transactions)]
pub struct UserTransaction {
    pub version: i64,
    pub block_height: i64,
    pub parent_signature_type: String,
    pub sender: String,
    pub sequence_number: i64,
    pub max_gas_amount: BigDecimal,
    pub expiration_timestamp_secs: chrono::NaiveDateTime,
    pub gas_unit_price: BigDecimal,
    pub timestamp: chrono::NaiveDateTime,
    pub entry_function_id_str: String,
    pub epoch: i64,
}

/// Need a separate struct for queryable because we don't want to define the inserted_at column (letting DB fill)
#[derive(Associations, Clone, Deserialize, Debug, Identifiable, Queryable, Serialize)]
#[diesel(belongs_to(TransactionQuery, foreign_key = version))]
#[diesel(primary_key(version))]
#[diesel(table_name = user_transactions)]
pub struct UserTransactionQuery {
    pub version: i64,
    pub block_height: i64,
    pub parent_signature_type: String,
    pub sender: String,
    pub sequence_number: i64,
    pub max_gas_amount: BigDecimal,
    pub expiration_timestamp_secs: chrono::NaiveDateTime,
    pub gas_unit_price: BigDecimal,
    pub timestamp: chrono::NaiveDateTime,
    pub entry_function_id_str: String,
    pub inserted_at: chrono::NaiveDateTime,
    pub epoch: i64,
}

impl UserTransaction {
    pub fn from_transaction(
        txn: &ProtoUserTransaction,
        timestamp_in_secs: i64,
        block_height: i64,
        epoch: i64,
        version: i64,
    ) -> (Self, Vec<Signature>) {
        let user_request = txn
            .request
            .as_ref()
            .expect("Sends is not present in user txn");
        (
            Self {
                version,
                block_height,
                parent_signature_type: txn
                    .request
                    .as_ref()
                    .unwrap()
                    .signature
                    .as_ref()
                    .map(Signature::get_signature_type)
                    .unwrap_or_default(),
                sender: user_request.sender.clone(),
                sequence_number: user_request.sequence_number as i64,
                max_gas_amount: u64_to_bigdecimal(user_request.max_gas_amount),
                expiration_timestamp_secs: parse_timestamp(
                    user_request
                        .expiration_timestamp_secs
                        .as_ref()
                        .expect("Expiration timestamp is not present in user txn")
                        .seconds
                        .try_into()
                        .unwrap(),
                    version,
                ),
                gas_unit_price: u64_to_bigdecimal(user_request.gas_unit_price),
                timestamp: parse_timestamp(timestamp_in_secs.try_into().unwrap(), version),
                entry_function_id_str: match &user_request.payload.as_ref().unwrap().payload {
                    Some(ProtoPayloadEnum::EntryFunctionPayload(payload)) => payload
                        .function
                        .as_ref()
                        .expect("function not exists.")
                        .name
                        .clone(),
                    _ => String::default(),
                },
                epoch,
            },
            user_request
                .signature
                .as_ref()
                .map(|s| {
                    Signature::from_user_transaction(s, &user_request.sender, version, block_height)
                        .unwrap()
                })
                .unwrap_or_default(), // empty vec if signature is None
        )
    }
}

// Prevent conflicts with other things named `Transaction`
pub type UserTransactionModel = UserTransaction;
