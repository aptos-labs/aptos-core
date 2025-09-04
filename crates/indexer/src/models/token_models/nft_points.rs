// Copyright © Velor Foundation
// SPDX-License-Identifier: Apache-2.0

// This is required because a diesel macro makes clippy sad
#![allow(clippy::extra_unused_lifetimes)]
#![allow(clippy::unused_unit)]

use crate::{
    schema::nft_points,
    util::{parse_timestamp, standardize_address},
};
use velor_api_types::{Transaction as APITransaction, TransactionPayload};
use bigdecimal::BigDecimal;
use diesel::prelude::*;
use field_count::FieldCount;
use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize, FieldCount, Identifiable, Insertable, Serialize)]
#[diesel(primary_key(transaction_version))]
#[diesel(table_name = nft_points)]
pub struct NftPoints {
    pub transaction_version: i64,
    pub owner_address: String,
    pub token_name: String,
    pub point_type: String,
    pub amount: BigDecimal,
    pub transaction_timestamp: chrono::NaiveDateTime,
}

impl NftPoints {
    pub fn from_transaction(
        transaction: &APITransaction,
        nft_points_contract: Option<String>,
    ) -> Option<Self> {
        if let Some(contract) = nft_points_contract {
            if let APITransaction::UserTransaction(user_txn) = transaction {
                let payload = &user_txn.request.payload;
                // If failed transaction, end
                if !user_txn.info.success {
                    return None;
                }
                if let TransactionPayload::EntryFunctionPayload(entry_function_payload) = payload {
                    if entry_function_payload.function.to_string() == contract {
                        let transaction_version = user_txn.info.version.0 as i64;
                        let owner_address = standardize_address(
                            entry_function_payload.arguments[0].as_str().unwrap(),
                        );
                        let amount = entry_function_payload.arguments[2]
                            .as_str()
                            .unwrap()
                            .parse()
                            .unwrap();
                        let transaction_timestamp =
                            parse_timestamp(user_txn.timestamp.0, transaction_version);
                        return Some(Self {
                            transaction_version,
                            owner_address,
                            token_name: entry_function_payload.arguments[1]
                                .as_str()
                                .unwrap()
                                .to_string(),
                            point_type: entry_function_payload.arguments[3]
                                .as_str()
                                .unwrap()
                                .to_string(),
                            amount,
                            transaction_timestamp,
                        });
                    }
                }
            }
        }
        None
    }
}
