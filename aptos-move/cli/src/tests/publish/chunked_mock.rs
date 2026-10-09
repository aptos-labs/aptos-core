// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Chunked publish against a mocked chain. A local session stands in for the on-chain
//! `StagingArea` check; transaction submission goes to the mock.

use crate::tests::common;
use aptos_cli_common::TransactionSummary;
use aptos_transaction_simulation::SimulationStateStore;
use aptos_transaction_simulation_session::Session;
use aptos_types::{
    state_store::{state_key::StateKey, state_value::StateValue},
    transaction::TransactionPayload,
};
use move_core_types::{account_address::AccountAddress, language_storage::StructTag};
use std::{
    path::Path,
    str::FromStr,
    sync::{Arc, Mutex},
};
use tempfile::TempDir;

const SENDER: &str = "0xcafe";
const PRIVATE_KEY: &str = "0x1111111111111111111111111111111111111111111111111111111111111111";

fn summary(success: bool) -> TransactionSummary {
    TransactionSummary {
        transaction_hash: aptos_crypto::HashValue::zero().into(),
        gas_used: None,
        gas_unit_price: None,
        pending: None,
        sender: None,
        sequence_number: None,
        replay_protector: None,
        success: Some(success),
        timestamp_us: None,
        version: None,
        vm_status: (!success).then(|| "Out of gas".to_string()),
        deployed_object_address: None,
        events: None,
        changes: None,
    }
}

fn entry_function_name(payload: &TransactionPayload) -> String {
    let TransactionPayload::EntryFunction(entry_function) = payload else {
        panic!("expected an entry function payload, got {:?}", payload);
    };
    entry_function.function().to_string()
}

/// Creates a session; if `stale_staging_area` is set, the sender's `StagingArea` holds data
/// left by an earlier chunked publish.
fn init_session(path: &Path, stale_staging_area: bool) {
    let mut session = Session::init(path).expect("session init");
    if stale_staging_area {
        let sender = AccountAddress::from_hex_literal(SENDER).unwrap();
        let tag = StructTag::from_str("0x7::large_packages::StagingArea").unwrap();
        session
            .state_store()
            .set_state_value(
                StateKey::resource(&sender, &tag).unwrap(),
                StateValue::new_legacy(vec![0u8].into()),
            )
            .unwrap();
        // Funding is a session operation, which persists the state written above.
        session.fund_account(sender, 1).unwrap();
    }
}

/// Runs a chunked publish of a small package, split into several chunks, and returns the
/// result together with the entry functions submitted, in order.
fn run_chunked_publish(
    stale_staging_area: bool,
    mut submit_result: impl FnMut(usize) -> TransactionSummary + Send + 'static,
) -> (Result<String, String>, Vec<String>) {
    let pkg = common::make_package("chunked_mock", &[(
        "chunked_mock",
        "module 0xCAFE::chunked_mock {
    public fun hello(): u64 { 42 }
}",
    )]);
    let session_dir = TempDir::new().unwrap();
    let session_path = session_dir.path().join("session");
    init_session(&session_path, stale_staging_area);

    let submitted = Arc::new(Mutex::new(vec![]));
    let recorder = submitted.clone();
    let (env, buffer) = common::env_with_mock(move |ctx| {
        ctx.expect_submit_transaction()
            .returning(move |_, payload| {
                let mut submitted = recorder.lock().unwrap();
                submitted.push(entry_function_name(&payload));
                Ok(submit_result(submitted.len()))
            });
    });

    let output = common::run_cli_with_env(
        &[
            "publish",
            "--package-dir",
            pkg.path().to_str().unwrap(),
            "--skip-fetch-latest-git-deps",
            "--assume-yes",
            "--chunked-publish",
            "--chunk-size",
            "200",
            "--large-packages-module-address",
            "0x7",
            "--session",
            session_path.to_str().unwrap(),
            "--private-key",
            PRIVATE_KEY,
            "--sender-account",
            SENDER,
        ],
        env,
        buffer,
    );
    let submitted = submitted.lock().unwrap().clone();
    (output.result, submitted)
}

#[test]
fn chunked_publish_clears_stale_staging_area_first() {
    let (result, submitted) = run_chunked_publish(true, |_| summary(true));

    assert!(result.is_ok(), "{:?}", result);
    assert!(submitted.len() > 2, "{:?}", submitted);
    assert_eq!(submitted[0], "cleanup_staging_area");
    let (last, staging) = submitted[1..].split_last().unwrap();
    assert!(
        staging.iter().all(|f| f == "stage_code_chunk"),
        "{:?}",
        submitted
    );
    assert_eq!(last, "stage_code_chunk_and_publish_to_account");
}

#[test]
fn chunked_publish_stops_at_failed_transaction() {
    // The second chunk fails; nothing after it may be submitted.
    let (result, submitted) = run_chunked_publish(false, |call| summary(call != 2));

    let error = result.expect_err("chunked publish should fail");
    assert!(error.contains("Out of gas"), "{}", error);
    assert!(error.contains("transaction 2 of"), "{}", error);
    assert_eq!(submitted, vec!["stage_code_chunk", "stage_code_chunk"]);
}
