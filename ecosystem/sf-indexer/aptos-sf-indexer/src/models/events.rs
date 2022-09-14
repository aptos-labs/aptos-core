// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0
#![allow(clippy::extra_unused_lifetimes)]
use crate::{models::transactions::Transaction, schema::events};
use aptos_protos::block_output::v1::EventOutput;
use aptos_types::{account_address::AccountAddress, event::EventKey};
use field_count::FieldCount;
use serde::Serialize;

#[derive(Associations, Debug, FieldCount, Identifiable, Insertable, Queryable, Serialize)]
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
    pub type_str: String,
    pub data: serde_json::Value,
    // Default time columns
    pub inserted_at: chrono::NaiveDateTime,
}

impl Event {
    pub fn from_event(event: &EventOutput, block_height: u64) -> Self {
        let key = EventKey::new(
            event.key.as_ref().unwrap().creation_number,
            AccountAddress::from_hex(&event.key.as_ref().unwrap().account_address).unwrap(),
        );
        Event {
            key: key.to_string(),
            sequence_number: event.sequence_number as i64,
            creation_number: key.get_creation_number() as i64,
            account_address: key.get_creator_address().to_string(),
            transaction_version: event.version as i64,
            transaction_block_height: block_height as i64,
            type_: event.r#type.clone(),
            type_str: event.type_str.clone(),
            data: serde_json::from_str(&event.data).unwrap(),
            inserted_at: chrono::Utc::now().naive_utc(),
        }
    }

    pub fn from_events(events: &[EventOutput], block_height: u64) -> Vec<Self> {
        events
            .iter()
            .map(|event| Self::from_event(event, block_height))
            .collect()
    }
}

// Prevent conflicts with other things named `Event`
pub type EventModel = Event;
