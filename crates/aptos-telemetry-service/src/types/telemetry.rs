// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

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
