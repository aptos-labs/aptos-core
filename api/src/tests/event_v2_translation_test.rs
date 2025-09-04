// Copyright © Velor Foundation
// Parts of the project are originally copyright © Meta Platforms, Inc.
// SPDX-License-Identifier: Apache-2.0

use super::new_test_context;
use crate::tests::new_test_context_with_orderless_flags;
use velor_api_test_context::{current_function_name, TestContext};
use velor_crypto::{ed25519::Ed25519PrivateKey, SigningKey, ValidCryptoMaterial};
use velor_sdk::types::LocalAccount;
use velor_types::account_config::RotationProofChallenge;
use move_core_types::{account_address::AccountAddress, language_storage::CORE_CODE_ADDRESS};
use rstest::rstest;
use serde_json::{json, Value};
use std::path::PathBuf;

static MODULE_EVENT_MIGRATION: u64 = 57;
static NEW_ACCOUNTS_DEFAULT_TO_FA_APT_STORE: u64 = 64;
static OPERATIONS_DEFAULT_TO_FA_APT_STORE: u64 = 65;
static NEW_ACCOUNTS_DEFAULT_TO_FA_STORE: u64 = 90;

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_feature_enable_disable() {
    let mut context = new_test_context(current_function_name!());
    context.enable_feature(MODULE_EVENT_MIGRATION).await;
    assert!(context.is_feature_enabled(MODULE_EVENT_MIGRATION).await);
    context.disable_feature(MODULE_EVENT_MIGRATION).await;
    assert!(!context.is_feature_enabled(MODULE_EVENT_MIGRATION).await);
    context.enable_feature(MODULE_EVENT_MIGRATION).await;
    assert!(context.is_feature_enabled(MODULE_EVENT_MIGRATION).await);
}

