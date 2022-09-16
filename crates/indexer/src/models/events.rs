// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0
#![allow(clippy::extra_unused_lifetimes)]
use crate::{models::transactions::Transaction, schema::events};
use aptos_api_types::Event as APIEvent;
use field_count::FieldCount;
use serde::{Deserialize, Serialize};

#[derive(
    Associations, Debug, Deserialize, FieldCount, Identifiable, Insertable, Queryable, Serialize,
)]
#[diesel(table_name = "events")]
#[belongs_to(Transaction, foreign_key = "transaction_version")]
#[primary_key(key, sequence_number)]
pub struct Event {
    pub key: String,
    pub sequence_number: i64,
    pub creation_number: i64,
    pub account_address: String,
    pub transaction_version: i64,
    pub transaction_block_height: i64,
    #[diesel(column_name = type)]
    pub type_: String,
    pub data: serde_json::Value,
    // Default time columns
    pub inserted_at: chrono::NaiveDateTime,
}

impl Event {
    pub fn from_event(
        event: &APIEvent,
        transaction_version: i64,
        transaction_block_height: i64,
    ) -> Self {
        Event {
            key: event.key.to_string(),
            account_address: event.key.0.get_creator_address().to_string(),
            creation_number: event.key.0.get_creation_number() as i64,
            transaction_version,
            transaction_block_height,
            sequence_number: *event.sequence_number.inner() as i64,
            type_: event.typ.to_string(),
            data: event.data.clone(),
            inserted_at: chrono::Utc::now().naive_utc(),
        }
    }

    pub fn from_events(
        events: &[APIEvent],
        transaction_version: i64,
        transaction_block_height: i64,
    ) -> Vec<Self> {
        events
            .iter()
            .map(|event| Self::from_event(event, transaction_version, transaction_block_height))
            .collect::<Vec<EventModel>>()
    }
}

// Prevent conflicts with other things named `Event`
pub type EventModel = Event;
