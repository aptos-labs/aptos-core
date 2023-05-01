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
    utils::util::{
        get_entry_function_from_user_request, parse_timestamp, standardize_address,
        u64_to_bigdecimal,
    },
};
use aptos_protos::{
    transaction::testing1::v1::UserTransaction as UserTransactionPB, util::timestamp::Timestamp,
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
        txn: &UserTransactionPB,
        timestamp: &Timestamp,
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
                sender: standardize_address(&user_request.sender),
                sequence_number: user_request.sequence_number as i64,
                max_gas_amount: u64_to_bigdecimal(user_request.max_gas_amount),
                expiration_timestamp_secs: parse_timestamp(
                    user_request
                        .expiration_timestamp_secs
                        .as_ref()
                        .expect("Expiration timestamp is not present in user txn"),
                    version,
                ),
                gas_unit_price: u64_to_bigdecimal(user_request.gas_unit_price),
                timestamp: parse_timestamp(timestamp, version),
                entry_function_id_str: get_entry_function_from_user_request(user_request)
                    .unwrap_or_default(),
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
