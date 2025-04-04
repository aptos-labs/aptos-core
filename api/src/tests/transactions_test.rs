// Copyright © Aptos Foundation
// Parts of the project are originally copyright © Meta Platforms, Inc.
// SPDX-License-Identifier: Apache-2.0

use super::new_test_context;
use crate::tests::{
    new_test_context_with_config, new_test_context_with_db_sharding_and_internal_indexer,
    new_test_context_with_orderless_flags,
    new_test_context_with_sharding_and_delayed_internal_indexer,
};
use aptos_api_test_context::{assert_json, current_function_name, pretty, TestContext};
use aptos_config::config::{GasEstimationStaticOverride, NodeConfig};
use aptos_crypto::{
    ed25519::{Ed25519PrivateKey, Ed25519Signature},
    multi_ed25519::{MultiEd25519PrivateKey, MultiEd25519PublicKey},
    PrivateKey, SigningKey, Uniform,
};
use aptos_sdk::types::{AccountKey, LocalAccount};
use aptos_types::{
    account_address::AccountAddress,
    account_config::aptos_test_root_address,
    transaction::{
        authenticator::{AuthenticationKey, TransactionAuthenticator},
        EntryFunction, ReplayProtector, Script, SignedTransaction,
    },
    utility_coin::{AptosCoinType, CoinType},
};
use move_core_types::{
    identifier::Identifier,
    language_storage::{ModuleId, TypeTag},
};
use poem_openapi::types::ParseFromJSON;
use rand::{distributions::Alphanumeric, thread_rng, Rng};
use rstest::rstest;
use serde_json::{json, Value};
use std::{path::PathBuf, time::Duration};
use tokio::time::sleep;

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_deserialize_genesis_transaction() {
    let context = new_test_context(current_function_name!());
    let resp = context.get("/transactions/by_version/0").await;
    // TODO: serde_json::from_value doesn't work here, either make it work
    // or remove the ability to do that.
    aptos_api_types::Transaction::parse_from_json(Some(resp)).unwrap();
}

