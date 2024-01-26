// Copyright © Aptos Foundation

use serde::{Deserialize, Serialize};

pub mod models;
pub mod schema;
pub mod utils;
pub mod worker;

#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct EventMessage {
    pub chain_id: i64,
    pub data: String,
    pub transaction_version: i64,
    pub timestamp: String,
}

impl ToString for EventMessage {
    fn to_string(&self) -> String {
        serde_json::to_string(self).unwrap()
    }
}
