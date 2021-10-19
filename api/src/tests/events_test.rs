// Copyright (c) The Diem Core Contributors
// SPDX-License-Identifier: Apache-2.0

use crate::tests::{assert_json, new_test_context};
use serde_json::json;

#[tokio::test]
async fn test_get_events() {
    let context = new_test_context();

    let resp = context
        .get("/events/0x00000000000000000000000000000000000000000a550c18")
        .await;

    assert_json(
        resp[0].clone(),
        json!({
          "key": "0x00000000000000000000000000000000000000000a550c18",
          "sequence_number": "0",
          "type": {
            "type": "struct",
            "address": "0x1",
            "module": "DiemAccount",
            "name": "CreateAccountEvent",
            "generic_type_params": []
          },
          "data": {
            "created": "0xa550c18",
            "role_id": "0"
          }
        }),
    );
}

#[tokio::test]
async fn test_get_events_filter_by_start_sequence_number() {
    let context = new_test_context();

    let resp = context
        .get("/events/0x00000000000000000000000000000000000000000a550c18?start=1")
        .await;

    assert_json(
        resp[0].clone(),
        json!({
          "key": "0x00000000000000000000000000000000000000000a550c18",
          "sequence_number": "1",
          "type": {
            "type": "struct",
            "address": "0x1",
            "module": "DiemAccount",
            "name": "CreateAccountEvent",
            "generic_type_params": []
          },
          "data": {
            "created": "0xb1e55ed",
            "role_id": "1"
          }
        }),
    );
}

#[tokio::test]
async fn test_get_events_filter_by_limit_page_size() {
    let context = new_test_context();

    let resp = context
        .get("/events/0x00000000000000000000000000000000000000000a550c18?start=1&limit=1")
        .await;
    assert_eq!(resp.as_array().unwrap().len(), 1);

    let resp = context
        .get("/events/0x00000000000000000000000000000000000000000a550c18?start=1&limit=2")
        .await;
    assert_eq!(resp.as_array().unwrap().len(), 2);
}
