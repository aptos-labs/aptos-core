// Copyright © Aptos Foundation
// Parts of the project are originally copyright © Meta Platforms, Inc.
// SPDX-License-Identifier: Apache-2.0

use super::{new_test_context, new_test_context_with_orderless_flags};
use aptos_api_test_context::{current_function_name, find_value, TestContext};
use aptos_api_types::{MoveModuleBytecode, MoveResource, MoveStructTag, StateKeyWrapper};
use aptos_cached_packages::aptos_stdlib;
use aptos_sdk::types::APTOS_COIN_TYPE_STR;
use aptos_types::{
    account_config::{primary_apt_store, ObjectCoreResource},
    transaction::{EntryFunction, TransactionPayload},
    AptosCoinType, CoinType,
};
use move_core_types::{
    account_address::AccountAddress,
    identifier::Identifier,
    language_storage::{ModuleId, TypeTag},
    move_resource::MoveStructType,
};
use rstest::rstest;
use serde_json::json;
use std::{path::PathBuf, str::FromStr};

/* TODO: reactivate once cause of failure for `"8"` vs `8` in the JSON output is known.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_get_account_resources_returns_empty_array_for_account_has_no_resources() {
    let mut context = new_test_context(current_function_name!());
    let address = "0x1";

    let resp = context.get(&account_resources(address)).await;
    context.check_golden_output(resp);
}
 */

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_get_account_resources_by_address_0x0() {
    let mut context = new_test_context(current_function_name!());
    let address = "0x0";

    let resp = context
        .expect_status_code(200)
        .get(&account_resources(address))
        .await;
    context.check_golden_output(resp);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_get_account_resources_by_valid_account_address() {
    let context = new_test_context(current_function_name!());
    let addresses = vec!["0x1", "0x00000000000000000000000000000001"];
    let mut res = vec![];
    for address in &addresses {
        let resp = context.get(&account_resources(address)).await;
        res.push(resp);
    }

    let shard_context = new_test_context(current_function_name!());
    let mut shard_res = vec![];
    for address in &addresses {
        let resp = shard_context.get(&account_resources(address)).await;
        shard_res.push(resp);
    }

    assert_eq!(res, shard_res);
}

// Unstable due to framework changes
#[ignore]
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_account_resources_response() {
    let mut context = new_test_context(current_function_name!());
    let address = "0x1";

    let resp = context.get(&account_resources(address)).await;
    context.check_golden_output(resp);
}

// Unstable due to framework changes
#[ignore]
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_account_modules() {
    let mut context = new_test_context(current_function_name!());
    let address = "0x1";

    let resp = context.get(&account_modules(address)).await;
    context.check_golden_output(resp);
}

// Unstable due to framework changes
#[ignore]
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_get_module_with_entry_functions() {
    let mut context = new_test_context(current_function_name!());
    let address = "0x1";

    let resp = context.get(&account_modules(address)).await;
    context.check_golden_output(resp);
}

// Unstable due to framework changes
#[ignore]
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_get_module_aptos_config() {
    let mut context = new_test_context(current_function_name!());
    let address = "0x1";

    let resp = context.get(&account_modules(address)).await;
    context.check_golden_output(resp);
}

// Unstable due to framework changes
#[ignore]
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_account_modules_structs() {
    let mut context = new_test_context(current_function_name!());
    let address = "0x1";

    let resp = context.get(&account_modules(address)).await;
    context.check_golden_output(resp);
}

async fn test_account_resources_by_ledger_version_with_context(mut context: TestContext) {
    let initial_ledger_version = u64::from(context.get_latest_ledger_info().ledger_version);

    let initial_resources = context
        .get(&account_resources(
            &context.root_account().await.address().to_hex_literal(),
        ))
        .await;
    let root_account = find_value(&initial_resources, |f| f["type"] == "0x1::account::Account");
    let initial_sequence_number = root_account["data"]["sequence_number"]
        .as_str()
        .unwrap()
        .parse::<u64>()
        .unwrap();

    let account = context.gen_account();
    let txn = context.create_user_account(&account).await;
    context.commit_block(&vec![txn.clone()]).await;

    if let Some(indexer_reader) = context.context.indexer_reader.as_ref() {
        // Waiting for the above transaction, block metadata and state checkpoint to get indexed.
        indexer_reader
            .wait_for_internal_indexer(initial_ledger_version + 3)
            .unwrap();
    }

    let ledger_version_1_resources = context
        .get(&account_resources(
            &context.root_account().await.address().to_hex_literal(),
        ))
        .await;
    let root_account = find_value(&ledger_version_1_resources, |f| {
        f["type"] == "0x1::account::Account"
    });
    if context.use_orderless_transactions {
        // Orderless transactions don't update sequence number
        assert_eq!(
            root_account["data"]["sequence_number"]
                .as_str()
                .unwrap()
                .parse::<u64>()
                .unwrap(),
            initial_sequence_number
        );
    } else {
        assert_eq!(
            root_account["data"]["sequence_number"]
                .as_str()
                .unwrap()
                .parse::<u64>()
                .unwrap(),
            initial_sequence_number + 1
        );
    }

    let ledger_version_0_resources = context
        .get(&account_resources_with_ledger_version(
            &context.root_account().await.address().to_hex_literal(),
            initial_ledger_version as i128,
        ))
        .await;
    let root_account = find_value(&ledger_version_0_resources, |f| {
        f["type"] == "0x1::account::Account"
    });
    assert_eq!(
        root_account["data"]["sequence_number"]
            .as_str()
            .unwrap()
            .parse::<u64>()
            .unwrap(),
        initial_sequence_number
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[rstest(
    use_txn_payload_v2_format,
    use_orderless_transactions,
    case(false, false),
    case(true, false),
    case(true, true)
)]
async fn test_get_account_resources_by_ledger_version(
    use_txn_payload_v2_format: bool,
    use_orderless_transactions: bool,
) {
    let context = new_test_context_with_orderless_flags(
        current_function_name!(),
        use_txn_payload_v2_format,
        use_orderless_transactions,
    );
    test_account_resources_by_ledger_version_with_context(context).await;
}
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[rstest(
    use_txn_payload_v2_format,
    use_orderless_transactions,
    case(false, false),
    case(true, false),
    case(true, true)
)]
async fn test_get_account_resources_by_ledger_version_with_shard_context(
    use_txn_payload_v2_format: bool,
    use_orderless_transactions: bool,
) {
    let shard_context = new_test_context_with_orderless_flags(
        current_function_name!(),
        use_txn_payload_v2_format,
        use_orderless_transactions,
    );
    test_account_resources_by_ledger_version_with_context(shard_context).await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_get_account_resources_by_too_large_ledger_version() {
    let mut context = new_test_context(current_function_name!());
    let account = context.root_account().await;
    let resp = context
        .expect_status_code(404)
        .get(&account_resources_with_ledger_version(
            &account.address().to_hex_literal(),
            1000000000000000000,
        ))
        .await;
    context.check_golden_output(resp);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_get_account_resources_by_invalid_ledger_version() {
    let mut context = new_test_context(current_function_name!());
    let resp = context
        .expect_status_code(400)
        .get(&account_resources_with_ledger_version(
            &context.root_account().await.address().to_hex_literal(),
            -1,
        ))
        .await;
    context.check_golden_output(resp);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_account_auto_creation() {
    let mut context = new_test_context(current_function_name!());
    let root_account = context.root_account().await;
    let account = context.gen_account();
    let txn1 = root_account.sign_with_transaction_builder(context.transaction_factory().payload(
        aptos_stdlib::coin_migrate_to_fungible_store(AptosCoinType::type_tag()),
    ));
    let txn2 = root_account.sign_with_transaction_builder(context.transaction_factory().payload(
        aptos_stdlib::aptos_account_fungible_transfer_only(account.address(), 10_000_000_000),
    ));
    context
        .commit_block(&vec![txn1.clone(), txn2.clone()])
        .await;
    let txn = account.sign_with_transaction_builder(
        context
            .transaction_factory()
            .payload(aptos_stdlib::aptos_account_fungible_transfer_only(
                root_account.address(),
                1,
            ))
            .gas_unit_price(1),
    );
    context.commit_block(&vec![txn.clone()]).await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[rstest(
    use_txn_payload_v2_format,
    use_orderless_transactions,
    case(false, false),
    case(true, false),
    case(true, true)
)]
async fn test_get_account_balance(
    use_txn_payload_v2_format: bool,
    use_orderless_transactions: bool,
) {
    let mut context = new_test_context_with_orderless_flags(
        current_function_name!(),
        use_txn_payload_v2_format,
        use_orderless_transactions,
    );
    let root_account = context.root_account().await;

    // First check coin balance
    let coin_balance_before = context
        .get(&account_balance(
            &root_account.address().to_hex_literal(),
            APTOS_COIN_TYPE_STR,
        ))
        .await;
    let txn = root_account.sign_with_transaction_builder(
        context
            .transaction_factory()
            .payload(aptos_stdlib::coin_migrate_to_fungible_store(
                AptosCoinType::type_tag(),
            ))
            .expiration_timestamp_secs(context.get_expiration_time())
            .upgrade_payload(
                &mut context.rng,
                context.use_txn_payload_v2_format,
                context.use_orderless_transactions,
            ),
    );
    context.commit_block(&vec![txn.clone()]).await;

    // Check coin balance after migration
    let coin_balance_after = context
        .get(&account_balance(
            &root_account.address().to_hex_literal(),
            APTOS_COIN_TYPE_STR,
        ))
        .await;
    assert_eq!(coin_balance_before, coin_balance_after);

    // Check fungible asset balance
    let fa_balance = context
        .get(&account_balance(
            &root_account.address().to_hex_literal(),
            &AccountAddress::TEN.to_hex_literal(),
        ))
        .await;
    assert_eq!(coin_balance_after, fa_balance);
    // upgrade to concurrent store
    let txn = root_account.sign_with_transaction_builder(
        context
            .transaction_factory()
            .payload(TransactionPayload::EntryFunction(EntryFunction::new(
                ModuleId::new(
                    AccountAddress::TEN,
                    Identifier::new("fungible_asset").unwrap(),
                ),
                Identifier::new("upgrade_store_to_concurrent").unwrap(),
                vec![TypeTag::Struct(Box::new(ObjectCoreResource::struct_tag()))],
                vec![bcs::to_bytes(&primary_apt_store(root_account.address())).unwrap()],
            )))
            .expiration_timestamp_secs(context.get_expiration_time())
            .upgrade_payload(
                &mut context.rng,
                context.use_txn_payload_v2_format,
                context.use_orderless_transactions,
            ),
    );
    context.commit_block(&vec![txn.clone()]).await;

    // Check concurrent fungible asset balance
    let concurrent_fa_balance = context
        .get(&account_balance(
            &root_account.address().to_hex_literal(),
            &AccountAddress::TEN.to_hex_literal(),
        ))
        .await;
    assert_eq!(concurrent_fa_balance, fa_balance);
}

async fn test_get_account_modules_by_ledger_version_with_context(mut context: TestContext) {
    let initial_ledger_version = u64::from(context.get_latest_ledger_info().ledger_version);
    let payload =
        aptos_stdlib::publish_module_source("test_module", "module 0xa550c18::test_module {}");

    let root_account = context.root_account().await;
    let txn = root_account.sign_with_transaction_builder(
        context
            .transaction_factory()
            .payload(payload)
            .expiration_timestamp_secs(context.get_expiration_time())
            .upgrade_payload(
                &mut context.rng,
                context.use_txn_payload_v2_format,
                context.use_orderless_transactions,
            ),
    );
    context.commit_block(&vec![txn.clone()]).await;

    if let Some(indexer_reader) = context.context.indexer_reader.as_ref() {
        // Waiting for the above transaction, block metadata, and state checkpoint to be indexed.
        indexer_reader
            .wait_for_internal_indexer(initial_ledger_version + 3)
            .unwrap();
    }

    let modules = context
        .get(&account_modules(
            &context.root_account().await.address().to_hex_literal(),
        ))
        .await;
    assert_ne!(modules, json!([]));

    // Making sure the module is not in the account modules initially.
    let modules = context
        .get(&account_modules_with_ledger_version(
            &context.root_account().await.address().to_hex_literal(),
            initial_ledger_version,
        ))
        .await;
    assert_eq!(modules, json!([]));
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[rstest(
    use_txn_payload_v2_format,
    use_orderless_transactions,
    case(false, false),
    case(true, false),
    case(true, true)
)]
async fn test_get_account_modules_by_ledger_version(
    use_txn_payload_v2_format: bool,
    use_orderless_transactions: bool,
) {
    let context = new_test_context_with_orderless_flags(
        current_function_name!(),
        use_txn_payload_v2_format,
        use_orderless_transactions,
    );
    test_get_account_modules_by_ledger_version_with_context(context).await;
    let shard_context = new_test_context_with_orderless_flags(
        current_function_name!(),
        use_txn_payload_v2_format,
        use_orderless_transactions,
    );
    test_get_account_modules_by_ledger_version_with_context(shard_context).await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[rstest(
    use_txn_payload_v2_format,
    use_orderless_transactions,
    case(false, false),
    case(true, false),
    case(true, true)
)]
async fn account_resource_created_only_by_seq_number_based_txns(
    use_txn_payload_v2_format: bool,
    use_orderless_transactions: bool,
) {
    let mut context = new_test_context_with_orderless_flags(
        current_function_name!(),
        use_txn_payload_v2_format,
        use_orderless_transactions,
    );

    // Prepare accounts
    let mut user = context.create_account().await;
    let user_addr = user.address();

    let resp = context
        .get(&account_resources(&user_addr.to_hex_literal()))
        .await
        .to_string();
    assert!(!resp.contains("0x1::account::Account"));

    // Publish packages
    let named_addresses = vec![("event".to_string(), user_addr)];
    let txn = futures::executor::block_on(async move {
        let path = PathBuf::from(std::env!("CARGO_MANIFEST_DIR"))
            .join("../aptos-move/move-examples/event");
        TestContext::build_package(path, named_addresses)
    });
    context.publish_package(&mut user, txn).await;

    let resp = context
        .get(&account_resources(&user_addr.to_hex_literal()))
        .await
        .to_string();
    if use_orderless_transactions {
        assert!(!resp.contains("0x1::account::Account"));
    } else {
        assert!(resp.contains("0x1::account::Account"));
    }

    context
        .api_execute_entry_function(
            &mut user,
            &format!("0x{}::event::emit", user_addr.to_hex()),
            json!([]),
            json!(["7"]),
        )
        .await;

    let resp = context
        .get(&account_resources(&user_addr.to_hex_literal()))
        .await
        .to_string();
    if use_orderless_transactions {
        assert!(!resp.contains("0x1::account::Account"));
    } else {
        assert!(resp.contains("0x1::account::Account"));
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_get_core_account_data() {
    let mut context = new_test_context(current_function_name!());
    let resp = context.get("/accounts/0x1").await;
    context.check_golden_output(resp);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_get_core_account_data_not_found() {
    let mut context = new_test_context(current_function_name!());
    let resp = context.expect_status_code(200).get("/accounts/0xf").await;
    context.check_golden_output(resp);
    context
        .disable_feature(aptos_types::on_chain_config::FeatureFlag::DEFAULT_ACCOUNT_RESOURCE as u64)
        .await;
    context.expect_status_code(404).get("/accounts/0xf").await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_get_account_resources_with_pagination() {
    let context = new_test_context(current_function_name!());
    let address = "0x1";

    // Make a request with no limit. We'll use this full list of resources
    // as a comparison with the results from using pagination parameters.
    // There should be no cursor in the header in this case. Note: This won't
    // be true if for some reason the account used in this test has more than
    // the default max page size for resources (1000 at the time of writing,
    // based on config/src/config/api_config.rs).
    let req = warp::test::request()
        .method("GET")
        .path(&format!("/v1{}", account_resources(address)));
    let resp = context.reply(req).await;
    assert_eq!(resp.status(), 200);
    assert!(!resp.headers().contains_key("X-Aptos-Cursor"));
    let all_resources: Vec<MoveResource> = serde_json::from_slice(resp.body()).unwrap();
    // We assert there are at least 10 resources. If there aren't, the rest of the
    // test will be wrong.
    assert!(all_resources.len() >= 10);

    // Make a request, assert we get a cursor back in the header for the next
    // page of results. Assert we can deserialize the string representation
    // of the cursor returned in the header.
    // FIXME: Pagination seems to be off by one (change 4 to 5 below and see what happens).
    let req = warp::test::request()
        .method("GET")
        .path(&format!("/v1{}?limit=4", account_resources(address)));
    let resp = context.reply(req).await;
    assert_eq!(resp.status(), 200);
    let cursor_header = resp
        .headers()
        .get("X-Aptos-Cursor")
        .expect("Cursor header was missing");
    let cursor_header = StateKeyWrapper::from_str(cursor_header.to_str().unwrap()).unwrap();
    let resources: Vec<MoveResource> = serde_json::from_slice(resp.body()).unwrap();
    println!("Returned {} resources:", resources.len());
    for r in resources
        .iter()
        .map(|mvr| &mvr.typ)
        .collect::<Vec<&MoveStructTag>>()
    {
        println!("0x1::{}::{}", r.module, r.name);
    }
    assert_eq!(resources.len(), 4);
    assert_eq!(resources, all_resources[0..4].to_vec());

    // Make a request using the cursor. Assert the 5 results we get back are the next 5.
    let req = warp::test::request().method("GET").path(&format!(
        "/v1{}?limit=5&start={}",
        account_resources(address),
        cursor_header
    ));
    let resp = context.reply(req).await;
    assert_eq!(resp.status(), 200);
    let cursor_header = resp
        .headers()
        .get("X-Aptos-Cursor")
        .expect("Cursor header was missing");
    let cursor_header = StateKeyWrapper::from_str(cursor_header.to_str().unwrap()).unwrap();
    let resources: Vec<MoveResource> = serde_json::from_slice(resp.body()).unwrap();
    assert_eq!(resources.len(), 5);
    assert_eq!(resources, all_resources[4..9].to_vec());

    // Get the rest of the resources, assert there is no cursor now.
    let req = warp::test::request().method("GET").path(&format!(
        "/v1{}?limit=1000&start={}",
        account_resources(address),
        cursor_header
    ));
    let resp = context.reply(req).await;
    assert_eq!(resp.status(), 200);
    assert!(!resp.headers().contains_key("X-Aptos-Cursor"));
    let resources: Vec<MoveResource> = serde_json::from_slice(resp.body()).unwrap();
    assert_eq!(resources.len(), all_resources.len() - 9);
    assert_eq!(resources, all_resources[9..].to_vec());
}

// Same as the above test but for modules.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_get_account_modules_with_pagination() {
    let context = new_test_context(current_function_name!());
    let address = "0x1";

    // Make a request with no limit. We'll use this full list of modules
    // as a comparison with the results from using pagination parameters.
    // There should be no cursor in the header in this case. Note: This won't
    // be true if for some reason the account used in this test has more than
    // the default max page size for modules (1000 at the time of writing,
    // based on config/src/config/api_config.rs).
    let req = warp::test::request()
        .method("GET")
        .path(&format!("/v1{}", account_modules(address)));
    let resp = context.reply(req).await;
    assert_eq!(resp.status(), 200);
    assert!(!resp.headers().contains_key("X-Aptos-Cursor"));
    let all_modules: Vec<MoveModuleBytecode> = serde_json::from_slice(resp.body()).unwrap();
    // We assert there are at least 10 modules. If there aren't, the rest of the
    // test will be wrong.
    assert!(all_modules.len() >= 10);

    // Make a request, assert we get a cursor back in the header for the next
    // page of results. Assert we can deserialize the string representation
    // of the cursor returned in the header.
    let req = warp::test::request()
        .method("GET")
        .path(&format!("/v1{}?limit=5", account_modules(address)));
    let resp = context.reply(req).await;
    assert_eq!(resp.status(), 200);
    let cursor_header = resp
        .headers()
        .get("X-Aptos-Cursor")
        .expect("Cursor header was missing");
    let cursor_header = StateKeyWrapper::from_str(cursor_header.to_str().unwrap()).unwrap();
    let modules: Vec<MoveModuleBytecode> = serde_json::from_slice(resp.body()).unwrap();
    assert_eq!(modules.len(), 5);
    assert_eq!(modules, all_modules[0..5].to_vec());

    // Make a request using the cursor. Assert the 5 results we get back are the next 5.
    let req = warp::test::request().method("GET").path(&format!(
        "/v1{}?limit=5&start={}",
        account_modules(address),
        cursor_header
    ));
    let resp = context.reply(req).await;
    assert_eq!(resp.status(), 200);
    let cursor_header = resp
        .headers()
        .get("X-Aptos-Cursor")
        .expect("Cursor header was missing");
    let cursor_header = StateKeyWrapper::from_str(cursor_header.to_str().unwrap()).unwrap();
    let modules: Vec<MoveModuleBytecode> = serde_json::from_slice(resp.body()).unwrap();
    assert_eq!(modules.len(), 5);
    assert_eq!(modules, all_modules[5..10].to_vec());

    // Get the rest of the modules, assert there is no cursor now.
    let req = warp::test::request().method("GET").path(&format!(
        "/v1{}?limit=1000&start={}",
        account_modules(address),
        cursor_header
    ));
    let resp = context.reply(req).await;
    assert_eq!(resp.status(), 200);
    assert!(!resp.headers().contains_key("X-Aptos-Cursor"));
    let modules: Vec<MoveModuleBytecode> = serde_json::from_slice(resp.body()).unwrap();
    assert_eq!(modules.len(), all_modules.len() - 10);
    assert_eq!(modules, all_modules[10..].to_vec());
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_get_account_items_limit_params() {
    let context = new_test_context(current_function_name!());
    let address = "0x1";

    // Ensure limit=0 is rejected.
    let req = warp::test::request()
        .method("GET")
        .path(&format!("/v1{}?limit=0", account_resources(address)));
    let resp = context.reply(req).await;
    assert_eq!(resp.status(), 400);

    // Ensure limit=0 is rejected.
    let req = warp::test::request()
        .method("GET")
        .path(&format!("/v1{}?limit=0", account_modules(address)));
    let resp = context.reply(req).await;
    assert_eq!(resp.status(), 400);

    // Ensure garbage start param values are rejected.
    let req = warp::test::request().method("GET").path(&format!(
        "/v1{}?start=iwouldnotsurviveavibecheckrightnow",
        account_modules(address)
    ));
    let resp = context.reply(req).await;
    assert_eq!(resp.status(), 400);
}

fn account_resources(address: &str) -> String {
    format!("/accounts/{}/resources", address)
}

fn account_resources_with_ledger_version(address: &str, ledger_version: i128) -> String {
    format!(
        "{}?ledger_version={}",
        account_resources(address),
        ledger_version
    )
}

fn account_modules(address: &str) -> String {
    format!("/accounts/{}/modules", address)
}

fn account_balance(address: &str, coin_type: &str) -> String {
    format!("/accounts/{}/balance/{}", address, coin_type)
}

fn account_modules_with_ledger_version(address: &str, ledger_version: u64) -> String {
    format!(
        "{}?ledger_version={}",
        account_modules(address),
        ledger_version
    )
}
