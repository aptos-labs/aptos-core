// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0
#![allow(clippy::extra_unused_lifetimes)]
use crate::{models::transactions::Transaction, schema::events};
use aptos_api_types::Event as APIEvent;
use bigdecimal::{BigDecimal, FromPrimitive};
use field_count::FieldCount;
use serde::Serialize;

#[derive(Associations, Debug, FieldCount, Identifiable, Insertable, Queryable, Serialize)]
#[diesel(table_name = "events")]
#[belongs_to(Transaction, foreign_key = "transaction_hash")]
#[primary_key(key, sequence_number)]
pub struct Event {
    pub transaction_hash: String,
    pub key: String,
    pub sequence_number: bigdecimal::BigDecimal,
    #[diesel(column_name = type)]
    pub type_: String,
    pub data: serde_json::Value,

    // Default time columns
    pub inserted_at: chrono::NaiveDateTime,
}

impl Event {
    pub fn from_event(transaction_hash: String, event: &APIEvent) -> Self {
        let event_key: aptos_types::event::EventKey = event.guid.into();
        Event {
            transaction_hash,
            key: event_key.to_string(),
            sequence_number: BigDecimal::from_u64(event.sequence_number.0)
                .expect("Should be able to convert U64 to big decimal"),
            type_: event.typ.to_string(),
            data: event.data.clone(),
            inserted_at: chrono::Utc::now().naive_utc(),
        }
    }

    pub fn from_events(transaction_hash: String, events: &[APIEvent]) -> Option<Vec<Self>> {
        if events.is_empty() {
            return None;
        }
        Some(
            events
                .iter()
                .map(|event| Self::from_event(transaction_hash.clone(), event))
                .collect::<Vec<EventModel>>(),
        )
    }
}

// Prevent conflicts with other things named `Event`
pub type EventModel = Event;
