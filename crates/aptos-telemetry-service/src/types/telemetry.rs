// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use super::common::NodeType;
use crate::types::common::EventIdentity;
use aptos_types::{chain_id::ChainId, PeerId};
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

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct RemoteNodeConfigEntry {
    node_type: NodeType,
    chain_id: ChainId,
    peer_ids: Vec<PeerId>,
    node_config: serde_yaml::Value,
}

#[derive(Debug, Deserialize, Clone)]
pub struct RemoteNodeConfigsFile {
    pub version: u64,
    pub node_configs: Vec<RemoteNodeConfigEntry>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct RemoteNodeConfig {
    pub version: u64,
    pub node_config: serde_yaml::Value,
}

#[test]
fn test_telemetry_type() {
    let entry = RemoteNodeConfigEntry {
        node_type: NodeType::PublicFullNode,
        chain_id: ChainId::new(81),
        peer_ids: vec![PeerId::random()],
        node_config: r"".into(),
    };

    println!("{}", serde_json::to_string(&entry).unwrap());
}
