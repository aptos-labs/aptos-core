// Copyright © Velor Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::types::common::EventIdentity;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// A useful struct for serialization a telemetry event
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct TelemetryEvent {
    pub name: String,
    pub params: BTreeMap<String, String>,
}

/// A useful struct for serializing a telemetry dump
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct TelemetryDump {
    pub client_id: String,
    pub user_id: String,
    pub timestamp_micros: String,
    pub events: Vec<TelemetryEvent>,
}

#[derive(Debug, Serialize, Clone)]
pub(crate) struct BigQueryRow {
    #[serde(flatten)]
    pub event_identity: EventIdentity,
    pub event_name: String,
    pub event_timestamp: u64,
    pub event_params: Vec<serde_json::Value>,
}
