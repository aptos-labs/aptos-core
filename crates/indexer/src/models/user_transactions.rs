// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

// This is required because a diesel macro makes clippy sad
#![allow(clippy::extra_unused_lifetimes)]
#![allow(clippy::unused_unit)]

use super::{signatures::Signature, transactions::Transaction};
use crate::{
    schema::user_transactions,
    util::{parse_timestamp, parse_timestamp_secs, u64_to_bigdecimal},
};
use aptos_api_types::{TransactionPayload, UserTransaction as APIUserTransaction};
use bigdecimal::BigDecimal;
use field_count::FieldCount;
use serde::{Deserialize, Serialize};

#[derive(
    Associations,
    Clone,
    Deserialize,
    Debug,
    FieldCount,
    Identifiable,
    Insertable,
    Queryable,
    Serialize,
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
    pub inserted_at: chrono::NaiveDateTime,
}

impl UserTransaction {
    pub fn from_transaction(txn: &APIUserTransaction, block_height: i64) -> (Self, Vec<Signature>) {
        let version = txn.info.version.0 as i64;
        (
            Self {
                version,
                block_height,
                parent_signature_type: txn
                    .request
                    .signature
                    .as_ref()
                    .map(Signature::get_signature_type)
                    .unwrap_or_default(),
                sender: txn.request.sender.inner().to_hex_literal(),
                sequence_number: txn.request.sequence_number.0 as i64,
                max_gas_amount: u64_to_bigdecimal(txn.request.max_gas_amount.0),
                expiration_timestamp_secs: parse_timestamp_secs(
                    txn.request.expiration_timestamp_secs,
                    version,
                ),
                gas_unit_price: u64_to_bigdecimal(txn.request.gas_unit_price.0),
                timestamp: parse_timestamp(txn.timestamp, version),
                inserted_at: chrono::Utc::now().naive_utc(),
                entry_function_id_str: match &txn.request.payload {
                    TransactionPayload::EntryFunctionPayload(payload) => {
                        payload.function.to_string()
                    }
                    _ => String::default(),
                },
            },
            txn.request
                .signature
                .as_ref()
                .map(|s| {
                    Signature::from_user_transaction(
                        s,
                        &txn.request.sender.to_string(),
                        version,
                        block_height,
                    )
                    .unwrap()
                })
                .unwrap_or_default(), // empty vec if signature is None
        )
    }
}

// Prevent conflicts with other things named `Transaction`
pub type UserTransactionModel = UserTransaction;
