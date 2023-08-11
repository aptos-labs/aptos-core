// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

// This is required because a diesel macro makes clippy sad
#![allow(clippy::extra_unused_lifetimes)]
#![allow(clippy::unused_unit)]

use crate::{
    schema::nft_points,
    utils::util::{get_entry_function_from_user_request, parse_timestamp, standardize_address},
};
use aptos_protos::transaction::v1::{
    transaction::TxnData, transaction_payload::Payload, Transaction,
};
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
        transaction: &Transaction,
        nft_points_contract: Option<String>,
    ) -> Option<Self> {
        let txn_data = transaction
            .txn_data
            .as_ref()
            .expect("Txn Data doesn't exit!");
        let version = transaction.version as i64;
        let timestamp = transaction
            .timestamp
            .as_ref()
            .expect("Transaction timestamp doesn't exist!");
        let transaction_info = transaction
            .info
            .as_ref()
            .expect("Transaction info doesn't exist!");
        if let Some(contract) = nft_points_contract {
            if let TxnData::User(user_txn) = txn_data {
                let user_request = user_txn
                    .request
                    .as_ref()
                    .expect("Sends is not present in user txn");
                let payload = user_txn
                    .request
                    .as_ref()
                    .expect("Getting user request failed.")
                    .payload
                    .as_ref()
                    .expect("Getting payload failed.");
                let entry_function_id_str =
                    get_entry_function_from_user_request(user_request).unwrap_or_default();

                // If failed transaction, end
                if !transaction_info.success {
                    return None;
                }
                if entry_function_id_str == contract {
                    if let Payload::EntryFunctionPayload(inner) = payload.payload.as_ref().unwrap()
                    {
                        let owner_address = standardize_address(&inner.arguments[0]);
                        let amount = inner.arguments[2].parse().unwrap();
                        let transaction_timestamp = parse_timestamp(timestamp, version);
                        return Some(Self {
                            transaction_version: version,
                            owner_address,
                            token_name: inner.arguments[1].clone(),
                            point_type: inner.arguments[3].clone(),
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
