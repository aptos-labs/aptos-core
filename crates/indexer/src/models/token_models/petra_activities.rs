// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

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
}

impl PetraActivity {
    pub fn from_transaction(transaction: &APITransaction) -> Vec<Self> {
        let mut petra_activities = vec![];

        if let APITransaction::UserTransaction(user_txn) = transaction {
            let version = user_txn.info.version.0 as i64;
            let mut addresses = HashSet::new();

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
                });
            }
        }

        petra_activities
    }
}
