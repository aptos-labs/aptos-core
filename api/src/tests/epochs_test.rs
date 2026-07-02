// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use super::new_test_context;
use aptos_api_test_context::{current_function_name, TestContext};
use aptos_api_types::{mime_types, Epoch};
use aptos_cached_packages::aptos_stdlib;
use aptos_storage_interface::DbReader;
use warp::http::header::{ACCEPT, CONTENT_TYPE};
use warp::test::request;

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_get_epoch_zero() {
    let context = new_test_context(current_function_name!());

    let resp = context.get(&epoch_path(0)).await;

    assert_eq!(resp["epoch"], "0");
    assert_eq!(resp["first_version"], "0");
    assert_eq!(resp["last_version"], "0");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_get_historical_sealed_epoch_range() {
    let mut context = new_test_context(current_function_name!());
    force_end_epoch(&mut context).await;
    force_end_epoch(&mut context).await;

    let previous_epoch_last_version = epoch_ending_version(&context, 0);
    let current_epoch_last_version = epoch_ending_version(&context, 1);
    let resp = context.get(&epoch_path(1)).await;

    assert_eq!(resp["epoch"], "1");
    assert_eq!(
        resp["first_version"],
        (previous_epoch_last_version + 1).to_string(),
    );
    assert_eq!(resp["last_version"], current_epoch_last_version.to_string());
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_get_latest_sealed_epoch_range() {
    let mut context = new_test_context(current_function_name!());
    force_end_epoch(&mut context).await;

    let latest_sealed_epoch = context
        .context
        .get_latest_ledger_info_with_signatures()
        .unwrap()
        .ledger_info()
        .next_block_epoch()
        - 1;
    let previous_epoch_last_version = epoch_ending_version(&context, latest_sealed_epoch - 1);
    let latest_epoch_last_version = epoch_ending_version(&context, latest_sealed_epoch);
    let resp = context.get(&epoch_path(latest_sealed_epoch)).await;

    assert_eq!(resp["epoch"], latest_sealed_epoch.to_string());
    assert_eq!(
        resp["first_version"],
        (previous_epoch_last_version + 1).to_string(),
    );
    assert_eq!(resp["last_version"], latest_epoch_last_version.to_string());
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_get_current_open_epoch_range() {
    let mut context = new_test_context(current_function_name!());
    force_end_epoch(&mut context).await;

    let current_open_epoch = context
        .context
        .get_latest_ledger_info_with_signatures()
        .unwrap()
        .ledger_info()
        .next_block_epoch();
    let previous_epoch_last_version = epoch_ending_version(&context, current_open_epoch - 1);
    let resp = context.get(&epoch_path(current_open_epoch)).await;

    assert_eq!(resp["epoch"], current_open_epoch.to_string());
    assert_eq!(
        resp["first_version"],
        (previous_epoch_last_version + 1).to_string(),
    );
    assert!(resp["last_version"].is_null());
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_get_current_epoch_default_route() {
    let mut context = new_test_context(current_function_name!());
    force_end_epoch(&mut context).await;

    let current_open_epoch = context
        .context
        .get_latest_ledger_info_with_signatures()
        .unwrap()
        .ledger_info()
        .next_block_epoch();
    let previous_epoch_last_version = epoch_ending_version(&context, current_open_epoch - 1);
    let resp = context.get("/epochs").await;

    assert_eq!(resp["epoch"], current_open_epoch.to_string());
    assert_eq!(
        resp["first_version"],
        (previous_epoch_last_version + 1).to_string(),
    );
    assert!(resp["last_version"].is_null());
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_get_future_epoch_returns_invalid_input() {
    let mut context = new_test_context(current_function_name!());
    force_end_epoch(&mut context).await;

    let future_epoch = context
        .context
        .get_latest_ledger_info_with_signatures()
        .unwrap()
        .ledger_info()
        .next_block_epoch()
        + 1;
    let resp = context
        .expect_status_code(400)
        .get(&epoch_path(future_epoch))
        .await;

    assert_eq!(resp["error_code"], "invalid_input");
    assert!(resp["message"]
        .as_str()
        .unwrap()
        .contains("has not started"));
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_get_current_open_epoch_bcs() {
    let mut context = new_test_context(current_function_name!());
    force_end_epoch(&mut context).await;

    let current_open_epoch = context
        .context
        .get_latest_ledger_info_with_signatures()
        .unwrap()
        .ledger_info()
        .next_block_epoch();
    let previous_epoch_last_version = epoch_ending_version(&context, current_open_epoch - 1);
    let resp = context
        .reply(
            request()
                .method("GET")
                .path(&format!("/v1{}", epoch_path(current_open_epoch)))
                .header(ACCEPT, mime_types::BCS),
        )
        .await;

    assert_eq!(resp.status(), 200);
    assert_eq!(resp.headers()[CONTENT_TYPE], mime_types::BCS);

    let epoch: Epoch = bcs::from_bytes(resp.body()).unwrap();
    assert_eq!(epoch.epoch.0, current_open_epoch);
    assert_eq!(epoch.first_version.0, previous_epoch_last_version + 1);
    assert!(epoch.last_version.is_null());
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_get_current_epoch_default_route_bcs() {
    let mut context = new_test_context(current_function_name!());
    force_end_epoch(&mut context).await;

    let current_open_epoch = context
        .context
        .get_latest_ledger_info_with_signatures()
        .unwrap()
        .ledger_info()
        .next_block_epoch();
    let previous_epoch_last_version = epoch_ending_version(&context, current_open_epoch - 1);
    let resp = context
        .reply(
            request()
                .method("GET")
                .path("/v1/epochs")
                .header(ACCEPT, mime_types::BCS),
        )
        .await;

    assert_eq!(resp.status(), 200);
    assert_eq!(resp.headers()[CONTENT_TYPE], mime_types::BCS);

    let epoch: Epoch = bcs::from_bytes(resp.body()).unwrap();
    assert_eq!(epoch.epoch.0, current_open_epoch);
    assert_eq!(epoch.first_version.0, previous_epoch_last_version + 1);
    assert!(epoch.last_version.is_null());
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_get_epoch_invalid_path_input() {
    let context = new_test_context(current_function_name!());

    let resp = context
        .expect_status_code(400)
        .get("/epochs/not_a_number")
        .await;

    assert_eq!(resp["error_code"], "web_framework_error");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_openapi_spec_includes_epoch_route_and_schema() {
    let context = new_test_context(current_function_name!());
    let resp = context
        .reply(request().method("GET").path("/v1/spec.json"))
        .await;
    assert_eq!(resp.status(), 200);
    let resp: serde_json::Value = serde_json::from_slice(resp.body()).unwrap();

    assert!(resp["paths"].get("/epochs").is_some());
    assert!(resp["paths"].get("/epochs/{epoch}").is_some());
    assert!(resp["components"]["schemas"].get("Epoch").is_some());
    assert!(resp["components"]["schemas"]["Epoch"]["properties"]
        .get("last_version")
        .is_some());
    assert!(resp["components"]["schemas"]["Epoch"]["required"]
        .as_array()
        .unwrap()
        .iter()
        .any(|field| field == "last_version"));
    assert_eq!(
        resp["components"]["schemas"]["NullableU64"]["oneOf"][1]["type"],
        "null",
    );
}

fn epoch_path(epoch: u64) -> String {
    format!("/epochs/{}", epoch)
}

async fn force_end_epoch(context: &mut TestContext) {
    let root_account = context.root_account().await;
    let txn = root_account.sign_with_transaction_builder(
        context
            .transaction_factory()
            .payload(aptos_stdlib::aptos_governance_force_end_epoch_test_only()),
    );
    context.commit_block(&[txn]).await;
}

fn epoch_ending_version(context: &TestContext, epoch: u64) -> u64 {
    let proof = context
        .db
        .get_epoch_ending_ledger_infos(epoch, epoch + 1)
        .unwrap();
    assert!(!proof.more);
    assert_eq!(proof.ledger_info_with_sigs.len(), 1);
    proof.ledger_info_with_sigs[0].ledger_info().version()
}