#[allow(clippy::cmp_owned)]
fn matches_event_details(
    event: &Value,
    event_type: &str,
    creation_number: u64,
    account_address: AccountAddress,
    sequence_number: u64,
) -> bool {
    event["type"] == event_type
        && event["guid"]["creation_number"] == creation_number.to_string()
        && event["guid"]["account_address"] == account_address.to_hex_literal()
        && event["sequence_number"] == sequence_number.to_string()
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[ignore]
async fn test_event_v2_translation_coin_deposit_event() {
    let context = &mut new_test_context(current_function_name!());

    // Start with the MODULE_EVENT_MIGRATION feature disabled
    context.disable_feature(MODULE_EVENT_MIGRATION).await;

    // Create two accounts
    let account1 = &mut context.api_create_account().await;
    let account2 = &mut context.api_create_account().await;

    // Transfer coins from account1 to account2, emitting V1 events as the feature is disabled
    context
        .api_execute_velor_account_transfer(account2, account1.address(), 101)
        .await;

    // Enable the MODULE_EVENT_MIGRATION feature
    context.enable_feature(MODULE_EVENT_MIGRATION).await;

    // Check the simulation API outputs the translated V1 event rather than the V2 event as it is
    let payload = json!({
        "type": "entry_function_payload",
        "function": "0x1::coin::transfer",
        "type_arguments": ["0x1::velor_coin::VelorCoin"],
        "arguments": [
            account1.address().to_hex_literal(), "102"
        ]
    });
    let resp = context.simulate_transaction(account2, payload, 200).await;

    let is_expected_event = |e: &Value| {
        matches_event_details(e, "0x1::coin::DepositEvent", 2, account1.address(), 2)
            && e["data"]["amount"] == "102"
    };

    assert!(resp[0]["events"]
        .as_array()
        .unwrap()
        .iter()
        .any(is_expected_event));

    // Transfer coins from account2 to account1, emitting V2 events as the feature is enabled
    context
        .api_execute_velor_account_transfer(account2, account1.address(), 102)
        .await;
    context.wait_for_internal_indexer_caught_up().await;

    // Check the event_by_creation_number API outputs the translated V1 event
    let resp = context
        .gen_events_by_creation_num(&account1.address(), 2)
        .await;
    assert!(is_expected_event(resp.as_array().unwrap().last().unwrap()));

    // Check the event_by_handle API outputs the translated V1 event
    let resp = context
        .gen_events_by_handle(
            &account1.address(),
            "0x1::coin::CoinStore%3C0x1::velor_coin::VelorCoin%3E",
            "deposit_events",
        )
        .await;
    assert!(is_expected_event(resp.as_array().unwrap().last().unwrap()));

    // Check the accounts-transactions API outputs the translated V1 event
    if !context.use_orderless_transactions {
        // /accounts/:address/transactions only outputs sequence number based transactions from the account
        let resp = context
            .get(
                format!(
                    "/accounts/{}/transactions?limit=1",
                    account2.address().to_hex_literal()
                )
                .as_str(),
            )
            .await;
        assert!(resp[0]["events"]
            .as_array()
            .unwrap()
            .iter()
            .any(is_expected_event));
    };
    let resp = context
        .get(
            format!(
                "/accounts/{}/transaction_summaries?limit=1",
                account2.address().to_hex_literal()
            )
            .as_str(),
        )
        .await;
    let hash = resp[0]["transaction_hash"].as_str().unwrap();
    let version = resp[0]["version"].as_str().unwrap();

    // Check the transactions API outputs the translated V1 event
    let resp = context
        .get(format!("/transactions?start={}&limit=1", version).as_str())
        .await;
    assert!(resp[0]["events"]
        .as_array()
        .unwrap()
        .iter()
        .any(is_expected_event));

    // Check the transactions_by_hash API outputs the translated V1 event
    let resp = context
        .get(format!("/transactions/by_hash/{}", hash).as_str())
        .await;
    assert!(resp["events"]
        .as_array()
        .unwrap()
        .iter()
        .any(is_expected_event));

    // Check the transactions_by_version API outputs the translated V1 event
    let resp = context
        .get(format!("/transactions/by_version/{}", version).as_str())
        .await;
    assert!(resp["events"]
        .as_array()
        .unwrap()
        .iter()
        .any(is_expected_event));
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[ignore]
async fn test_event_v2_translation_coin_withdraw_event() {
    let context = &mut new_test_context(current_function_name!());

    // Start with the MODULE_EVENT_MIGRATION feature disabled
    context.disable_feature(MODULE_EVENT_MIGRATION).await;
    context
        .disable_feature(NEW_ACCOUNTS_DEFAULT_TO_FA_APT_STORE)
        .await;
    context
        .disable_feature(OPERATIONS_DEFAULT_TO_FA_APT_STORE)
        .await;
    context
        .disable_feature(NEW_ACCOUNTS_DEFAULT_TO_FA_STORE)
        .await;

    // Create two accounts
    let account1 = &mut context.api_create_account().await;
    let account2 = &mut context.api_create_account().await;

    // Transfer coins from account1 to account2, emitting V1 events as the feature is disabled
    context
        .api_execute_velor_account_transfer(account2, account1.address(), 101)
        .await;

    // Enable the MODULE_EVENT_MIGRATION feature
    context.enable_feature(MODULE_EVENT_MIGRATION).await;

    // Check the simulation API outputs the translated V1 event rather than the V2 event as it is
    let payload = json!({
        "type": "entry_function_payload",
        "function": "0x1::coin::transfer",
        "type_arguments": ["0x1::velor_coin::VelorCoin"],
        "arguments": [
            account1.address().to_hex_literal(), "102"
        ]
    });
    let resp = context.simulate_transaction(account2, payload, 200).await;
    let address2_address = account2.address();
    let is_expected_event = |e: &Value| {
        matches_event_details(e, "0x1::coin::WithdrawEvent", 3, address2_address, 1)
            && e["data"]["amount"] == "102"
    };
    assert!(resp[0]["events"]
        .as_array()
        .unwrap()
        .iter()
        .any(is_expected_event));

    // Transfer coins from account2 to account1, emitting V2 events as the feature is enabled
    context
        .api_execute_velor_account_transfer(account2, account1.address(), 102)
        .await;
    context.wait_for_internal_indexer_caught_up().await;

    // Check the event_by_creation_number API outputs the translated V1 event
    let resp = context
        .gen_events_by_creation_num(&account2.address(), 3)
        .await;
    assert!(is_expected_event(resp.as_array().unwrap().last().unwrap()));

    // Check the event_by_handle API outputs the translated V1 event
    let resp = context
        .gen_events_by_handle(
            &account2.address(),
            "0x1::coin::CoinStore%3C0x1::velor_coin::VelorCoin%3E",
            "withdraw_events",
        )
        .await;
    assert!(is_expected_event(resp.as_array().unwrap().last().unwrap()));

    // Check the accounts-transactions API outputs the translated V1 event
    if !context.use_orderless_transactions {
        // /accounts/:address/transactions only outputs sequence number based transactions from the account
        let resp = context
            .get(
                format!(
                    "/accounts/{}/transactions?limit=1",
                    account2.address().to_hex_literal()
                )
                .as_str(),
            )
            .await;
        assert!(resp[0]["events"]
            .as_array()
            .unwrap()
            .iter()
            .any(is_expected_event));
    }
    let resp = context
        .get(
            format!(
                "/accounts/{}/transaction_summaries?limit=1",
                account2.address().to_hex_literal()
            )
            .as_str(),
        )
        .await;
    let hash = resp[0]["transaction_hash"].as_str().unwrap();
    let version = resp[0]["version"].as_str().unwrap();

    // Check the transactions API outputs the translated V1 event
    let resp = context
        .get(format!("/transactions?start={}&limit=1", version).as_str())
        .await;
    assert!(resp[0]["events"]
        .as_array()
        .unwrap()
        .iter()
        .any(is_expected_event));

    // Check the transactions_by_hash API outputs the translated V1 event
    let resp = context
        .get(format!("/transactions/by_hash/{}", hash).as_str())
        .await;
    assert!(resp["events"]
        .as_array()
        .unwrap()
        .iter()
        .any(is_expected_event));

    // Check the transactions_by_version API outputs the translated V1 event
    let resp = context
        .get(format!("/transactions/by_version/{}", version).as_str())
        .await;
    assert!(resp["events"]
        .as_array()
        .unwrap()
        .iter()
        .any(is_expected_event));
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[ignore]
async fn test_event_v2_translation_account_coin_register_event() {
    let context = &mut new_test_context(current_function_name!());

    // Make sure that the MODULE_EVENT_MIGRATION feature is enabled
    context.enable_feature(MODULE_EVENT_MIGRATION).await;
    context
        .disable_feature(NEW_ACCOUNTS_DEFAULT_TO_FA_APT_STORE)
        .await;
    context
        .disable_feature(OPERATIONS_DEFAULT_TO_FA_APT_STORE)
        .await;
    context
        .disable_feature(NEW_ACCOUNTS_DEFAULT_TO_FA_STORE)
        .await;

    // Create two accounts
    let account1 = &mut context.api_create_account().await;
    let account2 = &mut context.gen_account();

    let is_expected_event = |e: &Value| {
        matches_event_details(
            e,
            "0x1::account::CoinRegisterEvent",
            0,
            account2.address(),
            0,
        ) && e["data"]["type_info"]["struct_name"]
            == format!("0x{}", hex::encode("VelorCoin".to_string().as_bytes()))
    };

    // Transfer coins from account2 to account1, emitting V2 events as the feature is enabled
    context
        .api_execute_velor_account_transfer(account1, account2.address(), 102)
        .await;
    context.wait_for_internal_indexer_caught_up().await;

    // Check the event_by_creation_number API outputs the translated V1 event
    let resp = context
        .gen_events_by_creation_num(&account2.address(), 0)
        .await;
    assert!(is_expected_event(resp.as_array().unwrap().last().unwrap()));

    // Check the event_by_handle API outputs the translated V1 event
    let resp = context
        .gen_events_by_handle(
            &account2.address(),
            "0x1::account::Account",
            "coin_register_events",
        )
        .await;
    assert!(is_expected_event(resp.as_array().unwrap().last().unwrap()));

    // Check the accounts-transactions API outputs the translated V1 event
    if !context.use_orderless_transactions {
        // /accounts/:address/transactions only outputs sequence number based transactions from the account
        let resp = context
            .get(
                format!(
                    "/accounts/{}/transactions?limit=1",
                    account1.address().to_hex_literal()
                )
                .as_str(),
            )
            .await;
        assert!(resp[0]["events"]
            .as_array()
            .unwrap()
            .iter()
            .any(is_expected_event));
    }
    let resp = context
        .get(
            format!(
                "/accounts/{}/transaction_summaries?limit=1",
                account1.address().to_hex_literal()
            )
            .as_str(),
        )
        .await;
    let hash = resp[0]["transaction_hash"].as_str().unwrap();
    let version = resp[0]["version"].as_str().unwrap();

    // Check the transactions API outputs the translated V1 event
    let resp = context
        .get(format!("/transactions?start={}&limit=1", version).as_str())
        .await;
    assert!(resp[0]["events"]
        .as_array()
        .unwrap()
        .iter()
        .any(is_expected_event));

    // Check the transactions_by_hash API outputs the translated V1 event
    let resp = context
        .get(format!("/transactions/by_hash/{}", hash).as_str())
        .await;
    assert!(resp["events"]
        .as_array()
        .unwrap()
        .iter()
        .any(is_expected_event));

    // Check the transactions_by_version API outputs the translated V1 event
    let resp = context
        .get(format!("/transactions/by_version/{}", version).as_str())
        .await;
    assert!(resp["events"]
        .as_array()
        .unwrap()
        .iter()
        .any(is_expected_event));
}

fn rotate_authentication_key_payload(
    account: &LocalAccount,
    new_private_key: &Ed25519PrivateKey,
    new_public_key_bytes: Vec<u8>,
) -> Value {
    let from_scheme = 0;
    let to_scheme = 0;

    // Construct a proof challenge struct that proves that
    // the user intends to rotate their auth key.
    let rotation_proof = RotationProofChallenge {
        account_address: CORE_CODE_ADDRESS,
        module_name: String::from("account"),
        struct_name: String::from("RotationProofChallenge"),
        sequence_number: account.sequence_number(),
        originator: account.address(),
        current_auth_key: AccountAddress::from_bytes(account.authentication_key()).unwrap(),
        new_public_key: new_public_key_bytes.clone(),
    };

    let rotation_msg = bcs::to_bytes(&rotation_proof).unwrap();

    // Sign the rotation message by the current private key and the new private key.
    let signature_by_curr_privkey = account.private_key().sign_arbitrary_message(&rotation_msg);
    let signature_by_new_privkey = new_private_key.sign_arbitrary_message(&rotation_msg);

    json!({
        "type": "entry_function_payload",
        "function": "0x1::account::rotate_authentication_key",
        "type_arguments": [],
        "arguments": [
            from_scheme,
            hex::encode(account.public_key().to_bytes()),
            to_scheme,
            hex::encode(new_public_key_bytes),
            hex::encode(signature_by_curr_privkey.to_bytes()),
            hex::encode(signature_by_new_privkey.to_bytes()),
        ]
    })
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[rstest(
    use_txn_payload_v2_format,
    use_orderless_transactions,
    case(false, false),
    case(true, false),
    case(true, true)
)]
async fn test_event_v2_translation_account_key_rotation_event(
    use_txn_payload_v2_format: bool,
    use_orderless_transactions: bool,
) {
    let context = &mut new_test_context_with_orderless_flags(
        current_function_name!(),
        use_txn_payload_v2_format,
        use_orderless_transactions,
    );

    // Make sure that the MODULE_EVENT_MIGRATION feature is enabled
    context.enable_feature(MODULE_EVENT_MIGRATION).await;

    // Create two accounts
    let account1 = &mut context.api_create_account().await;
    let account2 = &mut context.gen_account();

    // Check the simulation API outputs the translated V1 event rather than the V2 event as it is
    let payload = rotate_authentication_key_payload(
        account1,
        account2.private_key(),
        account2.public_key().to_bytes().to_vec(),
    );
    let resp = context
        .simulate_transaction(account1, payload.clone(), 200)
        .await;

    let account1_address = account1.address();
    let account1_authentication_key = account1.authentication_key();
    let is_expected_event = |e: &Value| {
        matches_event_details(e, "0x1::account::KeyRotationEvent", 1, account1_address, 0)
            && e["data"]["old_authentication_key"]
                == format!("0x{}", hex::encode(account1_authentication_key.to_bytes()))
            && e["data"]["new_authentication_key"]
                == format!(
                    "0x{}",
                    hex::encode(account2.authentication_key().to_bytes())
                )
    };

    assert!(resp[0]["events"]
        .as_array()
        .unwrap()
        .iter()
        .any(is_expected_event));

    // Rotate authentication key, emitting V2 events as the feature is enabled
    context.api_execute_txn(account1, payload).await;
    context.wait_for_internal_indexer_caught_up().await;

    // Check the event_by_creation_number API outputs the translated V1 event
    let resp = context
        .gen_events_by_creation_num(&account1.address(), 1)
        .await;
    assert!(resp.as_array().unwrap().iter().any(is_expected_event));

    // Check the event_by_handle API outputs the translated V1 event
    let resp = context
        .gen_events_by_handle(
            &account1.address(),
            "0x1::account::Account",
            "key_rotation_events",
        )
        .await;
    assert!(resp.as_array().unwrap().iter().any(is_expected_event));

    // Check the accounts-transactions API outputs the translated V1 event
    if !context.use_orderless_transactions {
        // /accounts/:address/transactions only outputs sequence number based transactions from the account
        let resp = context
            .get(
                format!(
                    "/accounts/{}/transactions?limit=1",
                    account1.address().to_hex_literal()
                )
                .as_str(),
            )
            .await;
        assert!(resp[0]["events"]
            .as_array()
            .unwrap()
            .iter()
            .any(is_expected_event));
    }
    let resp = context
        .get(
            format!(
                "/accounts/{}/transaction_summaries?limit=1",
                account1.address().to_hex_literal()
            )
            .as_str(),
        )
        .await;
    let hash = resp[0]["transaction_hash"].as_str().unwrap();
    let version = resp[0]["version"].as_str().unwrap();

    // Check the transactions API outputs the translated V1 event
    let resp = context
        .get(format!("/transactions?start={}&limit=1", version).as_str())
        .await;
    assert!(resp[0]["events"]
        .as_array()
        .unwrap()
        .iter()
        .any(is_expected_event));

    // Check the transactions_by_hash API outputs the translated V1 event
    let resp = context
        .get(format!("/transactions/by_hash/{}", hash).as_str())
        .await;
    assert!(resp["events"]
        .as_array()
        .unwrap()
        .iter()
        .any(is_expected_event));

    // Check the transactions_by_version API outputs the translated V1 event
    let resp = context
        .get(format!("/transactions/by_version/{}", version).as_str())
        .await;
    assert!(resp["events"]
        .as_array()
        .unwrap()
        .iter()
        .any(is_expected_event));
}

fn check_for_event_v2_translation_token_objects(
    resp: Value,
    creator_addr: AccountAddress,
    user_addr: AccountAddress,
) -> String {
    // Test TransferTranslator
    assert!(resp["events"].as_array().unwrap().iter().any(|e: &Value| {
        e["type"] == "0x1::object::TransferEvent"
            && e["sequence_number"] == "0"
            && e["data"]["from"] == creator_addr.to_hex_literal()
            && e["data"]["to"] == user_addr.to_hex_literal()
    }));

    // Test TokenMutationTranslator
    assert!(resp["events"].as_array().unwrap().iter().any(|e: &Value| {
        e["type"] == "0x4::token::MutationEvent"
            && e["sequence_number"] == "0"
            && e["data"]["mutated_field_name"] == *"uri"
    }));

    // Test CollectionMutationTranslator
    assert!(resp["events"].as_array().unwrap().iter().any(|e: &Value| {
        e["type"] == "0x4::collection::MutationEvent"
            && e["sequence_number"] == "0"
            && e["data"]["mutated_field_name"] == *"uri"
    }));

    // Test MintTranslator
    // The example Move package uses ConcurrentSupply which doesn't have the mint event handle.
    // So, the mint event is not translated in this case.
    assert!(resp["events"].as_array().unwrap().iter().any(|e: &Value| {
        e["type"] == "0x4::collection::Mint"
            && e["guid"]["account_address"] == *"0x0"
            && e["sequence_number"] == "0"
    }));

    let object_address = resp["events"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|e| {
            e["type"] == "0x4::collection::Mint"
                && e["guid"]["account_address"] == *"0x0"
                && e["sequence_number"] == "0"
        })
        .collect::<Vec<_>>()[0]["data"]["token"]
        .clone()
        .to_string();

    // The first and last char is double quotes. Remove them to get the object address.
    object_address[1..object_address.len() - 1].to_string()
    // The cases with FixedSupply and UnlimitedSupply have been tested in the localnet.
    // In those cases, the mint event is translated correctly as follows:
    //   Object {
    //       "guid": Object {
    //           "creation_number": String("1125899906842626"),
    //           "account_address": String("0x999a601c1abf720ccb54acae160a980f9a35209611a12b1e31e091172ed061fc"),
    //       },
    //       "sequence_number": String("0"),
    //       "type": String("0x4::collection::MintEvent"),
    //       "data": Object {
    //           "index": String("1"),
    //           "token": String("0x7dbdec16c12211da2db477a15941df2495218ceb6c221da7bd3efcb93d75cffe"),
    //       },
    //   },
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[rstest(
    use_txn_payload_v2_format,
    use_orderless_transactions,
    case(false, false),
    case(true, false),
    case(true, true)
)]
async fn test_event_v2_translation_token_objects(
    use_txn_payload_v2_format: bool,
    use_orderless_transactions: bool,
) {
    let context = &mut new_test_context_with_orderless_flags(
        current_function_name!(),
        use_txn_payload_v2_format,
        use_orderless_transactions,
    );

    // Make sure that the MODULE_EVENT_MIGRATION feature is enabled
    context.enable_feature(MODULE_EVENT_MIGRATION).await;

    // Create two accounts
    let creator = &mut context.api_create_account().await;
    let user = &mut context.api_create_account().await;

    let creator_addr = creator.address();
    let user_addr = user.address();

    let named_addresses = vec![("addr".to_string(), creator_addr)];
    let txn = futures::executor::block_on(async move {
        let path = PathBuf::from(std::env!("CARGO_MANIFEST_DIR"))
            .join("src/tests/move/pack_token_objects");
        TestContext::build_package(path, named_addresses)
    });
    context.publish_package(creator, txn).await;
    context.wait_for_internal_indexer_caught_up().await;

    let payload = json!({
        "type": "entry_function_payload",
        "function": format!("{}::token_objects::run", creator_addr.to_hex_literal()),
        "type_arguments": [],
        "arguments": [
            user_addr.to_hex_literal()
        ]
    });
    context.api_execute_txn(creator, payload).await;
    context.wait_for_internal_indexer_caught_up().await;

    if !context.use_orderless_transactions {
        let resp = context
            .get(
                format!(
                    "/accounts/{}/transactions?limit=1",
                    creator_addr.to_hex_literal()
                )
                .as_str(),
            )
            .await;
        check_for_event_v2_translation_token_objects(resp[0].clone(), creator_addr, user_addr);
    }
    let resp = context
        .get(
            format!(
                "/accounts/{}/transaction_summaries?limit=1",
                creator_addr.to_hex_literal()
            )
            .as_str(),
        )
        .await;
    let hash = resp[0]["transaction_hash"].as_str().unwrap();
    let version = resp[0]["version"].as_str().unwrap();

    // Check the transactions API outputs the translated V1 event
    let resp = context
        .get(format!("/transactions?start={}&limit=1", version).as_str())
        .await;
    check_for_event_v2_translation_token_objects(resp[0].clone(), creator_addr, user_addr);

    // Check the transactions_by_hash API outputs the translated V1 event
    let resp = context
        .get(format!("/transactions/by_hash/{}", hash).as_str())
        .await;
    check_for_event_v2_translation_token_objects(resp, creator_addr, user_addr);

    // Check the transactions_by_version API outputs the translated V1 event
    let resp = context
        .get(format!("/transactions/by_version/{}", version).as_str())
        .await;
    let object_address =
        check_for_event_v2_translation_token_objects(resp, creator_addr, user_addr);
    let payload = json!({
        "type": "entry_function_payload",
        "function": format!("{}::token_objects::burn", creator_addr.to_hex_literal()),
        "type_arguments": [],
        "arguments": [
            object_address
        ]
    });
    context.api_execute_txn(creator, payload).await;
    context.wait_for_internal_indexer_caught_up().await;

    if !context.use_orderless_transactions {
        // /accounts/:address/transactions only outputs sequence number based transactions from the account
        let resp = context
            .get(
                format!(
                    "/accounts/{}/transactions?limit=1",
                    creator_addr.to_hex_literal()
                )
                .as_str(),
            )
            .await;
        // Test BurnTranslator
        // The example Move package uses ConcurrentSupply which doesn't have the burn event handle.
        // So, the burn event is not translated in this case.
        assert!(resp[0]["events"]
            .as_array()
            .unwrap()
            .iter()
            .any(|e: &Value| {
                e["type"] == "0x4::collection::Burn"
                    && e["guid"]["account_address"] == *"0x0"
                    && e["sequence_number"] == "0"
            }));
    }
    let resp = context
        .get(
            format!(
                "/accounts/{}/transaction_summaries?limit=1",
                creator_addr.to_hex_literal()
            )
            .as_str(),
        )
        .await;
    let hash = resp[0]["transaction_hash"].as_str().unwrap();
    let version = resp[0]["version"].as_str().unwrap();
    let resp = context
        .get(format!("/transactions?start={}&limit=1", version).as_str())
        .await;
    assert!(resp[0]["events"]
        .as_array()
        .unwrap()
        .iter()
        .any(|e: &Value| {
            e["type"] == "0x4::collection::Burn"
                && e["guid"]["account_address"] == *"0x0"
                && e["sequence_number"] == "0"
        }));

    // Check the transactions_by_hash API outputs the translated V1 event
    let resp = context
        .get(format!("/transactions/by_hash/{}", hash).as_str())
        .await;
    assert!(resp["events"].as_array().unwrap().iter().any(|e: &Value| {
        e["type"] == "0x4::collection::Burn"
            && e["guid"]["account_address"] == *"0x0"
            && e["sequence_number"] == "0"
    }));

    // Check the transactions_by_version API outputs the translated V1 event
    let resp = context
        .get(format!("/transactions/by_version/{}", version).as_str())
        .await;
    assert!(resp["events"].as_array().unwrap().iter().any(|e: &Value| {
        e["type"] == "0x4::collection::Burn"
            && e["guid"]["account_address"] == *"0x0"
            && e["sequence_number"] == "0"
    }));

    // The cases with FixedSupply and UnlimitedSupply have been tested in the localnet.
    // In those cases, the burn event is translated correctly as follows:
    //   Object {
    //       "guid": Object {
    //           "creation_number": String("1125899906842625"),
    //           "account_address": String("0x999a601c1abf720ccb54acae160a980f9a35209611a12b1e31e091172ed061fc"),
    //       },
    //       "sequence_number": String("0"),
    //       "type": String("0x4::collection::BurnEvent"),
    //       "data": Object {
    //           "index": String("1"),
    //           "token": String("0x7dbdec16c12211da2db477a15941df2495218ceb6c221da7bd3efcb93d75cffe"),
    //       },
    //   },
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[rstest(
    use_txn_payload_v2_format,
    use_orderless_transactions,
    case(false, false),
    case(true, false),
    case(true, true)
)]
async fn test_event_v2_translation_token_v1(
    use_txn_payload_v2_format: bool,
    use_orderless_transactions: bool,
) {
    let context = &mut new_test_context_with_orderless_flags(
        current_function_name!(),
        use_txn_payload_v2_format,
        use_orderless_transactions,
    );

    // Make sure that the MODULE_EVENT_MIGRATION feature is enabled
    context.enable_feature(MODULE_EVENT_MIGRATION).await;

    // Create two accounts
    let creator = &mut context.api_create_account().await;
    // let user = &mut context.api_create_account().await;

    let creator_addr = creator.address();
    // let user_addr = user.address();

    let named_addresses = vec![("addr".to_string(), creator_addr)];
    let txn = futures::executor::block_on(async move {
        let path =
            PathBuf::from(std::env!("CARGO_MANIFEST_DIR")).join("src/tests/move/pack_token_v1");
        TestContext::build_package(path, named_addresses)
    });
    context.publish_package(creator, txn).await;
    context.wait_for_internal_indexer_caught_up().await;

    let payload = json!({
        "type": "entry_function_payload",
        "function": format!("{}::token_v1::run", creator_addr.to_hex_literal()),
        "type_arguments": [],
        "arguments": [
        ]
    });
    context.api_execute_txn(creator, payload).await;
    context.wait_for_internal_indexer_caught_up().await;
    if !context.use_orderless_transactions {
        let resp = context
            .get(
                format!(
                    "/accounts/{}/transactions?limit=1",
                    creator_addr.to_hex_literal()
                )
                .as_str(),
            )
            .await;
        check_for_event_v2_translation_token_v1(resp[0].clone(), creator_addr);
    }
    let resp = context
        .get(
            format!(
                "/accounts/{}/transaction_summaries?limit=1",
                creator_addr.to_hex_literal()
            )
            .as_str(),
        )
        .await;
    let hash = resp[0]["transaction_hash"].as_str().unwrap();
    let version = resp[0]["version"].as_str().unwrap();

    // Check the transactions API outputs the translated V1 event
    let resp = context
        .get(format!("/transactions?start={}&limit=1", version).as_str())
        .await;
    check_for_event_v2_translation_token_v1(resp[0].clone(), creator_addr);

    // Check the transactions_by_hash API outputs the translated V1 event
    let resp = context
        .get(format!("/transactions/by_hash/{}", hash).as_str())
        .await;
    check_for_event_v2_translation_token_v1(resp, creator_addr);

    // Check the transactions_by_version API outputs the translated V1 event
    let resp = context
        .get(format!("/transactions/by_version/{}", version).as_str())
        .await;
    check_for_event_v2_translation_token_v1(resp, creator_addr);
}

fn check_for_event_v2_translation_token_v1(resp: Value, creator_addr: AccountAddress) {
    // Test TokenDepositTranslator
    assert!(resp["events"].as_array().unwrap().iter().any(|e: &Value| {
        e["type"] == "0x3::token::DepositEvent"
            && e["sequence_number"] == "4"
            && e["data"]["id"]["token_data_id"]["creator"] == creator_addr.to_hex_literal()
    }));

    // Test TokenWithdrawTranslator
    assert!(resp["events"].as_array().unwrap().iter().any(|e: &Value| {
        e["type"] == "0x3::token::WithdrawEvent"
            && e["sequence_number"] == "4"
            && e["data"]["id"]["token_data_id"]["creator"] == creator_addr.to_hex_literal()
    }));

    // Test BurnTokenTranslator
    assert!(resp["events"].as_array().unwrap().iter().any(|e: &Value| {
        e["type"] == "0x3::token::BurnTokenEvent"
            && e["sequence_number"] == "0"
            && e["data"]["id"]["token_data_id"]["creator"] == creator_addr.to_hex_literal()
    }));

    // Test MutatePropertyMapTranslator
    assert!(resp["events"].as_array().unwrap().iter().any(|e: &Value| {
        e["type"] == "0x3::token::MutateTokenPropertyMapEvent"
            && e["sequence_number"] == "0"
            && e["data"]["new_id"]["property_version"] == "1"
    }));

    // Test MintTokenTranslator
    assert!(resp["events"].as_array().unwrap().iter().any(|e: &Value| {
        e["type"] == "0x3::token::MintTokenEvent"
            && e["sequence_number"] == "0"
            && e["data"]["amount"] == "10"
    }));

    // Test CreateCollectionTranslator
    assert!(resp["events"].as_array().unwrap().iter().any(|e: &Value| {
        e["type"] == "0x3::token::CreateCollectionEvent"
            && e["sequence_number"] == "0"
            && e["data"]["creator"] == creator_addr.to_hex_literal()
    }));

    // Test TokenDataCreationTranslator
    assert!(resp["events"].as_array().unwrap().iter().any(|e: &Value| {
        e["type"] == "0x3::token::CreateTokenDataEvent"
            && e["sequence_number"] == "0"
            && e["data"]["name"] == "Token 1"
    }));

    // Test OfferTranslator
    assert!(resp["events"].as_array().unwrap().iter().any(|e: &Value| {
        e["type"] == "0x3::token_transfers::TokenOfferEvent"
            && e["sequence_number"] == "1"
            && e["data"]["amount"] == "1"
    }));

    // Test CancelOfferTranslator
    assert!(resp["events"].as_array().unwrap().iter().any(|e: &Value| {
        e["type"] == "0x3::token_transfers::TokenCancelOfferEvent"
            && e["sequence_number"] == "0"
            && e["data"]["amount"] == "1"
    }));

    // Test ClaimTranslator
    assert!(resp["events"].as_array().unwrap().iter().any(|e: &Value| {
        e["type"] == "0x3::token_transfers::TokenClaimEvent"
            && e["sequence_number"] == "0"
            && e["data"]["amount"] == "1"
    }));

    // Test CollectionDescriptionMutateTranslator
    assert!(resp["events"].as_array().unwrap().iter().any(|e: &Value| {
        e["type"] == "0x3::token_event_store::CollectionDescriptionMutateEvent"
            && e["sequence_number"] == "0"
            && e["data"]["creator_addr"] == creator_addr.to_hex_literal()
    }));

    // Test CollectionUriMutateTranslator
    assert!(resp["events"].as_array().unwrap().iter().any(|e: &Value| {
        e["type"] == "0x3::token_event_store::CollectionUriMutateEvent"
            && e["sequence_number"] == "0"
            && e["data"]["creator_addr"] == creator_addr.to_hex_literal()
    }));

    // Test CollectionMaximumMutateTranslator
    assert!(resp["events"].as_array().unwrap().iter().any(|e: &Value| {
        e["type"] == "0x3::token_event_store::CollectionMaxiumMutateEvent"
            && e["sequence_number"] == "0"
            && e["data"]["creator_addr"] == creator_addr.to_hex_literal()
    }));

    // Test UriMutationTranslator
    assert!(resp["events"].as_array().unwrap().iter().any(|e: &Value| {
        e["type"] == "0x3::token_event_store::UriMutationEvent"
            && e["sequence_number"] == "0"
            && e["data"]["creator"] == creator_addr.to_hex_literal()
    }));

    // Test DefaultPropertyMutateTranslator
    assert!(resp["events"].as_array().unwrap().iter().any(|e: &Value| {
        e["type"] == "0x3::token_event_store::DefaultPropertyMutateEvent"
            && e["sequence_number"] == "0"
            && e["data"]["creator"] == creator_addr.to_hex_literal()
    }));

    // Test DescriptionMutateTranslator
    assert!(resp["events"].as_array().unwrap().iter().any(|e: &Value| {
        e["type"] == "0x3::token_event_store::DescriptionMutateEvent"
            && e["sequence_number"] == "0"
            && e["data"]["creator"] == creator_addr.to_hex_literal()
    }));

    // Test RoyaltyMutateTranslator
    assert!(resp["events"].as_array().unwrap().iter().any(|e: &Value| {
        e["type"] == "0x3::token_event_store::RoyaltyMutateEvent"
            && e["sequence_number"] == "0"
            && e["data"]["creator"] == creator_addr.to_hex_literal()
    }));

    // Test MaximumMutateTranslator
    assert!(resp["events"].as_array().unwrap().iter().any(|e: &Value| {
        e["type"] == "0x3::token_event_store::MaxiumMutateEvent"
            && e["sequence_number"] == "0"
            && e["data"]["creator"] == creator_addr.to_hex_literal()
    }));

    // Test OptInTransferTranslator
    assert!(resp["events"].as_array().unwrap().iter().any(|e: &Value| {
        e["type"] == "0x3::token_event_store::OptInTransferEvent"
            && e["sequence_number"] == "0"
            && e["data"]["opt_in"] == Value::Bool(true)
    }));
}
