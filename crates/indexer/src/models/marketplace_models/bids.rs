#![allow(clippy::extra_unused_lifetimes)]
#![allow(clippy::unused_unit)]

use aptos_api_types::{TransactionPayload, UserTransaction};
use field_count::FieldCount;
use serde::{Deserialize, Serialize};

use crate::{schema::marketplace_bids, util::parse_timestamp};

#[derive(Debug, Deserialize, FieldCount, Identifiable, Insertable, Queryable, Serialize)]
#[diesel(primary_key(creator_address, collection_name))]
#[diesel(table_name = marketplace_bids)]
pub struct MarketplaceBids {
    creator_address: String,
    collection_name: String,
    token_name: String,
    property_version: i32,
    price: i64,
    maker: String,
    timestamp: chrono::NaiveDateTime,
}

impl MarketplaceBids {
    pub fn from_transaction(txn: &UserTransaction) -> Option<Self> {
        let version = txn.info.version.0;
        match txn.request.payload {
            TransactionPayload::EntryFunctionPayload(payload) => Some(Self {
                creator_address: payload.arguments[0]["creator"].to_string(),
                collection_name: payload.arguments[0]["collection_name"].to_string(),
                token_name: payload.arguments[0]["token_name"].to_string(),
                property_version: payload.arguments[0]["property_version"]
                    .as_i64()
                    .unwrap()
                    .try_into()
                    .unwrap(),
                price: payload.arguments[0]["price"].as_i64().unwrap(),
                maker: txn.request.sender.inner().to_hex_literal(),
                timestamp: parse_timestamp(txn.timestamp.0, version.try_into().unwrap()),
            }),
            _ => None,
        }
    }
}
