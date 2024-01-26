// Copyright © Aptos Foundation

use serde::{Deserialize, Serialize};

#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct PubSubEventMessage {
    pub chain_id: i64,
    pub data: Vec<String>,
    pub transaction_version: i64,
    pub timestamp: String,
}

impl ToString for PubSubEventMessage {
    fn to_string(&self) -> String {
        serde_json::to_string(self).unwrap()
    }
}

#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct StreamEventMessage {
    pub chain_id: i64,
    pub data: String,
    pub transaction_version: i64,
    pub timestamp: String,
}

impl ToString for StreamEventMessage {
    fn to_string(&self) -> String {
        serde_json::to_string(self).unwrap()
    }
}

impl StreamEventMessage {
    pub fn list_from_pubsub(pubsub_event_message: PubSubEventMessage) -> Vec<Self> {
        pubsub_event_message
            .data
            .iter()
            .map(|data| StreamEventMessage {
                chain_id: pubsub_event_message.chain_id,
                data: data.to_string(),
                transaction_version: pubsub_event_message.transaction_version,
                timestamp: pubsub_event_message.timestamp.to_string(),
            })
            .collect()
    }
}
