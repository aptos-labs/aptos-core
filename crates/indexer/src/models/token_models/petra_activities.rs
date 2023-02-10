// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use crate::models::coin_models::coin_activities::CoinActivity;
use crate::models::token_models::token_activities::TokenActivity;
use crate::schema::petra_activities;
use crate::util::standardize_address;
use aptos_api_types::{Transaction as APITransaction, WriteSetChange as APIWriteSetChange};
use field_count::FieldCount;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;

#[derive(Debug, Deserialize, FieldCount, Identifiable, Insertable, Serialize)]
#[diesel(primary_key(account_address, transaction_version))]
#[diesel(table_name = petra_activities)]
pub struct PetraActivity {
    pub transaction_version: i64,
    pub account_address: String,
    pub coin_activities: serde_json::Value,
    pub token_activities: serde_json::Value,
}

const EMPTY_JSON_ARRAY: serde_json::Value = serde_json::Value::Array(vec![]);

impl PetraActivity {
    pub fn from_transaction(
        transaction: &APITransaction,
        coin_activities: Vec<CoinActivity>,
        token_activities: Vec<TokenActivity>,
    ) -> Vec<Self> {
        let mut petra_activities = vec![];

        if let APITransaction::UserTransaction(user_txn) = transaction {
            let version = user_txn.info.version.0 as i64;
            let mut addresses = HashSet::new();
            let coin_activities_json =
                serde_json::to_value(coin_activities).unwrap_or(EMPTY_JSON_ARRAY);
            let token_activities_json =
                serde_json::to_value(token_activities).unwrap_or(EMPTY_JSON_ARRAY);

            // Add the transaction sender.
            addresses.insert(user_txn.request.sender.to_string());

            // Add any other accounts referenced in the writeset.
            for wsc in user_txn.info.changes.iter() {
                match wsc {
                    APIWriteSetChange::WriteResource(write_resource) => {
                        addresses.insert(write_resource.address.to_string());
                    },
                    APIWriteSetChange::WriteTableItem(write_table_item) => {
                        if let Some(decoded_table_data) = &write_table_item.data {
                            if decoded_table_data.key_type == "address" {
                                // The string is surrounded by quotes ("0x1").
                                let address_json = decoded_table_data.key.to_string();
                                let address = &address_json[1..address_json.len() - 1];
                                addresses.insert(address.into());
                            }
                        }
                    },
                    _ => {},
                }
            }

            for address in addresses.iter() {
                petra_activities.push(Self {
                    transaction_version: version,
                    account_address: standardize_address(address),
                    coin_activities: coin_activities_json.clone(),
                    token_activities: token_activities_json.clone(),
                });
            }
        }

        petra_activities
    }
}