// Unstable due to framework changes
#[ignore]
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_get_transactions_output_genesis_transaction() {
    let mut context = new_test_context(current_function_name!());
    let resp = context.get("/transactions").await;
    context.check_golden_output(resp);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[rstest(
    case_name,
    use_txn_payload_v2_format,
    use_orderless_transactions,
    case("", false, false),
    case("_payload_v2", true, false),
    case("_orderless", true, true)
)]
async fn test_get_transactions_returns_last_page_when_start_version_is_not_specified(
    case_name: &str,
    use_txn_payload_v2_format: bool,
    use_orderless_transactions: bool,
) {
    let mut context = new_test_context_with_orderless_flags(
        current_function_name!() + case_name,
        use_txn_payload_v2_format,
        use_orderless_transactions,
    );

    let mut root_account = context.root_account().await;
    for _i in 0..20 {
        let account = context.gen_account();
        let txn = context.create_user_account_by(&mut root_account, &account);
        context.commit_block(&vec![txn.clone()]).await;
    }

    let resp = context.get("/transactions").await;
    context.check_golden_output(resp);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_get_transactions_with_start_version_is_too_large() {
    let mut context = new_test_context(current_function_name!());
    let resp = context
        .expect_status_code(400)
        .get("/transactions?start=1000000&limit=10")
        .await;
    context.check_golden_output(resp);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_get_transactions_with_invalid_start_version_param() {
    let mut context = new_test_context(current_function_name!());
    let resp = context
        .expect_status_code(400)
        .get("/transactions?start=hello")
        .await;
    context.check_golden_output(resp);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_get_transactions_with_invalid_limit_param() {
    let mut context = new_test_context(current_function_name!());
    let resp = context
        .expect_status_code(400)
        .get("/transactions?limit=hello")
        .await;
    context.check_golden_output(resp);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_get_transactions_with_zero_limit() {
    let mut context = new_test_context(current_function_name!());
    let resp = context
        .expect_status_code(400)
        .get("/transactions?limit=0")
        .await;
    context.check_golden_output(resp);
}

// TODO: Add tests for /transaction_summaries endpoint.

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[ignore]
async fn test_get_transactions_param_limit_exceeds_limit() {
    // Exceeding the limit, will return only the amount expected
    let mut context = new_test_context(current_function_name!());
    let resp = context.get("/transactions?limit=2000").await;
    context.check_golden_output(resp);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[rstest(
    case_name,
    use_txn_payload_v2_format,
    use_orderless_transactions,
    case("", false, false),
    case("_payload_v2", true, false),
    case("_orderless", true, true)
)]
async fn test_get_transactions_output_user_transaction_with_entry_function_payload(
    case_name: &str,
    use_txn_payload_v2_format: bool,
    use_orderless_transactions: bool,
) {
    let mut context = new_test_context_with_orderless_flags(
        current_function_name!() + case_name,
        use_txn_payload_v2_format,
        use_orderless_transactions,
    );
    let initial_ledger_version = u64::from(context.get_latest_ledger_info().ledger_version);
    let account = context.gen_account();
    let txn = context.create_user_account(&account).await;
    context.commit_block(&vec![txn.clone()]).await;

    let txns = context.get("/transactions?start=1").await;
    assert_eq!(
        (initial_ledger_version + 3) as usize,
        txns.as_array().unwrap().len()
    );
    context.check_golden_output(txns);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[rstest(
    case_name,
    use_txn_payload_v2_format,
    use_orderless_transactions,
    case("", false, false),
    case("_payload_v2", true, false),
    case("_orderless", true, true)
)]
async fn test_post_bcs_format_transaction(
    case_name: &str,
    use_txn_payload_v2_format: bool,
    use_orderless_transactions: bool,
) {
    let mut context = new_test_context_with_orderless_flags(
        current_function_name!() + case_name,
        use_txn_payload_v2_format,
        use_orderless_transactions,
    );
    let account = context.gen_account();
    let txn = context.create_user_account(&account).await;
    let body = bcs::to_bytes(&txn).unwrap();
    let resp = context
        .expect_status_code(202)
        .post_bcs_txn("/transactions", body)
        .await;
    context.check_golden_output(resp.clone());

    // ensure ed25519 sig txn can be submitted into mempool by JSON format
    // TODO[Orderless]: When the input is given in JSON format, the /transactions endpoint will use payload v1 format
    // to construct the transactions. So, the supplied signature that is signed on payload v2 format will not match.
    if !context.use_txn_payload_v2_format || context.use_orderless_transactions {
        context
            .expect_status_code(202)
            .post("/transactions", resp)
            .await;
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[rstest(
    use_txn_payload_v2_format,
    use_orderless_transactions,
    case(false, false),
    case(true, false),
    case(true, true)
)]
async fn test_post_invalid_bcs_format_transaction(
    use_txn_payload_v2_format: bool,
    use_orderless_transactions: bool,
) {
    let mut context = new_test_context_with_orderless_flags(
        current_function_name!(),
        use_txn_payload_v2_format,
        use_orderless_transactions,
    );

    let resp = context
        .expect_status_code(400)
        .post_bcs_txn("/transactions", bcs::to_bytes("invalid data").unwrap())
        .await;
    context.check_golden_output(resp);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[rstest(
    use_txn_payload_v2_format,
    use_orderless_transactions,
    case(false, false),
    case(true, false),
    case(true, true)
)]
async fn test_post_invalid_signature_transaction(
    use_txn_payload_v2_format: bool,
    use_orderless_transactions: bool,
) {
    let mut context = new_test_context_with_orderless_flags(
        current_function_name!(),
        use_txn_payload_v2_format,
        use_orderless_transactions,
    );
    let txn = context.create_invalid_signature_transaction().await;
    let body = bcs::to_bytes(&txn).unwrap();
    let resp = context
        .expect_status_code(400)
        .post_bcs_txn("/transactions", &body)
        .await;
    context.check_golden_output(resp);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[rstest(
    use_txn_payload_v2_format,
    use_orderless_transactions,
    case(false, false),
    case(true, false),
    case(true, true)
)]
async fn test_post_transaction_rejected_by_mempool(
    use_txn_payload_v2_format: bool,
    use_orderless_transactions: bool,
) {
    let mut context = new_test_context_with_orderless_flags(
        current_function_name!(),
        use_txn_payload_v2_format,
        use_orderless_transactions,
    );
    let account1 = context.gen_account();
    let account2 = context.gen_account();
    let txn1 = context.create_user_account(&account1).await;
    let txn2 = context.create_user_account(&account2).await;

    context
        .expect_status_code(202)
        .post_bcs_txn("/transactions", &bcs::to_bytes(&txn1).unwrap())
        .await;

    // If txn1 and txn2 are sequence number based transactions, both of them have same sequence number.
    // So, txn2 will be rejected by mempool.
    // But if txn1 and txn2 are orderless transactions, both transactions should be accepted by mempool.
    if context.use_orderless_transactions {
        context
            .expect_status_code(202)
            .post_bcs_txn("/transactions", &bcs::to_bytes(&txn2).unwrap())
            .await;
    } else {
        let resp = context
            .expect_status_code(400)
            .post_bcs_txn("/transactions", &bcs::to_bytes(&txn2).unwrap())
            .await;
        context.check_golden_output(resp);
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[rstest(
    use_txn_payload_v2_format,
    use_orderless_transactions,
    case(false, false),
    case(true, false),
    case(true, true)
)]
async fn test_multi_agent_signed_transaction(
    use_txn_payload_v2_format: bool,
    use_orderless_transactions: bool,
) {
    let mut context = new_test_context_with_orderless_flags(
        current_function_name!(),
        use_txn_payload_v2_format,
        use_orderless_transactions,
    );
    let account = context.gen_account();
    let secondary = context.gen_account();
    let factory = context.transaction_factory();
    let mut root_account = context.root_account().await;

    // Create secondary signer account
    let txn1 = context.create_user_account_by(&mut root_account, &secondary);
    context.commit_block(&[txn1]).await;

    // Create a new account with a multi-agent signer
    let txn = root_account.sign_multi_agent_with_transaction_builder(
        vec![&secondary],
        factory
            .create_user_account(account.public_key())
            .expiration_timestamp_secs(context.get_expiration_time())
            .upgrade_payload(
                &mut context.rng,
                context.use_txn_payload_v2_format,
                context.use_orderless_transactions,
            ),
    );

    let body = bcs::to_bytes(&txn).unwrap();
    let resp = context
        .expect_status_code(202)
        .post_bcs_txn("/transactions", body)
        .await;

    let (sender, secondary_signers) = match txn.authenticator() {
        TransactionAuthenticator::MultiAgent {
            sender,
            secondary_signer_addresses: _,
            secondary_signers,
        } => (sender, secondary_signers),
        _ => panic!(
            "expecting TransactionAuthenticator::MultiAgent, but got: {:?}",
            txn.authenticator()
        ),
    };
    assert_json(
        resp["signature"].clone(),
        json!({
            "type": "multi_agent_signature",
            "sender": {
                "type": "ed25519_signature",
                "public_key": format!("0x{}", hex::encode(sender.public_key_bytes())),
                "signature": format!("0x{}", hex::encode(sender.signature_bytes())),
            },
            "secondary_signer_addresses": [
                secondary.address().to_hex_literal(),
            ],
            "secondary_signers": [
                {
                    "type": "ed25519_signature",
                    "public_key": format!("0x{}",hex::encode(secondary_signers[0].public_key_bytes())),
                    "signature": format!("0x{}", hex::encode(secondary_signers[0].signature_bytes())),
                }
            ]
        }),
    );

    // ensure multi agent txns can be submitted into mempool by JSON format
    // TODO[Orderless]: When the input is given in JSON format, the /transactions endpoint will use payload v1 format
    // to construct the transactions. So, the supplied signature that is signed on payload v2 format will not match.
    if !context.use_txn_payload_v2_format || context.use_orderless_transactions {
        context
            .expect_status_code(202)
            .post("/transactions", resp)
            .await;
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[rstest(
    use_txn_payload_v2_format,
    use_orderless_transactions,
    case(false, false),
    case(true, false),
    case(true, true)
)]
async fn test_fee_payer_signed_transaction(
    use_txn_payload_v2_format: bool,
    use_orderless_transactions: bool,
) {
    let mut context = new_test_context_with_orderless_flags(
        current_function_name!(),
        use_txn_payload_v2_format,
        use_orderless_transactions,
    );
    let account = context.gen_account();
    let fee_payer = context.create_account().await;
    let factory = context.transaction_factory();
    let mut root_account = context.root_account().await;

    let txn1 = context.create_user_account_by(&mut root_account, &fee_payer);
    context.commit_block(&[txn1]).await;

    // Create a new account with a multi-agent signer
    let txn = root_account.sign_fee_payer_with_transaction_builder(
        vec![],
        &fee_payer,
        factory
            .create_user_account(account.public_key())
            .expiration_timestamp_secs(context.get_expiration_time())
            .upgrade_payload(
                &mut context.rng,
                context.use_txn_payload_v2_format,
                context.use_orderless_transactions,
            ),
    );

    let body = bcs::to_bytes(&txn).unwrap();
    let resp = context
        .expect_status_code(202)
        .post_bcs_txn("/transactions", body)
        .await;

    let (sender, _, fee_payer_signer) = match txn.authenticator() {
        TransactionAuthenticator::FeePayer {
            sender,
            secondary_signer_addresses: _,
            secondary_signers,
            fee_payer_address: _,
            fee_payer_signer,
        } => (sender, secondary_signers, fee_payer_signer),
        _ => panic!(
            "expecting TransactionAuthenticator::FeePayer, but got: {:?}",
            txn.authenticator()
        ),
    };
    assert_json(
        resp["signature"].clone(),
        json!({
            "type": "fee_payer_signature",
            "sender": {
                "type": "ed25519_signature",
                "public_key": format!("0x{}", hex::encode(sender.public_key_bytes())),
                "signature": format!("0x{}", hex::encode(sender.signature_bytes())),
            },
            "secondary_signer_addresses": [
            ],
            "secondary_signers": [
            ],
            "fee_payer_address": fee_payer.address().to_hex_literal(),
            "fee_payer_signer": {
                "type": "ed25519_signature",
                "public_key": format!("0x{}",hex::encode(fee_payer_signer.public_key_bytes())),
                "signature": format!("0x{}", hex::encode(fee_payer_signer.signature_bytes())),
            },
        }),
    );

    // ensure fee payer txns can be submitted into mempool by JSON format
    // TODO[Orderless]: When the input is given in JSON format, the /transactions endpoint will use payload v1 format
    // to construct the transactions. So, the supplied signature that is signed on payload v2 format will not match.
    if !context.use_txn_payload_v2_format || context.use_orderless_transactions {
        context
            .expect_status_code(202)
            .post("/transactions", resp)
            .await;
    }

    // Now test for the new format where the fee payer is unset... this also tests account creation
    let another_account = context.gen_account();
    let yet_another_account = context.gen_account();
    let another_raw_txn = another_account
        .sign_fee_payer_with_transaction_builder(
            vec![],
            &fee_payer,
            factory
                .create_user_account(yet_another_account.public_key())
                .max_gas_amount(200_000)
                .gas_unit_price(1)
                .expiration_timestamp_secs(context.get_expiration_time())
                .upgrade_payload(
                    &mut context.rng,
                    context.use_txn_payload_v2_format,
                    context.use_orderless_transactions,
                ),
        )
        .into_raw_transaction();
    let another_txn = another_raw_txn
        .clone()
        .sign_fee_payer(
            another_account.private_key(),
            vec![],
            vec![],
            AccountAddress::ZERO,
            fee_payer.private_key(),
        )
        .unwrap();

    let (sender, secondary_signer_addresses, secondary_signers) = match another_txn.authenticator()
    {
        TransactionAuthenticator::FeePayer {
            sender,
            secondary_signer_addresses,
            secondary_signers,
            fee_payer_address: _,
            fee_payer_signer: _,
        } => (sender, secondary_signer_addresses, secondary_signers),
        _ => panic!(
            "expecting TransactionAuthenticator::FeePayer, but got: {:?}",
            txn.authenticator()
        ),
    };

    let another_txn = another_raw_txn
        .clone()
        .sign_fee_payer(
            another_account.private_key(),
            vec![],
            vec![],
            fee_payer.address(),
            fee_payer.private_key(),
        )
        .unwrap();

    let another_txn = match another_txn.authenticator() {
        TransactionAuthenticator::FeePayer {
            sender: _,
            secondary_signer_addresses: _,
            secondary_signers: _,
            fee_payer_address,
            fee_payer_signer,
        } => {
            let auth = TransactionAuthenticator::fee_payer(
                sender,
                secondary_signer_addresses,
                secondary_signers,
                fee_payer_address,
                fee_payer_signer,
            );
            SignedTransaction::new_signed_transaction(another_raw_txn, auth)
        },
        _ => panic!(
            "expecting TransactionAuthenticator::FeePayer, but got: {:?}",
            txn.authenticator()
        ),
    };

    let body = bcs::to_bytes(&another_txn).unwrap();
    let resp = context
        .expect_status_code(202)
        .post_bcs_txn("/transactions", body)
        .await;
    // TODO[Orderless]: When the input is given in JSON format, the /transactions endpoint will use payload v1 format
    // to construct the transactions. So, the supplied signature that is signed on payload v2 format will not match.
    if !context.use_txn_payload_v2_format || context.use_orderless_transactions {
        context
            .expect_status_code(202)
            .post("/transactions", resp)
            .await;
    }
}

#[ignore]
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_multi_ed25519_signed_transaction() {
    let mut context = new_test_context(current_function_name!());

    let private_key = MultiEd25519PrivateKey::generate_for_testing();
    let public_key = MultiEd25519PublicKey::from(&private_key);
    let auth_key = AuthenticationKey::multi_ed25519(&public_key);

    let factory = context.transaction_factory();
    let root_account = context.root_account().await;
    // TODO: migrate once multi-ed25519 is supported
    let create_account_txn = root_account.sign_with_transaction_builder(
        factory
            .create_user_account(&Ed25519PrivateKey::generate_for_testing().public_key())
            .expiration_timestamp_secs(context.get_expiration_time())
            .upgrade_payload(
                &mut context.rng,
                context.use_txn_payload_v2_format,
                context.use_orderless_transactions,
            ),
    );
    context.commit_block(&vec![create_account_txn]).await;

    let raw_txn = factory
        .mint(auth_key.account_address(), 1000)
        .sender(auth_key.account_address())
        .sequence_number(0)
        .expiration_timestamp_secs(context.get_expiration_time())
        .upgrade_payload(
            &mut context.rng,
            context.use_txn_payload_v2_format,
            context.use_orderless_transactions,
        )
        .build();

    let signature = private_key.sign(&raw_txn).unwrap();
    let txn = SignedTransaction::new_multisig(raw_txn, public_key, signature.clone());

    let body = bcs::to_bytes(&txn).unwrap();
    let resp = context
        .expect_status_code(202)
        .post_bcs_txn("/transactions", body)
        .await;

    assert_json(
        resp["signature"].clone(),
        json!({
          "type": "multi_ed25519_signature",
          "public_keys": [
            "0x9e4208caddd825f71957c9b12dbfbd13a23fb0ea23eb398fd7e1f418b51f8fbc",
            "0x4708a77bb9285ce3745ffdd48c51980326b625488209803228ff623f3768c64e",
            "0x852b13cd7a89b0c223d74504705e84c745d32261244ed233ef0285637a1dece0",
            "0x77e7fe2a510e4f14e15071fc420469ee287b64f2c8f8c0221b946a3fd9cbfef3",
            "0xd0c66cfef88b999f027347726bd54eda4675ae312af9146bfdc9e9fa702cc90a",
            "0xd316059933e0dd6415f00ce350962c8e94b46373b7fb5fb49687f3d6b9e3cb30",
            "0xf20e973e6dfeda74ca8e15f1a7aed9c87d67bd12e071fd3de4240368422712c9",
            "0xead82d6e9e3f3baeaa557bd7a431a1c6fe9f35a82c10fed123f362615ee7c2cd",
            "0x5c048c8c456ff9dd2810343bbd630fb45bf064317efae22c65a1535cf392c5d5",
            "0x861546d0818178f2b5f37af0fa712fe8ce3cceeda894b553ee274f3fbcb4b32f",
            "0xfe047a766a47719591348a4601afb3f38b0c77fa3f820e0298c064e7cde6763f"
          ],
          "signatures": [
                "0xcf9e7a0284434c568cefecd995d2f1c950b041513e815f9bdd8a42cb641c9b6dfcc692b767ace76f4171ef4fa032d3b4687e9944ffbb6b2ebe7033758e55a002",
                "0x840caf50f80da4ca2d4146458da3d93a0fd8e46796d231e36fa426614a10e372a25c2a4843367f6a632fa2459fd6bd8f0a4b35febad4fbdb780fcfba36d81f0b",
                "0xe1523537cc3d2be86df0c65a03cc1168c4d10e9436d8f69bce0e229f8e91c1714a0440e57d9813eedb495a39790fb9090b688173634bfbefe55e194384c45b05"

          ],
          "threshold": 3,
          "bitmap": "0xe0000000"
        }),
    );

    // ensure multi sig txns can be submitted into mempool by JSON format
    context
        .expect_status_code(202)
        .post("/transactions", resp)
        .await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[rstest(
    use_txn_payload_v2_format,
    use_orderless_transactions,
    case(false, false),
    case(true, false),
    case(true, true)
)]
async fn test_get_transaction_by_hash(
    use_txn_payload_v2_format: bool,
    use_orderless_transactions: bool,
) {
    let mut context = new_test_context_with_orderless_flags(
        current_function_name!(),
        use_txn_payload_v2_format,
        use_orderless_transactions,
    );
    let account = context.gen_account();
    let txn = context.create_user_account(&account).await;
    context.commit_block(&vec![txn.clone()]).await;

    let txns = context.get("/transactions?start=2&limit=1").await;
    assert_eq!(1, txns.as_array().unwrap().len());

    let resp = context
        .get(&format!(
            "/transactions/by_hash/{}",
            txns[0]["hash"].as_str().unwrap()
        ))
        .await;
    assert_json(resp, txns[0].clone());
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[rstest(
    case_name,
    use_txn_payload_v2_format,
    use_orderless_transactions,
    case("", false, false),
    case("_payload_v2", true, false),
    case("_orderless", true, true)
)]
async fn test_get_transaction_by_hash_with_delayed_internal_indexer(
    case_name: &str,
    use_txn_payload_v2_format: bool,
    use_orderless_transactions: bool,
) {
    let mut context = new_test_context_with_sharding_and_delayed_internal_indexer(
        current_function_name!() + case_name,
        None,
        use_txn_payload_v2_format,
        use_orderless_transactions,
    );
    let mut account = context.gen_account();
    let txn = context.create_user_account(&account).await;
    context.commit_block(&vec![txn.clone()]).await;
    let txn1 = context.account_transfer_to(
        &mut account,
        AccountAddress::from_hex_literal("0x1").unwrap(),
        1,
    );
    context.commit_block(&vec![txn1.clone()]).await;
    let committed_hash = txn1.committed_hash().to_hex_literal();
    context.wait_for_internal_indexer_caught_up().await;
    let resp = context
        .get(&format!("/transactions/by_hash/{}", committed_hash))
        .await;
    context.check_golden_output(resp);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_get_transaction_by_hash_not_found() {
    let mut context = new_test_context(current_function_name!());

    let resp = context
        .expect_status_code(404)
        .get("/transactions/by_hash/0xdadfeddcca7cb6396c735e9094c76c6e4e9cb3e3ef814730693aed59bd87b31d")
        .await;
    context.check_golden_output(resp);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_get_transaction_by_invalid_hash() {
    let mut context = new_test_context(current_function_name!());

    let resp = context
        .expect_status_code(400)
        .get("/transactions/by_hash/0x1")
        .await;
    context.check_golden_output(resp);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_get_transaction_by_version_not_found() {
    let mut context = new_test_context(current_function_name!());

    let resp = context
        .expect_status_code(404)
        .get("/transactions/by_version/10000")
        .await;
    context.check_golden_output(resp);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[rstest(
    use_txn_payload_v2_format,
    use_orderless_transactions,
    case(false, false),
    case(true, false),
    case(true, true)
)]
async fn test_get_transaction_by_version(
    use_txn_payload_v2_format: bool,
    use_orderless_transactions: bool,
) {
    let mut context = new_test_context_with_orderless_flags(
        current_function_name!(),
        use_txn_payload_v2_format,
        use_orderless_transactions,
    );
    let account = context.gen_account();
    let txn = context.create_user_account(&account).await;
    context.commit_block(&vec![txn.clone()]).await;

    let txns = context.get("/transactions?start=2&limit=1").await;
    assert_eq!(1, txns.as_array().unwrap().len());

    let resp = context.get("/transactions/by_version/2").await;
    assert_json(resp, txns[0].clone())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[rstest(
    use_txn_payload_v2_format,
    use_orderless_transactions,
    case(false, false),
    case(true, false),
    case(true, true)
)]
async fn test_get_pending_transaction_by_hash(
    use_txn_payload_v2_format: bool,
    use_orderless_transactions: bool,
) {
    let mut context = new_test_context_with_orderless_flags(
        current_function_name!(),
        use_txn_payload_v2_format,
        use_orderless_transactions,
    );
    let account = context.gen_account();
    let txn = context.create_user_account(&account).await;
    let body = bcs::to_bytes(&txn).unwrap();
    let pending_txn = context
        .expect_status_code(202)
        .post_bcs_txn("/transactions", body)
        .await;

    let txn_hash = pending_txn["hash"].as_str().unwrap();

    let mut txn = context
        .get(&format!("/transactions/by_hash/{}", txn_hash))
        .await;

    // The pending txn response from the POST request doesn't the type field,
    // since it is a PendingTransaction, not a Transaction. Remove it from the
    // response from the GET request and confirm it is correct before doing the
    // JSON comparison.
    assert_eq!(
        txn.as_object_mut().unwrap().remove("type").unwrap(),
        "pending_transaction"
    );

    assert_json(txn, pending_txn);

    let not_found = context
        .expect_status_code(404)
        .get("/transactions/by_hash/0xdadfeddcca7cb6396c735e9094c76c6e4e9cb3e3ef814730693aed59bd87b31d")
        .await;
    context.check_golden_output(not_found);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[rstest(
    use_txn_payload_v2_format,
    use_orderless_transactions,
    case(false, false),
    case(true, false),
    case(true, true)
)]
async fn test_wait_transaction_by_hash(
    use_txn_payload_v2_format: bool,
    use_orderless_transactions: bool,
) {
    let mut node_config = NodeConfig::default();
    node_config.api.wait_by_hash_timeout_ms = 2_000;
    let mut context = new_test_context_with_config(
        current_function_name!(),
        node_config,
        use_txn_payload_v2_format,
        use_orderless_transactions,
    );
    let account = context.gen_account();
    let txn = context.create_user_account(&account).await;
    context.commit_block(&vec![txn.clone()]).await;

    let txns = context.get("/transactions?start=2&limit=1").await;
    assert_eq!(1, txns.as_array().unwrap().len());

    let start_time = std::time::Instant::now();
    let resp = context
        .get(&format!(
            "/transactions/wait_by_hash/{}",
            txns[0]["hash"].as_str().unwrap()
        ))
        .await;
    // return immediately
    assert!(start_time.elapsed().as_millis() < 2_000);
    assert_json(resp, txns[0].clone());
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_wait_transaction_by_hash_not_found() {
    let mut node_config = NodeConfig::default();
    node_config.api.wait_by_hash_timeout_ms = 2_000;
    let mut context = new_test_context(current_function_name!());

    let start_time = std::time::Instant::now();
    let resp = context
        .expect_status_code(404)
        .get("/transactions/wait_by_hash/0xdadfeddcca7cb6396c735e9094c76c6e4e9cb3e3ef814730693aed59bd87b31d")
        .await;
    // return immediately
    assert!(start_time.elapsed().as_millis() < 2_000);
    context.check_golden_output(resp);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_wait_transaction_by_invalid_hash() {
    let mut node_config = NodeConfig::default();
    node_config.api.wait_by_hash_timeout_ms = 2_000;
    let mut context =
        new_test_context_with_config(current_function_name!(), node_config, false, false);

    let start_time = std::time::Instant::now();
    let resp = context
        .expect_status_code(400)
        .get("/transactions/wait_by_hash/0x1")
        .await;
    // return immediately
    assert!(start_time.elapsed().as_millis() < 2_000);
    context.check_golden_output(resp);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[rstest(
    use_txn_payload_v2_format,
    use_orderless_transactions,
    case(false, false),
    case(true, false),
    case(true, true)
)]
async fn test_wait_pending_transaction_by_hash(
    use_txn_payload_v2_format: bool,
    use_orderless_transactions: bool,
) {
    let mut node_config = NodeConfig::default();
    node_config.api.wait_by_hash_timeout_ms = 2_000;
    let mut context = new_test_context_with_config(
        current_function_name!(),
        node_config,
        use_txn_payload_v2_format,
        use_orderless_transactions,
    );
    let account = context.gen_account();
    let txn = context.create_user_account(&account).await;
    let body = bcs::to_bytes(&txn).unwrap();
    let pending_txn = context
        .expect_status_code(202)
        .post_bcs_txn("/transactions", body)
        .await;

    let txn_hash = pending_txn["hash"].as_str().unwrap();

    let start_time = std::time::Instant::now();
    let mut txn = context
        .get(&format!("/transactions/wait_by_hash/{}", txn_hash))
        .await;
    // return after waiting for pending to become committed
    assert!(start_time.elapsed().as_millis() > 2_000);

    // The pending txn response from the POST request doesn't the type field,
    // since it is a PendingTransaction, not a Transaction. Remove it from the
    // response from the GET request and confirm it is correct before doing the
    // JSON comparison.
    assert_eq!(
        txn.as_object_mut().unwrap().remove("type").unwrap(),
        "pending_transaction"
    );

    assert_json(txn, pending_txn);

    let not_found = context
        .expect_status_code(404)
        .get("/transactions/by_hash/0xdadfeddcca7cb6396c735e9094c76c6e4e9cb3e3ef814730693aed59bd87b31d")
        .await;
    context.check_golden_output(not_found);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[rstest(
    use_txn_payload_v2_format,
    use_orderless_transactions,
    case(false, false),
    // TODO[Orderless]: `/transactions/encode_submission` API currently encodes transactions using payload v1 format
    // unless a nonce is supplied. So, not testing with payload v2 format here. After the API is updated to use payload v2 format,
    // we can also test with payload v2 format here.
    // case(true, false),
    case(true, true)
)]
async fn test_signing_message_with_entry_function_payload(
    use_txn_payload_v2_format: bool,
    use_orderless_transactions: bool,
) {
    let mut context = new_test_context_with_orderless_flags(
        current_function_name!(),
        use_txn_payload_v2_format,
        use_orderless_transactions,
    );
    let account = context.gen_account();
    let txn = context.create_user_account(&account).await;
    let payload = json!({
        "type": "entry_function_payload",
        "function": "0x1::aptos_account::create_account",
        "type_arguments": [],
        "arguments": [
            account.address().to_hex_literal(), // new_account_address
        ]
    });
    test_signing_message_with_payload(context, txn, payload).await;
}

async fn test_signing_message_with_payload(
    mut context: TestContext,
    txn: SignedTransaction,
    payload: serde_json::Value,
) {
    let initial_ledger_version = context.get_latest_ledger_info().ledger_version;
    let sender = context.root_account().await;
    let mut body = {
        let mut json = json!({
            "sender": sender.address().to_hex_literal(),
            "sequence_number": if context.use_orderless_transactions {
                (u64::MAX).to_string()
            } else {
                sender.sequence_number().to_string()
            },
            "gas_unit_price": txn.gas_unit_price().to_string(),
            "max_gas_amount": txn.max_gas_amount().to_string(),
            "expiration_timestamp_secs": txn.expiration_timestamp_secs().to_string(),
            "payload": payload,
        });

        if context.use_orderless_transactions {
            let nonce = match txn.replay_protector() {
                ReplayProtector::Nonce(n) => n,
                _ => rand::thread_rng().gen(),
            };
            json["replay_protection_nonce"] = json!(nonce.to_string());
        }

        json
    };

    let resp = context
        .post("/transactions/encode_submission", body.clone())
        .await;

    let signing_msg = context
        .api_specific_config
        .unwrap_signing_message_response(resp);
    assert_eq!(
        signing_msg.to_string(),
        format!(
            "0x{}",
            hex::encode(
                txn.clone()
                    .into_raw_transaction()
                    .signing_message()
                    .unwrap()
            )
        )
    );

    let sig = context
        .root_account()
        .await
        .private_key()
        .sign_arbitrary_message(signing_msg.inner());
    let expected_sig = match txn.authenticator() {
        TransactionAuthenticator::Ed25519 {
            public_key: _,
            signature,
        } => signature,
        _ => panic!("expect TransactionAuthenticator::Ed25519"),
    };
    assert_eq!(sig, expected_sig);

    // assert transaction can be submitted into mempool and execute.
    body["signature"] = json!({
        "type": "ed25519_signature",
        "public_key": format!("0x{}", hex::encode(sender.public_key().to_bytes())),
        "signature": format!("0x{}", hex::encode(sig.to_bytes())),
    });

    context
        .expect_status_code(202)
        .post("/transactions", body)
        .await;

    context.commit_mempool_txns(10).await;

    assert_eq!(
        u64::from(context.get_latest_ledger_info().ledger_version),
        u64::from(initial_ledger_version) + 3
    ); // metadata + user txn + state checkpoint
}

async fn match_transactions_with_expected_value(
    context: &TestContext,
    addr: AccountAddress,
    initial_sequence_number: u64,
    initial_ledger_version: u64,
    expected_txns: Value,
    expected_num_txns: usize,
) {
    assert_eq!(expected_txns.as_array().unwrap().len(), expected_num_txns);
    let txns = context
        .get(
            format!(
                "/accounts/{}/transactions?start={}",
                addr, initial_sequence_number,
            )
            .as_str(),
        )
        .await;

    if context.use_orderless_transactions {
        assert_eq!(txns.as_array().unwrap().len(), 0);
    } else {
        assert_eq!(
            txns.as_array().unwrap().len(),
            expected_txns.as_array().unwrap().len()
        );
    }
    let txn_summaries = context
        .get(
            format!(
                "/accounts/{}/transaction_summaries?start_version={}",
                addr,
                initial_ledger_version + 2,
            )
            .as_str(),
        )
        .await;
    assert_eq!(
        txn_summaries.as_array().unwrap().len(),
        expected_txns.as_array().unwrap().len()
    );
    let mut txns_from_summaries = Vec::new();
    for txn_summary in txn_summaries.as_array().unwrap() {
        let txn = context
            .get(
                format!(
                    "/transactions/by_hash/{}",
                    txn_summary["transaction_hash"].as_str().unwrap()
                )
                .as_str(),
            )
            .await;
        txns_from_summaries.push(txn);
    }
    if context.use_orderless_transactions {
        assert_eq!(json!(txns_from_summaries), expected_txns);
        assert_json(json!(txns_from_summaries), expected_txns);
    } else {
        assert_eq!(txns, json!(txns_from_summaries));
        assert_eq!(txns, expected_txns);
        assert_json(txns, expected_txns);
    }
}

async fn test_account_transaction_with_context(mut context: TestContext) {
    let initial_sequence_number = context.root_account().await.sequence_number();
    let initial_ledger_version = u64::from(context.get_latest_ledger_info().ledger_version);
    let account = context.gen_account();
    let txn = context.create_user_account(&account).await;
    context.commit_block(&vec![txn]).await;

    if let Some(indexer_reader) = context.context.indexer_reader.as_ref() {
        indexer_reader
            .wait_for_internal_indexer(initial_ledger_version + 2)
            .unwrap();
    }
    let expected_txns = context
        .get(&format!(
            "/transactions?start={}&limit=1",
            initial_ledger_version + 2
        ))
        .await;
    match_transactions_with_expected_value(
        &context,
        context.root_account().await.address(),
        initial_sequence_number,
        initial_ledger_version,
        expected_txns,
        1,
    )
    .await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[rstest(
    use_txn_payload_v2_format,
    use_orderless_transactions,
    case(false, false),
    case(true, false),
    case(true, true)
)]
async fn test_get_account_transactions(
    use_txn_payload_v2_format: bool,
    use_orderless_transactions: bool,
) {
    let context = new_test_context_with_orderless_flags(
        current_function_name!(),
        use_txn_payload_v2_format,
        use_orderless_transactions,
    );
    test_account_transaction_with_context(context).await;
    let shard_context = new_test_context_with_db_sharding_and_internal_indexer(
        current_function_name!(),
        use_txn_payload_v2_format,
        use_orderless_transactions,
    );
    test_account_transaction_with_context(shard_context).await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[rstest(
    use_txn_payload_v2_format,
    use_orderless_transactions,
    case(false, false),
    case(true, false),
    // Not testing for orderless transactions as orderless transactions don't have sequence number
    // case(true, true)
)]
async fn test_get_account_transactions_filter_transactions_by_start_sequence_number(
    use_txn_payload_v2_format: bool,
    use_orderless_transactions: bool,
) {
    let mut context = new_test_context_with_orderless_flags(
        current_function_name!(),
        use_txn_payload_v2_format,
        use_orderless_transactions,
    );
    let initial_sequence_number = context.root_account().await.sequence_number();
    let account = context.gen_account();
    let txn = context.create_user_account(&account).await;
    context.commit_block(&vec![txn.clone()]).await;

    let txns = context
        .get(
            format!(
                "/accounts/{}/transactions?start={}",
                context.root_account().await.address(),
                initial_sequence_number + 1
            )
            .as_str(),
        )
        .await;
    assert_json(txns, json!([]));
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[rstest(
    use_txn_payload_v2_format,
    use_orderless_transactions,
    case(false, false),
    case(true, false),
    case(true, true)
)]
async fn test_get_account_transactions_filter_transactions_by_start_sequence_number_is_too_large(
    use_txn_payload_v2_format: bool,
    use_orderless_transactions: bool,
) {
    let mut context = new_test_context_with_orderless_flags(
        current_function_name!(),
        use_txn_payload_v2_format,
        use_orderless_transactions,
    );
    let account = context.gen_account();
    let txn = context.create_user_account(&account).await;
    context.commit_block(&vec![txn]).await;

    let txns = context
        .get(
            format!(
                "/accounts/{}/transactions?start=1000",
                context.root_account().await.address()
            )
            .as_str(),
        )
        .await;
    assert_json(txns, json!([]));
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[rstest(
    use_txn_payload_v2_format,
    use_orderless_transactions,
    case(false, false),
    case(true, false),
    case(true, true)
)]
async fn test_get_account_transactions_filter_transactions_by_limit(
    use_txn_payload_v2_format: bool,
    use_orderless_transactions: bool,
) {
    let mut context = new_test_context_with_orderless_flags(
        current_function_name!(),
        use_txn_payload_v2_format,
        use_orderless_transactions,
    );
    let mut root_account = context.root_account().await;
    let initial_sequence_number = root_account.sequence_number();
    let initial_ledger_version = u64::from(context.get_latest_ledger_info().ledger_version);
    let account1 = context.gen_account();
    let txn1 = context.create_user_account_by(&mut root_account, &account1);
    let account2 = context.gen_account();
    let txn2 = context.create_user_account_by(&mut root_account, &account2);
    context.commit_block(&vec![txn1, txn2]).await;

    if !context.use_orderless_transactions {
        let txns = context
            .get(
                format!(
                    "/accounts/{}/transactions?start={}&limit=1",
                    context.root_account().await.address(),
                    initial_sequence_number,
                )
                .as_str(),
            )
            .await;
        assert_eq!(txns.as_array().unwrap().len(), 1);

        let txns = context
            .get(
                format!(
                    "/accounts/{}/transactions?start={}&limit=2",
                    context.root_account().await.address(),
                    initial_sequence_number,
                )
                .as_str(),
            )
            .await;
        assert_eq!(txns.as_array().unwrap().len(), 2);
    }

    let expected_txns = context
        .get(&format!(
            "/transactions?start={}&limit=2",
            initial_ledger_version + 2
        ))
        .await;
    match_transactions_with_expected_value(
        &context,
        context.root_account().await.address(),
        initial_sequence_number,
        initial_ledger_version,
        expected_txns,
        2,
    )
    .await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[rstest(
    case_name,
    use_txn_payload_v2_format,
    use_orderless_transactions,
    case("", false, false),
    case("_payload_v2", true, false),
    case("_orderless", true, true)
)]
async fn test_get_txn_execute_failed_by_invalid_script_payload_bytecode(
    case_name: &str,
    use_txn_payload_v2_format: bool,
    use_orderless_transactions: bool,
) {
    let mut context = new_test_context_with_orderless_flags(
        current_function_name!() + case_name,
        use_txn_payload_v2_format,
        use_orderless_transactions,
    );
    let root_account = context.root_account().await;
    let invalid_bytecode = hex::decode("a11ceb0b030000").unwrap();
    let txn = root_account.sign_with_transaction_builder(
        context
            .transaction_factory()
            .script(Script::new(invalid_bytecode, vec![], vec![]))
            .expiration_timestamp_secs(context.get_expiration_time())
            .upgrade_payload(
                &mut context.rng,
                context.use_txn_payload_v2_format,
                context.use_orderless_transactions,
            ),
    );
    test_transaction_vm_status(context, txn, false).await
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[rstest(
    case_name,
    use_txn_payload_v2_format,
    use_orderless_transactions,
    case("", false, false),
    case("_payload_v2", true, false),
    case("_orderless", true, true)
)]
async fn test_get_txn_execute_failed_by_invalid_entry_function_address(
    case_name: &str,
    use_txn_payload_v2_format: bool,
    use_orderless_transactions: bool,
) {
    let context = new_test_context_with_orderless_flags(
        current_function_name!() + case_name,
        use_txn_payload_v2_format,
        use_orderless_transactions,
    );
    let account = context.root_account().await;
    test_get_txn_execute_failed_by_invalid_entry_function(
        context,
        account,
        "0x1222",
        "Coin",
        "transfer",
        vec![AptosCoinType::type_tag()],
        vec![
            bcs::to_bytes(&AccountAddress::from_hex_literal("0xdd").unwrap()).unwrap(),
            bcs::to_bytes(&1u64).unwrap(),
        ],
    )
    .await
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[rstest(
    case_name,
    use_txn_payload_v2_format,
    use_orderless_transactions,
    case("", false, false),
    case("_payload_v2", true, false),
    case("_orderless", true, true)
)]
async fn test_get_txn_execute_failed_by_invalid_entry_function_module_name(
    case_name: &str,
    use_txn_payload_v2_format: bool,
    use_orderless_transactions: bool,
) {
    let context = new_test_context_with_orderless_flags(
        current_function_name!() + case_name,
        use_txn_payload_v2_format,
        use_orderless_transactions,
    );
    let account = context.root_account().await;
    test_get_txn_execute_failed_by_invalid_entry_function(
        context,
        account,
        "0x1",
        "CoinInvalid",
        "transfer",
        vec![AptosCoinType::type_tag()],
        vec![
            bcs::to_bytes(&AccountAddress::from_hex_literal("0xdd").unwrap()).unwrap(),
            bcs::to_bytes(&1u64).unwrap(),
        ],
    )
    .await
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[rstest(
    case_name,
    use_txn_payload_v2_format,
    use_orderless_transactions,
    case("", false, false),
    case("_payload_v2", true, false),
    case("_orderless", true, true)
)]
async fn test_get_txn_execute_failed_by_invalid_entry_function_name(
    case_name: &str,
    use_txn_payload_v2_format: bool,
    use_orderless_transactions: bool,
) {
    let context = new_test_context_with_orderless_flags(
        current_function_name!() + case_name,
        use_txn_payload_v2_format,
        use_orderless_transactions,
    );
    let account = context.root_account().await;
    test_get_txn_execute_failed_by_invalid_entry_function(
        context,
        account,
        "0x1",
        "Coin",
        "transfer_invalid",
        vec![AptosCoinType::type_tag()],
        vec![
            bcs::to_bytes(&AccountAddress::from_hex_literal("0xdd").unwrap()).unwrap(),
            bcs::to_bytes(&1u64).unwrap(),
        ],
    )
    .await
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[rstest(
    case_name,
    use_txn_payload_v2_format,
    use_orderless_transactions,
    case("", false, false),
    case("_payload_v2", true, false),
    case("_orderless", true, true)
)]
async fn test_get_txn_execute_failed_by_invalid_entry_function_arguments(
    case_name: &str,
    use_txn_payload_v2_format: bool,
    use_orderless_transactions: bool,
) {
    let context = new_test_context_with_orderless_flags(
        current_function_name!() + case_name,
        use_txn_payload_v2_format,
        use_orderless_transactions,
    );
    let account = context.root_account().await;
    test_get_txn_execute_failed_by_invalid_entry_function(
        context,
        account,
        "0x1",
        "Coin",
        "transfer",
        vec![AptosCoinType::type_tag()],
        vec![
            bcs::to_bytes(&AccountAddress::from_hex_literal("0xdd").unwrap()).unwrap(),
            bcs::to_bytes(&1u8).unwrap(), // invalid type
        ],
    )
    .await
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[rstest(
    case_name,
    use_txn_payload_v2_format,
    use_orderless_transactions,
    case("", false, false),
    case("_payload_v2", true, false),
    case("_orderless", true, true)
)]
async fn test_get_txn_execute_failed_by_missing_entry_function_arguments(
    case_name: &str,
    use_txn_payload_v2_format: bool,
    use_orderless_transactions: bool,
) {
    let context = new_test_context_with_orderless_flags(
        current_function_name!() + case_name,
        use_txn_payload_v2_format,
        use_orderless_transactions,
    );
    let account = context.root_account().await;
    test_get_txn_execute_failed_by_invalid_entry_function(
        context,
        account,
        "0x1",
        "Coin",
        "transfer",
        vec![AptosCoinType::type_tag()],
        vec![
            bcs::to_bytes(&AccountAddress::from_hex_literal("0xdd").unwrap()).unwrap(),
            // missing arguments
        ],
    )
    .await
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[rstest(
    case_name,
    use_txn_payload_v2_format,
    use_orderless_transactions,
    case("", false, false),
    case("_payload_v2", true, false),
    case("_orderless", true, true)
)]
async fn test_get_txn_execute_failed_by_entry_function_validation(
    case_name: &str,
    use_txn_payload_v2_format: bool,
    use_orderless_transactions: bool,
) {
    let mut context = new_test_context_with_orderless_flags(
        current_function_name!() + case_name,
        use_txn_payload_v2_format,
        use_orderless_transactions,
    );
    let account = context.gen_account();
    let txn1 = context.create_user_account(&account).await;
    context.commit_block(&vec![txn1]).await;

    test_get_txn_execute_failed_by_invalid_entry_function(
        context,
        account,
        "0x1",
        "Coin",
        "transfer",
        vec![AptosCoinType::type_tag()],
        vec![
            bcs::to_bytes(&AccountAddress::from_hex_literal("0xdd").unwrap()).unwrap(),
            bcs::to_bytes(&123u64).unwrap(), // exceed limit, account balance is 0.
        ],
    )
    .await
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[rstest(
    use_txn_payload_v2_format,
    use_orderless_transactions,
    case(false, false),
    case(true, false),
    case(true, true)
)]
async fn test_get_txn_execute_failed_by_entry_function_invalid_module_name(
    use_txn_payload_v2_format: bool,
    use_orderless_transactions: bool,
) {
    let mut context = new_test_context_with_orderless_flags(
        current_function_name!(),
        use_txn_payload_v2_format,
        use_orderless_transactions,
    );
    let account = context.gen_account();
    let txn1 = context.create_user_account(&account).await;
    context.commit_block(&vec![txn1]).await;

    test_submit_entry_function_api_validation(
        context,
        account,
        "0x1",
        "coin",
        "transfer::what::what",
        vec![AptosCoinType::type_tag()],
        vec![
            bcs::to_bytes(&AccountAddress::from_hex_literal("0xdd").unwrap()).unwrap(),
            bcs::to_bytes(&123u64).unwrap(), // exceed limit, account balance is 0.
        ],
    )
    .await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[rstest(
    use_txn_payload_v2_format,
    use_orderless_transactions,
    case(false, false),
    case(true, false),
    case(true, true)
)]
async fn test_get_txn_execute_failed_by_entry_function_invalid_function_name(
    use_txn_payload_v2_format: bool,
    use_orderless_transactions: bool,
) {
    let mut context = new_test_context_with_orderless_flags(
        current_function_name!(),
        use_txn_payload_v2_format,
        use_orderless_transactions,
    );
    let account = context.gen_account();
    let txn1 = context.create_user_account(&account).await;
    context.commit_block(&vec![txn1]).await;

    test_submit_entry_function_api_validation(
        context,
        account,
        "0x1",
        "coin::coin",
        "transfer",
        vec![AptosCoinType::type_tag()],
        vec![
            bcs::to_bytes(&AccountAddress::from_hex_literal("0xdd").unwrap()).unwrap(),
            bcs::to_bytes(&123u64).unwrap(), // exceed limit, account balance is 0.
        ],
    )
    .await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[rstest(
    use_txn_payload_v2_format,
    use_orderless_transactions,
    case(false, false),
    case(true, false),
    case(true, true)
)]
async fn test_get_txn_execute_failed_by_entry_function_execution_failure(
    use_txn_payload_v2_format: bool,
    use_orderless_transactions: bool,
) {
    let mut context = new_test_context_with_orderless_flags(
        current_function_name!(),
        use_txn_payload_v2_format,
        use_orderless_transactions,
    );
    let mut admin = context.create_account().await;

    let named_addresses = vec![("entry_func_fail".to_string(), admin.address())];

    let named_addresses_clone = named_addresses.clone();
    let txn = futures::executor::block_on(async move {
        let path = PathBuf::from(std::env!("CARGO_MANIFEST_DIR")).join("test-context/move");
        TestContext::build_package(path, named_addresses_clone)
    });
    let txn = context.publish_package(&mut admin, txn).await;

    let resp = context
        .get(
            format!(
                "/transactions/by_hash/{}",
                txn.committed_hash().to_hex_literal()
            )
            .as_str(),
        )
        .await;

    assert!(!resp["success"].as_bool().unwrap(), "{}", pretty(&resp));
}

#[ignore] // re-enable after cleaning compiled code
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_get_txn_execute_failed_by_script_execution_failure() {
    let mut context = new_test_context(current_function_name!());

    // script {
    //     fun main() {
    //         1/0;
    //     }
    // }
    let script =
        hex::decode("a11ceb0b030000000105000100000000050601000000000000000600000000000000001a0102")
            .unwrap();
    let root_account = context.root_account().await;
    let txn = root_account.sign_with_transaction_builder(
        context
            .transaction_factory()
            .script(Script::new(script, vec![], vec![]))
            .expiration_timestamp_secs(context.get_expiration_time())
            .upgrade_payload(
                &mut context.rng,
                context.use_txn_payload_v2_format,
                context.use_orderless_transactions,
            ),
    );

    test_transaction_vm_status(context, txn, false).await
}

async fn test_submit_entry_function_api_validation(
    mut context: TestContext,
    account: LocalAccount,
    address: &str,
    module_id: &str,
    func: &str,
    ty_args: Vec<TypeTag>,
    args: Vec<Vec<u8>>,
) {
    // This is a way to get around the Identifier checks!
    #[derive(serde::Serialize)]
    struct HackStruct(pub Box<str>);

    // Identifiers check when you call new, but they don't check when you deserialize, surprise!
    let module_id: Identifier =
        serde_json::from_str(&serde_json::to_string(&HackStruct(module_id.into())).unwrap())
            .unwrap();
    let func: Identifier =
        serde_json::from_str(&serde_json::to_string(&HackStruct(func.into())).unwrap()).unwrap();

    let txn = account.sign_with_transaction_builder(
        context
            .transaction_factory()
            .entry_function(EntryFunction::new(
                ModuleId::new(
                    AccountAddress::from_hex_literal(address).unwrap(),
                    module_id,
                ),
                func,
                ty_args,
                args,
            ))
            .expiration_timestamp_secs(context.get_expiration_time())
            .upgrade_payload(
                &mut context.rng,
                context.use_txn_payload_v2_format,
                context.use_orderless_transactions,
            ),
    );

    let body = bcs::to_bytes(&txn).unwrap();
    // we don't validate transaction payload when submit txn into mempool.
    let resp = context
        .expect_status_code(400)
        .post_bcs_txn("/transactions", body)
        .await;
    context.check_golden_output(resp);
}

async fn test_get_txn_execute_failed_by_invalid_entry_function(
    mut context: TestContext,
    account: LocalAccount,
    address: &str,
    module_id: &str,
    func: &str,
    ty_args: Vec<TypeTag>,
    args: Vec<Vec<u8>>,
) {
    let txn = account.sign_with_transaction_builder(
        context
            .transaction_factory()
            .entry_function(EntryFunction::new(
                ModuleId::new(
                    AccountAddress::from_hex_literal(address).unwrap(),
                    Identifier::new(module_id).unwrap(),
                ),
                Identifier::new(func).unwrap(),
                ty_args,
                args,
            ))
            .expiration_timestamp_secs(context.get_expiration_time())
            .upgrade_payload(
                &mut context.rng,
                context.use_txn_payload_v2_format,
                context.use_orderless_transactions,
            ),
    );

    test_transaction_vm_status(context, txn, false).await
}

async fn test_transaction_vm_status(
    mut context: TestContext,
    txn: SignedTransaction,
    success: bool,
) {
    let body = bcs::to_bytes(&txn).unwrap();
    // we don't validate transaction payload when submit txn into mempool.
    context
        .expect_status_code(202)
        .post_bcs_txn("/transactions", body)
        .await;
    context.commit_mempool_txns(1).await;
    let resp = context
        .get(
            format!(
                "/transactions/by_hash/{}",
                txn.committed_hash().to_hex_literal()
            )
            .as_str(),
        )
        .await;
    assert_eq!(
        resp["success"].as_bool().unwrap(),
        success,
        "{}",
        pretty(&resp)
    );
    context.check_golden_output(resp);
}

#[ignore]
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_submit_transaction_rejects_payload_too_large_bcs_txn_body() {
    let mut context = new_test_context(current_function_name!());

    let resp = context
        .expect_status_code(413)
        .post_bcs_txn(
            "/transactions",
            gen_string(context.context.content_length_limit() + 1).as_bytes(),
        )
        .await;
    context.check_golden_output(resp);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
#[ignore]
async fn test_submit_transaction_rejects_payload_too_large_json_body() {
    let mut context = new_test_context(current_function_name!());

    let resp = context
        .expect_status_code(413)
        .post(
            "/transactions",
            json!({
                "data": gen_string(context.context.content_length_limit()+2).as_bytes(),
            }),
        )
        .await;
    context.check_golden_output(resp);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[rstest(
    use_txn_payload_v2_format,
    use_orderless_transactions,
    case(false, false),
    case(true, false),
    case(true, true)
)]
async fn test_submit_transaction_rejects_invalid_content_type(
    use_txn_payload_v2_format: bool,
    use_orderless_transactions: bool,
) {
    let mut context = new_test_context_with_orderless_flags(
        current_function_name!(),
        use_txn_payload_v2_format,
        use_orderless_transactions,
    );
    let req = warp::test::request()
        .header("content-type", "invalid")
        .method("POST")
        .body("text")
        .path(&build_path(""));

    let resp = context.expect_status_code(415).execute(req).await;
    context.check_golden_output(resp);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[rstest(
    use_txn_payload_v2_format,
    use_orderless_transactions,
    case(false, false),
    case(true, false),
    case(true, true)
)]
async fn test_submit_transaction_rejects_invalid_json(
    use_txn_payload_v2_format: bool,
    use_orderless_transactions: bool,
) {
    let mut context = new_test_context_with_orderless_flags(
        current_function_name!(),
        use_txn_payload_v2_format,
        use_orderless_transactions,
    );
    let req = warp::test::request()
        .header("content-type", "application/json")
        .method("POST")
        .body("invalid json")
        .path(&build_path(""));

    let resp = context.expect_status_code(400).execute(req).await;
    context.check_golden_output(resp);
}

#[ignore]
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_create_signing_message_rejects_payload_too_large_json_body() {
    let mut context = new_test_context(current_function_name!());

    let resp = context
        .expect_status_code(413)
        .post(
            "/transactions/encode_submission",
            json!({
                "data": gen_string(context.context.content_length_limit()+1).as_bytes(),
            }),
        )
        .await;
    context.check_golden_output(resp);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[rstest(
    use_txn_payload_v2_format,
    use_orderless_transactions,
    case(false, false),
    case(true, false),
    case(true, true)
)]
async fn test_create_signing_message_rejects_invalid_content_type(
    use_txn_payload_v2_format: bool,
    use_orderless_transactions: bool,
) {
    let mut context = new_test_context_with_orderless_flags(
        current_function_name!(),
        use_txn_payload_v2_format,
        use_orderless_transactions,
    );
    let req = warp::test::request()
        .header("content-type", "invalid")
        .method("POST")
        .body("text")
        .path(&build_path("/encode_submission"));

    let resp = context.expect_status_code(415).execute(req).await;
    context.check_golden_output(resp);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[rstest(
    use_txn_payload_v2_format,
    use_orderless_transactions,
    case(false, false),
    case(true, false),
    case(true, true)
)]
async fn test_create_signing_message_rejects_invalid_json(
    use_txn_payload_v2_format: bool,
    use_orderless_transactions: bool,
) {
    let mut context = new_test_context_with_orderless_flags(
        current_function_name!(),
        use_txn_payload_v2_format,
        use_orderless_transactions,
    );
    let req = warp::test::request()
        .header("content-type", "application/json")
        .method("POST")
        .body("invalid json")
        .path(&build_path("/encode_submission"));

    let resp = context.expect_status_code(400).execute(req).await;
    context.check_golden_output(resp);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[rstest(
    use_txn_payload_v2_format,
    use_orderless_transactions,
    case(false, false),
    case(true, false),
    case(true, true)
)]
async fn test_create_signing_message_rejects_no_content_length_request(
    use_txn_payload_v2_format: bool,
    use_orderless_transactions: bool,
) {
    let mut context = new_test_context_with_orderless_flags(
        current_function_name!(),
        use_txn_payload_v2_format,
        use_orderless_transactions,
    );
    let req = warp::test::request()
        .header("content-type", "application/json")
        .method("POST")
        .path(&build_path("/encode_submission"));

    let resp = context.expect_status_code(411).execute(req).await;
    context.check_golden_output(resp);
}

// Note: in tests, the min gas unit price is 0
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[rstest(
    use_txn_payload_v2_format,
    use_orderless_transactions,
    case(false, false),
    case(true, false),
    case(true, true)
)]
async fn test_gas_estimation_empty(
    use_txn_payload_v2_format: bool,
    use_orderless_transactions: bool,
) {
    let mut node_config = NodeConfig::default();
    node_config.api.gas_estimation.enabled = true;
    let mut context = new_test_context_with_config(
        current_function_name!(),
        node_config,
        use_txn_payload_v2_format,
        use_orderless_transactions,
    );

    let resp = context.get("/estimate_gas_price").await;
    assert!(context.last_updated_gas_schedule().is_some());
    context.check_golden_output(resp);
}

async fn fill_block(
    block: &mut Vec<SignedTransaction>,
    ctx: &mut TestContext,
    creator: &mut LocalAccount,
) {
    let owner = &mut ctx.gen_account();
    for _i in 0..(500 - block.len()) {
        let txn = ctx.account_transfer(creator, owner, 1);
        block.push(txn);
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[rstest(
    use_txn_payload_v2_format,
    use_orderless_transactions,
    case(false, false),
    case(true, false),
    case(true, true)
)]
async fn test_gas_estimation_ten_blocks(
    use_txn_payload_v2_format: bool,
    use_orderless_transactions: bool,
) {
    let mut node_config = NodeConfig::default();
    node_config.api.gas_estimation.enabled = true;
    let mut context = new_test_context_with_config(
        current_function_name!(),
        node_config,
        use_txn_payload_v2_format,
        use_orderless_transactions,
    );

    let ctx = &mut context;
    let creator = &mut ctx.gen_account();
    let mint_txn = ctx.mint_user_account(creator).await;

    // Include the mint txn in the first block
    let mut block = vec![mint_txn];
    // First block is ignored in gas estimate, so make 11
    for _i in 0..11 {
        fill_block(&mut block, ctx, creator).await;
        ctx.commit_block(&block).await;
        block.clear();
    }

    let resp = context.get("/estimate_gas_price").await;
    // multiple times, to exercise cache
    for _i in 0..2 {
        let cached = context.get("/estimate_gas_price").await;
        assert_eq!(resp, cached);
    }
    context.check_golden_output(resp);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[rstest(
    use_txn_payload_v2_format,
    use_orderless_transactions,
    case(false, false),
    case(true, false),
    case(true, true)
)]
async fn test_gas_estimation_ten_empty_blocks(
    use_txn_payload_v2_format: bool,
    use_orderless_transactions: bool,
) {
    let mut node_config = NodeConfig::default();
    node_config.api.gas_estimation.enabled = true;
    let mut context = new_test_context_with_config(
        current_function_name!(),
        node_config,
        use_txn_payload_v2_format,
        use_orderless_transactions,
    );

    let ctx = &mut context;
    // First block is ignored in gas estimate, so make 11
    for _i in 0..11 {
        ctx.commit_block(&[]).await;
    }

    let resp = context.get("/estimate_gas_price").await;
    // multiple times, to exercise cache
    for _i in 0..2 {
        let cached = context.get("/estimate_gas_price").await;
        assert_eq!(resp, cached);
    }
    context.check_golden_output(resp);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[rstest(
    use_txn_payload_v2_format,
    use_orderless_transactions,
    case(false, false),
    case(true, false),
    case(true, true)
)]
async fn test_gas_estimation_cache(
    use_txn_payload_v2_format: bool,
    use_orderless_transactions: bool,
) {
    let mut node_config = NodeConfig::default();
    node_config.api.gas_estimation.enabled = true;
    // Sets max cache size to 10
    let max_block_history = 10;
    node_config.api.gas_estimation.low_block_history = max_block_history;
    node_config.api.gas_estimation.market_block_history = max_block_history;
    node_config.api.gas_estimation.aggressive_block_history = max_block_history;
    let sleep_duration =
        Duration::from_millis(node_config.api.gas_estimation.cache_expiration_ms * 2);
    let mut context = new_test_context_with_config(
        current_function_name!(),
        node_config,
        use_txn_payload_v2_format,
        use_orderless_transactions,
    );

    let ctx = &mut context;
    // First block is ignored in gas estimate, so expect 4 entries
    for _i in 0..5 {
        ctx.commit_block(&[]).await;
    }
    ctx.get("/estimate_gas_price").await;
    assert_eq!(ctx.last_updated_gas_estimation_cache_size(), 4);
    // Wait for cache to expire
    sleep(sleep_duration).await;

    // Expect max of 10 entries
    for _i in 0..8 {
        ctx.commit_block(&[]).await;
    }
    ctx.get("/estimate_gas_price").await;
    assert_eq!(
        ctx.last_updated_gas_estimation_cache_size(),
        max_block_history
    );
    // Wait for cache to expire
    sleep(sleep_duration).await;
    ctx.get("/estimate_gas_price").await;
    assert_eq!(
        ctx.last_updated_gas_estimation_cache_size(),
        max_block_history
    );

    // Expect max of 10 entries
    for _i in 0..8 {
        ctx.commit_block(&[]).await;
    }
    ctx.get("/estimate_gas_price").await;
    assert_eq!(
        ctx.last_updated_gas_estimation_cache_size(),
        max_block_history
    );
    // Wait for cache to expire
    sleep(sleep_duration).await;
    ctx.get("/estimate_gas_price").await;
    assert_eq!(
        ctx.last_updated_gas_estimation_cache_size(),
        max_block_history
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
async fn test_gas_estimation_disabled(
    use_txn_payload_v2_format: bool,
    use_orderless_transactions: bool,
) {
    let mut node_config = NodeConfig::default();
    node_config.api.gas_estimation.enabled = false;
    let mut context = new_test_context_with_config(
        current_function_name!(),
        node_config,
        use_txn_payload_v2_format,
        use_orderless_transactions,
    );

    let ctx = &mut context;
    let creator = &mut ctx.gen_account();
    let mint_txn = ctx.mint_user_account(creator).await;

    // Include the mint txn in the first block
    let mut block = vec![mint_txn];
    // First block is ignored in gas estimate, so make 11
    for _i in 0..11 {
        fill_block(&mut block, ctx, creator).await;
        ctx.commit_block(&block).await;
        block.clear();
    }

    // It's disabled, so we always expect the default, despite the blocks being filled above
    let resp = context.get("/estimate_gas_price").await;
    for _i in 0..2 {
        let cached = context.get("/estimate_gas_price").await;
        assert_eq!(resp, cached);
    }
    context.check_golden_output(resp);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[rstest(
    use_txn_payload_v2_format,
    use_orderless_transactions,
    case(false, false),
    case(true, false),
    case(true, true)
)]
async fn test_gas_estimation_static_override(
    use_txn_payload_v2_format: bool,
    use_orderless_transactions: bool,
) {
    let mut node_config = NodeConfig::default();
    node_config.api.gas_estimation.enabled = true;
    node_config.api.gas_estimation.static_override = Some(GasEstimationStaticOverride {
        low: 100,
        market: 200,
        aggressive: 300,
    });
    let mut context = new_test_context_with_config(
        current_function_name!(),
        node_config,
        use_txn_payload_v2_format,
        use_orderless_transactions,
    );

    let ctx = &mut context;
    let creator = &mut ctx.gen_account();
    let mint_txn = ctx.mint_user_account(creator).await;

    // Include the mint txn in the first block
    let mut block = vec![mint_txn];
    // First block is ignored in gas estimate, so make 11
    for _i in 0..11 {
        fill_block(&mut block, ctx, creator).await;
        ctx.commit_block(&block).await;
        block.clear();
    }

    // It's disabled, so we always expect the default, despite the blocks being filled above
    let resp = context.get("/estimate_gas_price").await;
    for _i in 0..2 {
        let cached = context.get("/estimate_gas_price").await;
        assert_eq!(resp, cached);
    }
    context.check_golden_output(resp);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[rstest(
    use_txn_payload_v2_format,
    use_orderless_transactions,
    case(false, false),
    case(true, false),
    case(true, true)
)]
async fn test_simulation_failure_error_message(
    use_txn_payload_v2_format: bool,
    use_orderless_transactions: bool,
) {
    let mut context = new_test_context_with_orderless_flags(
        current_function_name!(),
        use_txn_payload_v2_format,
        use_orderless_transactions,
    );
    let admin0 = context.root_account().await;

    // script {
    //     fun main() {
    //         1/0;
    //     }
    // }

    let output = context.simulate_transaction(&admin0, json!({
        "type": "script_payload",
        "code": {
            "bytecode": "a11ceb0b030000000105000100000000050601000000000000000600000000000000001a0102",
        },
        "type_arguments": [],
        "arguments": [],
    }), 200).await;
    let resp = &output.as_array().unwrap()[0];

    assert!(!resp["success"].as_bool().unwrap());
    assert!(resp["vm_status"]
        .as_str()
        .unwrap()
        .contains("Division by zero"));
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[rstest(
    case_name,
    use_txn_payload_v2_format,
    use_orderless_transactions,
    case("", false, false),
    case("_payload_v2", true, false),
    case("_orderless", true, true)
)]
async fn test_simulation_failure_with_move_abort_error_rendering(
    case_name: &str,
    use_txn_payload_v2_format: bool,
    use_orderless_transactions: bool,
) {
    let mut context = new_test_context_with_orderless_flags(
        current_function_name!() + case_name,
        use_txn_payload_v2_format,
        use_orderless_transactions,
    );
    let account = context.create_account().await;
    let raw_txn = context
        .transaction_factory()
        .entry_function(EntryFunction::new(
            ModuleId::new(
                AccountAddress::from_hex_literal("0x1").unwrap(),
                Identifier::new("aptos_account").unwrap(),
            ),
            Identifier::new("transfer").unwrap(),
            vec![],
            vec![
                bcs::to_bytes(&AccountAddress::from_hex_literal("0x1").unwrap()).unwrap(),
                bcs::to_bytes(&999999999999999999u64).unwrap(),
            ],
        ))
        .sender(account.address())
        .sequence_number(account.sequence_number())
        .expiration_timestamp_secs(context.get_expiration_time())
        .upgrade_payload(
            &mut context.rng,
            context.use_txn_payload_v2_format,
            context.use_orderless_transactions,
        )
        .build();
    let invalid_key = AccountKey::generate(&mut context.rng());

    let txn = raw_txn
        .sign(invalid_key.private_key(), account.public_key().clone())
        .unwrap()
        .into_inner();
    let body = bcs::to_bytes(&txn).unwrap();
    let resp = context
        .expect_status_code(200)
        .post_bcs_txn("/transactions/simulate", body)
        .await;
    context.check_golden_output(resp);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[rstest(
    case_name,
    use_txn_payload_v2_format,
    use_orderless_transactions,
    case("", false, false),
    case("_payload_v2", true, false),
    case("_orderless", true, true)
)]
async fn test_simulation_failure_with_detail_error(
    case_name: &str,
    use_txn_payload_v2_format: bool,
    use_orderless_transactions: bool,
) {
    let mut context = new_test_context_with_orderless_flags(
        current_function_name!() + case_name,
        use_txn_payload_v2_format,
        use_orderless_transactions,
    );
    let account = context.root_account().await;
    let raw_txn = context
        .transaction_factory()
        .entry_function(EntryFunction::new(
            ModuleId::new(
                AccountAddress::from_hex_literal("0x1").unwrap(),
                Identifier::new("MemeCoin").unwrap(),
            ),
            Identifier::new("transfer").unwrap(),
            vec![AptosCoinType::type_tag()],
            vec![
                bcs::to_bytes(&AccountAddress::from_hex_literal("0xdd").unwrap()).unwrap(),
                bcs::to_bytes(&1u64).unwrap(),
            ],
        ))
        .sender(account.address())
        .sequence_number(account.sequence_number())
        .expiration_timestamp_secs(context.get_expiration_time())
        .upgrade_payload(
            &mut context.rng,
            context.use_txn_payload_v2_format,
            context.use_orderless_transactions,
        )
        .build();
    let invalid_key = AccountKey::generate(&mut context.rng());
    let txn = raw_txn
        .sign(invalid_key.private_key(), account.public_key().clone())
        .unwrap()
        .into_inner();
    let body = bcs::to_bytes(&txn).unwrap();
    let resp = context
        .expect_status_code(200)
        .post_bcs_txn("/transactions/simulate", body)
        .await;
    context.check_golden_output(resp);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[rstest(
    use_txn_payload_v2_format,
    use_orderless_transactions,
    case(false, false),
    case(true, false),
    case(true, true)
)]
async fn test_runtime_error_message_in_interpreter(
    use_txn_payload_v2_format: bool,
    use_orderless_transactions: bool,
) {
    let mut context = new_test_context_with_orderless_flags(
        current_function_name!(),
        use_txn_payload_v2_format,
        use_orderless_transactions,
    );
    let account = context.root_account().await;

    let named_addresses = vec![("addr".to_string(), account.address())];
    let path =
        PathBuf::from(std::env!("CARGO_MANIFEST_DIR")).join("src/tests/move/pack_exceed_limit");
    let payload = TestContext::build_package(path, named_addresses);
    let txn = account.sign_with_transaction_builder(
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
    let body = bcs::to_bytes(&txn).unwrap();
    let resp = context
        .expect_status_code(202)
        .post_bcs_txn("/transactions", body)
        .await;

    let mut payload = json!({
        "sender": resp["sender"],
        "sequence_number": resp["sequence_number"],
        "max_gas_amount": resp["max_gas_amount"],
        "gas_unit_price": resp["gas_unit_price"],
        "expiration_timestamp_secs": resp["expiration_timestamp_secs"],
        "payload": resp["payload"],
        "signature": {
            "type": resp["signature"]["type"],
            "public_key": resp["signature"]["public_key"],
            "signature": Ed25519Signature::dummy_signature().to_string(),
        }
    });

    if context.use_orderless_transactions {
        let nonce = rand::thread_rng().gen::<u64>();
        payload["replay_protection_nonce"] = json!(nonce.to_string());
    }
    let resp = context
        .expect_status_code(200)
        .post("/transactions/simulate", payload)
        .await;

    assert!(!resp[0]["success"].as_bool().unwrap());
    let vm_status = resp[0]["vm_status"].as_str().unwrap();
    assert!(vm_status.contains("VERIFICATION_ERROR"));
    assert!(vm_status
        .contains("Number of type nodes when constructing type layout exceeded the maximum"));
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[rstest(
    use_txn_payload_v2_format,
    use_orderless_transactions,
    case(false, false),
    case(true, false),
    case(true, true)
)]
async fn test_simulation_filter_deny(
    use_txn_payload_v2_format: bool,
    use_orderless_transactions: bool,
) {
    let mut node_config = NodeConfig::default();

    // Blocklist the balance function.
    let mut filter = node_config.api.simulation_filter.clone();
    filter = filter.add_deny_all();
    node_config.api.simulation_filter = filter;

    let mut context = new_test_context_with_config(
        current_function_name!(),
        node_config,
        use_txn_payload_v2_format,
        use_orderless_transactions,
    );

    let admin0 = context.root_account().await;

    let resp = context.simulate_transaction(&admin0, json!({
        "type": "script_payload",
        "code": {
            "bytecode": "a11ceb0b030000000105000100000000050601000000000000000600000000000000001a0102",
        },
        "type_arguments": [],
        "arguments": [],
    }), 403).await;

    context.check_golden_output(resp);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[rstest(
    use_txn_payload_v2_format,
    use_orderless_transactions,
    case(false, false),
    case(true, false),
    case(true, true)
)]
async fn test_simulation_filter_allow_sender(
    use_txn_payload_v2_format: bool,
    use_orderless_transactions: bool,
) {
    let mut node_config = NodeConfig::default();

    // Allow the root sender only.
    let mut filter = node_config.api.simulation_filter.clone();
    filter = filter.add_allow_sender(aptos_test_root_address());
    filter = filter.add_deny_all();
    node_config.api.simulation_filter = filter;

    let mut context = new_test_context_with_config(
        current_function_name!(),
        node_config,
        use_txn_payload_v2_format,
        use_orderless_transactions,
    );

    let admin0 = context.root_account().await;
    let other_account = context.create_account().await;

    context.simulate_transaction(&admin0, json!({
        "type": "script_payload",
        "code": {
            "bytecode": "a11ceb0b030000000105000100000000050601000000000000000600000000000000001a0102",
        },
        "type_arguments": [],
        "arguments": [],
    }), 200).await;

    let resp = context.simulate_transaction(&other_account, json!({
        "type": "script_payload",
        "code": {
            "bytecode": "a11ceb0b030000000105000100000000050601000000000000000600000000000000001a0102",
        },
        "type_arguments": [],
        "arguments": [],
    }), 403).await;

    // It was difficult to prune when using a vec of responses so we just put the
    // rejection response in the goldens.
    context.check_golden_output(resp);
}

fn gen_string(len: u64) -> String {
    let mut rng = thread_rng();
    std::iter::repeat(())
        .map(|()| rng.sample(Alphanumeric))
        .take(len as usize)
        .map(char::from)
        .collect()
}

// For use when not using the methods on `TestContext` directly.
fn build_path(path: &str) -> String {
    format!("/v1/transactions{}", path)
}
