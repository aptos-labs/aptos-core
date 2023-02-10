// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use crate::{
    models::{
        coin_models::coin_activities::CoinActivity, token_models::token_activities::TokenActivity,
    },
    schema::petra_activities,
    util::standardize_address,
};
use aptos_api_types::Transaction as APITransaction;
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

            // Add addresses in coin_activities.
            for coin_activity in coin_activities.iter() {
                addresses.insert(coin_activity.event_account_address.clone());
                addresses.insert(coin_activity.owner_address.clone());
            }

            // Add addresses in token_activities.
            for token_activity in token_activities.iter() {
                addresses.insert(token_activity.event_account_address.clone());
                addresses.insert(token_activity.creator_address.clone());
                if let Some(from_address) = token_activity.from_address.clone() {
                    addresses.insert(from_address);
                }
                if let Some(to_address) = token_activity.to_address.clone() {
                    addresses.insert(to_address);
                }
            }

            let coin_activities_json =
                serde_json::to_value(coin_activities).unwrap_or(EMPTY_JSON_ARRAY);
            let token_activities_json =
                serde_json::to_value(token_activities).unwrap_or(EMPTY_JSON_ARRAY);

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
