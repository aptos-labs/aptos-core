// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use std::collections::BTreeMap;

use aptos_config::config::PeerSet;
use aptos_types::{chain_id::ChainId, PeerId};
use chrono::Utc;
use serde_json::json;

use crate::{
    jwt_auth::create_jwt_token,
    types::{
        common::NodeType,
        telemetry::{TelemetryDump, TelemetryEvent},
    },
};

use super::test_context::new_test_context;

#[tokio::test]
async fn test_custom_event() {
    let test_context = new_test_context().await;
    let chain_id = ChainId::new(28);
    let peer_id = PeerId::random();
    let node_type = NodeType::Validator;
    let epoch = 10;

    test_context
        .inner
        .peers()
        .validators()
        .write()
        .insert(chain_id, (epoch, PeerSet::default()));

    let jwt_token = create_jwt_token(
        test_context.inner.jwt_service(),
        chain_id,
        peer_id,
        node_type,
        epoch,
    )
    .unwrap();

    let body = TelemetryDump {
        client_id: "test-client".into(),
        user_id: peer_id.to_string(),
        timestamp_micros: Utc::now().timestamp_micros().to_string(),
        events: vec![TelemetryEvent {
            name: "sample-event".into(),
            params: BTreeMap::new(),
        }],
    };
    test_context
        .with_bearer_auth(jwt_token)
        .expect_status_code(500)
        .post("/custom_event", json!(body))
        .await;
}
