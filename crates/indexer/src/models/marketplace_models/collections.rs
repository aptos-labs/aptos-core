#![allow(clippy::extra_unused_lifetimes)]
#![allow(clippy::unused_unit)]

use aptos_api_types::{TransactionPayload, UserTransaction};
use field_count::FieldCount;
use serde::{Deserialize, Serialize};

use crate::{schema::marketplace_collections, util::parse_timestamp};

#[derive(Debug, Deserialize, FieldCount, Identifiable, Insertable, Queryable, Serialize)]
#[diesel(primary_key(creator_address, collection_name))]
#[diesel(table_name = marketplace_collections)]
pub struct MarketplaceCollection {
    creator_address: String,
    collection_name: String,
    creation_timestamp: chrono::NaiveDateTime,
}

impl MarketplaceCollection {
    pub fn from_transaction(txn: &UserTransaction) -> Option<Self> {
        let version = txn.info.version.0;
        match txn.request.payload {
            TransactionPayload::EntryFunctionPayload(payload) => Some(Self {
                creator_address: payload.arguments[0]["creator"].to_string(),
                collection_name: payload.arguments[0]["collection_name"].to_string(),
                creation_timestamp: parse_timestamp(txn.timestamp.0, version.try_into().unwrap()),
            }),
            _ => None,
        }
    }
}
