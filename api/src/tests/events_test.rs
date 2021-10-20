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

#[tokio::test]
async fn test_get_events_by_invalid_key() {
    let context = new_test_context();

    let resp = context.expect_status_code(400).get("/events/invalid").await;

    assert_json(
        resp,
        json!({
            "code": 400,
            "message": "invalid parameter event key: invalid"
        }),
    );
}

#[tokio::test]
async fn test_get_events_by_account_event_handle() {
    let context = new_test_context();
    let resp = context
        .get("/accounts/0xa550c18/events/0x1::DiemAccount::AccountOperationsCapability/creation_events")
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
async fn test_get_events_by_invalid_account_event_handle_struct_address() {
    let context = new_test_context();
    let resp = context
        .expect_status_code(404)
        .get("/accounts/0xa550c18/events/0x9::DiemAccount::AccountOperationsCapability/creation_events")
        .await;

    assert_json(
        resp,
        json!({
          "code": 404,
          "message": "resource not found by address(0xa550c18), struct tag(0x9::DiemAccount::AccountOperationsCapability) and ledger version(0)",
          "diem_ledger_version": "0"
        }),
    );
}

#[tokio::test]
async fn test_get_events_by_invalid_account_event_handle_struct_module() {
    let context = new_test_context();
    let resp = context
        .expect_status_code(404)
        .get(
            "/accounts/0xa550c18/events/0x1::NotFound::AccountOperationsCapability/creation_events",
        )
        .await;

    assert_json(
        resp,
        json!({
          "code": 404,
          "message": "resource not found by address(0xa550c18), struct tag(0x1::NotFound::AccountOperationsCapability) and ledger version(0)",
          "diem_ledger_version": "0"
        }),
    );
}

#[tokio::test]
async fn test_get_events_by_invalid_account_event_handle_struct_name() {
    let context = new_test_context();
    let resp = context
        .expect_status_code(404)
        .get("/accounts/0xa550c18/events/0x1::DiemAccount::NotFound/creation_events")
        .await;

    assert_json(
        resp,
        json!({
          "code": 404,
          "message": "resource not found by address(0xa550c18), struct tag(0x1::DiemAccount::NotFound) and ledger version(0)",
          "diem_ledger_version": "0"
        }),
    );
}

#[tokio::test]
async fn test_get_events_by_invalid_account_event_handle_field_name() {
    let context = new_test_context();
    let resp = context
        .expect_status_code(404)
        .get("/accounts/0xa550c18/events/0x1::DiemAccount::AccountOperationsCapability/not_found")
        .await;

    assert_json(
        resp,
        json!({
          "code": 404,
          "message": "resource not found by address(0xa550c18), struct tag(0x1::DiemAccount::AccountOperationsCapability), field name(not_found) and ledger version(0)",
          "diem_ledger_version": "0"
        }),
    );
}

#[tokio::test]
async fn test_get_events_by_invalid_account_event_handle_field_type() {
    let context = new_test_context();
    let resp = context
        .expect_status_code(400)
        .get("/accounts/0xa550c18/events/0x1::DiemAccount::AccountOperationsCapability/limits_cap")
        .await;

    assert_json(
        resp,
        json!({
          "code": 400,
          "message": "field(limits_cap) type is not EventHandle struct, deserialize error: unexpected end of input"
        }),
    );
}
