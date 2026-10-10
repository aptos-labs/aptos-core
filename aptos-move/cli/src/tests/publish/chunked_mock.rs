// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Chunked publish against a mocked chain. A local session holds the state that the CLI reads, such
//! as a `StagingArea` left by an earlier chunked publish; transaction submission goes to the mock.

use crate::tests::common;
use aptos_cli_common::TransactionSummary;
use aptos_framework::{
    chunked_publish::chunk_key,
    natives::code::{PackageMetadata, PackageRegistry, UpgradePolicy},
};
use aptos_package_builder::PackageBuilder;
use aptos_transaction_simulation::SimulationStateStore;
use aptos_transaction_simulation_session::Session;
use aptos_types::{
    account_config::AccountResource,
    event::{EventHandle, EventKey},
    object_address::create_object_code_deployment_address,
    state_store::{state_key::StateKey, state_value::StateValue, table::TableHandle},
    transaction::{EntryFunction, TransactionPayload},
};
use move_binary_format::CompiledModule;
use move_core_types::{account_address::AccountAddress, language_storage::StructTag};
use std::{
    path::Path,
    str::FromStr,
    sync::{Arc, Mutex},
};
use tempfile::TempDir;

const SENDER: &str = "0xcafe";
const PRIVATE_KEY: &str = "0x1111111111111111111111111111111111111111111111111111111111111111";

/// A chunk of a package: its part, its index in the part, and its bytes.
type Chunk = (u16, u16, Vec<u8>);

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

fn entry_function(payload: &TransactionPayload) -> &EntryFunction {
    let TransactionPayload::EntryFunction(entry_function) = payload else {
        panic!("expected an entry function payload, got {:?}", payload);
    };
    entry_function
}

fn entry_function_name(payload: &TransactionPayload) -> String {
    entry_function(payload).function().to_string()
}

/// What the sender's `StagingArea` holds when a test starts.
enum Staged {
    /// There is no `StagingArea`.
    Nothing,
    /// There is a resource that isn't laid out like a `StagingArea`, e.g. one that an older version
    /// of the module left.
    Garbage,
    /// There is a `StagingArea` with these chunks.
    Chunks(Vec<Chunk>),
}

fn write_state(session: &mut Session, key: StateKey, bytes: Vec<u8>) {
    session
        .state_store()
        .set_state_value(key, StateValue::new_legacy(bytes.into()))
        .unwrap();
}

fn init_session(path: &Path, staged: Staged) {
    let mut session = Session::init(path).expect("session init");
    let sender = AccountAddress::from_hex_literal(SENDER).unwrap();
    let staging_area = StateKey::resource(
        &sender,
        &StructTag::from_str("0x7::large_packages::StagingArea").unwrap(),
    )
    .unwrap();
    match staged {
        Staged::Nothing => return,
        Staged::Garbage => write_state(&mut session, staging_area, vec![0u8]),
        Staged::Chunks(chunks) => {
            // StagingArea { chunks: TableWithLength { inner: Table { handle }, length }, keys }
            let handle = AccountAddress::from_hex_literal("0xabc").unwrap();
            let keys: Vec<u64> = chunks
                .iter()
                .map(|(part, index, _)| chunk_key(*part, *index))
                .collect();
            let mut bytes = bcs::to_bytes(&handle).unwrap();
            bytes.extend(bcs::to_bytes(&(chunks.len() as u64)).unwrap());
            bytes.extend(bcs::to_bytes(&keys).unwrap());
            write_state(&mut session, staging_area, bytes);
            for ((_, _, chunk), key) in chunks.iter().zip(keys) {
                write_state(
                    &mut session,
                    StateKey::table_item(&TableHandle(handle), &bcs::to_bytes(&key).unwrap()),
                    bcs::to_bytes(chunk).unwrap(),
                );
            }
        },
    }
    // Funding is a session operation, which persists the state written above.
    session.fund_account(sender, 1).unwrap();
}

/// The outcome of a command run against the mocked chain.
struct Run {
    result: Result<String, String>,
    /// The transactions that were submitted, in order.
    submitted: Vec<TransactionPayload>,
}

impl Run {
    fn names(&self) -> Vec<String> {
        self.submitted.iter().map(entry_function_name).collect()
    }

    /// The chunks that the transactions stage, in the order in which they are submitted.
    fn staged_chunks(&self) -> Vec<Chunk> {
        let mut chunks = vec![];
        for payload in &self.submitted {
            let entry_function = entry_function(payload);
            if !entry_function
                .function()
                .as_str()
                .starts_with("stage_chunks")
            {
                continue;
            }
            let args = entry_function.args();
            let parts: Vec<u16> = bcs::from_bytes(&args[0]).unwrap();
            let indices: Vec<u16> = bcs::from_bytes(&args[1]).unwrap();
            let bytes: Vec<Vec<u8>> = bcs::from_bytes(&args[2]).unwrap();
            for ((part, index), bytes) in parts.into_iter().zip(indices).zip(bytes) {
                chunks.push((part, index, bytes));
            }
        }
        chunks
    }

    /// The number of chunks in each part of the package, which the publishing transaction carries.
    fn chunk_counts(&self) -> Vec<u16> {
        let last = entry_function(self.submitted.last().expect("a transaction was submitted"));
        bcs::from_bytes(&last.args()[3]).unwrap()
    }
}

fn run_command(
    args: &[&str],
    staged: Staged,
    submit_result: impl FnMut(usize) -> TransactionSummary + Send + 'static,
) -> Run {
    run_command_with_setup(args, staged, |_| {}, submit_result)
}

/// Like [`run_command`], and `setup` can put anything else that a test needs into the session.
fn run_command_with_setup(
    args: &[&str],
    staged: Staged,
    setup: impl FnOnce(&mut Session),
    mut submit_result: impl FnMut(usize) -> TransactionSummary + Send + 'static,
) -> Run {
    let session_dir = TempDir::new().unwrap();
    let session_path = session_dir.path().join("session");
    init_session(&session_path, staged);
    let mut session = Session::load(&session_path).expect("session load");
    setup(&mut session);
    // Funding is a session operation, which persists the state written above.
    session
        .fund_account(AccountAddress::from_hex_literal(SENDER).unwrap(), 1)
        .unwrap();
    drop(session);

    let submitted = Arc::new(Mutex::new(vec![]));
    let recorder = submitted.clone();
    let (env, buffer) = common::env_with_mock(move |ctx| {
        ctx.expect_submit_transaction()
            .returning(move |_, payload| {
                let mut submitted = recorder.lock().unwrap();
                submitted.push(payload);
                Ok(submit_result(submitted.len()))
            });
    });

    let mut args = args.to_vec();
    args.extend([
        "--assume-yes",
        "--large-packages-module-address",
        "0x7",
        "--session",
        session_path.to_str().unwrap(),
        "--private-key",
        PRIVATE_KEY,
        "--sender-account",
        SENDER,
    ]);
    let output = common::run_cli_with_env(&args, env, buffer);
    let submitted = submitted.lock().unwrap().clone();
    Run {
        result: output.result,
        submitted,
    }
}

/// A small package, which is split into several chunks.
fn make_test_package() -> TempDir {
    common::make_package("chunked_mock", &[(
        "chunked_mock",
        "module 0xCAFE::chunked_mock {
    public fun hello(): u64 { 42 }
}",
    )])
}

/// Runs a chunked publish of `pkg` with a chunk size of 64 bytes.
fn run_chunked_publish(
    pkg: &TempDir,
    staged: Staged,
    submit_result: impl FnMut(usize) -> TransactionSummary + Send + 'static,
) -> Run {
    run_command(
        &[
            "publish",
            "--package-dir",
            pkg.path().to_str().unwrap(),
            "--skip-fetch-latest-git-deps",
            "--chunked-publish",
            "--chunk-size",
            "64",
        ],
        staged,
        submit_result,
    )
}

/// The `stage_chunks` transactions followed by the one that publishes.
fn staging_then_publishing(names: &[String]) -> bool {
    match names.split_last() {
        Some((last, staging)) => {
            last == "stage_chunks_and_publish_to_account"
                && staging.iter().all(|name| name == "stage_chunks")
        },
        None => false,
    }
}

#[test]
fn chunked_publish_clears_stale_staging_area_first() {
    // The session's genesis has the current `large_packages` module, so its positional entry
    // functions are used.
    let run = run_chunked_publish(&make_test_package(), Staged::Garbage, |_| summary(true));

    assert!(run.result.is_ok(), "{:?}", run.result);
    let names = run.names();
    assert!(names.len() > 2, "{:?}", names);
    assert_eq!(names[0], "cleanup_staging_area");
    assert!(staging_then_publishing(&names[1..]), "{:?}", names);
}

#[test]
fn chunked_publish_stops_at_failed_transaction() {
    // The second chunk fails; nothing after it may be submitted.
    let run = run_chunked_publish(&make_test_package(), Staged::Nothing, |call| {
        summary(call != 2)
    });

    let names = run.names();
    let error = run.result.expect_err("chunked publish should fail");
    assert!(error.contains("Out of gas"), "{}", error);
    assert!(error.contains("transaction 2 of"), "{}", error);
    assert_eq!(names, vec!["stage_chunks", "stage_chunks"]);
}

#[test]
fn chunked_publish_resumes_incomplete_publish() {
    let pkg = make_test_package();
    let full = run_chunked_publish(&pkg, Staged::Nothing, |_| summary(true));
    assert!(full.result.is_ok(), "{:?}", full.result);
    let all = full.staged_chunks();
    assert!(all.len() >= 4, "the package needs several chunks");

    // An earlier publish got as far as staging the first two chunks.
    let resumed = run_chunked_publish(&pkg, Staged::Chunks(all[..2].to_vec()), |_| summary(true));

    assert!(resumed.result.is_ok(), "{:?}", resumed.result);
    let names = resumed.names();
    assert!(staging_then_publishing(&names), "{:?}", names);
    assert_eq!(resumed.staged_chunks(), all[2..].to_vec());
    assert!(resumed.submitted.len() < full.submitted.len());
    // The module is still told how many chunks make up the whole package.
    assert_eq!(resumed.chunk_counts(), full.chunk_counts());
}

#[test]
fn chunked_publish_resumes_with_only_the_publishing_transaction_left() {
    let pkg = make_test_package();
    let full = run_chunked_publish(&pkg, Staged::Nothing, |_| summary(true));
    let all = full.staged_chunks();

    // Every chunk was staged, but the transaction that publishes the package failed.
    let resumed = run_chunked_publish(&pkg, Staged::Chunks(all), |_| summary(true));

    assert!(resumed.result.is_ok(), "{:?}", resumed.result);
    assert_eq!(resumed.names(), vec!["stage_chunks_and_publish_to_account"]);
    assert!(resumed.staged_chunks().is_empty());
    assert_eq!(resumed.chunk_counts(), full.chunk_counts());
}

#[test]
fn chunked_publish_clears_staged_chunks_of_another_package() {
    let pkg = make_test_package();
    let full = run_chunked_publish(&pkg, Staged::Nothing, |_| summary(true));
    let mut staged = full.staged_chunks();
    // The package was changed after the chunks were staged: one chunk differs.
    staged[1].2[0] ^= 0xFF;

    let run = run_chunked_publish(&pkg, Staged::Chunks(staged), |_| summary(true));

    assert!(run.result.is_ok(), "{:?}", run.result);
    let names = run.names();
    assert_eq!(names[0], "cleanup_staging_area");
    assert_eq!(names[1..], full.names()[..]);
    assert_eq!(run.staged_chunks(), full.staged_chunks());
}

#[test]
fn chunked_publish_clears_staged_chunk_the_package_does_not_have() {
    let pkg = make_test_package();
    let full = run_chunked_publish(&pkg, Staged::Nothing, |_| summary(true));
    let mut staged = full.staged_chunks()[..2].to_vec();
    // A chunk of a module that the package doesn't have.
    staged.push((100, 0, vec![1, 2, 3]));

    let run = run_chunked_publish(&pkg, Staged::Chunks(staged), |_| summary(true));

    assert!(run.result.is_ok(), "{:?}", run.result);
    let names = run.names();
    assert_eq!(names[0], "cleanup_staging_area");
    assert_eq!(names[1..], full.names()[..]);
}

#[test]
fn chunked_publish_with_empty_staging_area_stages_everything() {
    let pkg = make_test_package();
    let full = run_chunked_publish(&pkg, Staged::Nothing, |_| summary(true));

    let run = run_chunked_publish(&pkg, Staged::Chunks(vec![]), |_| summary(true));

    assert!(run.result.is_ok(), "{:?}", run.result);
    assert_eq!(run.names(), full.names());
    assert_eq!(run.staged_chunks(), full.staged_chunks());
}

fn run_clear_staging_area(staged: Staged) -> Run {
    run_command(&["clear-staging-area"], staged, |_| summary(true))
}

#[test]
fn clear_staging_area_without_staging_area_submits_nothing() {
    let run = run_clear_staging_area(Staged::Nothing);

    let names = run.names();
    let output: serde_json::Value = serde_json::from_str(&run.result.expect("succeeds")).unwrap();
    assert!(output["Result"].is_null(), "{}", output);
    assert!(names.is_empty(), "{:?}", names);
}

#[test]
fn clear_staging_area_removes_staging_area() {
    let run = run_clear_staging_area(Staged::Garbage);

    assert!(run.result.is_ok(), "{:?}", run.result);
    assert_eq!(run.names(), vec!["cleanup_staging_area"]);
}

/// A small package whose address is assigned on the command line, as it is for an object.
fn make_object_test_package() -> TempDir {
    let mut builder = PackageBuilder::new("chunked_mock");
    builder.add_alias("chunked_mock", "_");
    builder.add_source(
        "chunked_mock",
        "module chunked_mock::chunked_mock {
    public fun hello(): u64 { 42 }
}",
    );
    builder
        .write_to_temp()
        .expect("failed to create temp package")
}

/// Gives the sender's account in the session this sequence number.
fn set_sender_sequence_number(session: &mut Session, sequence_number: u64) {
    let sender = AccountAddress::from_hex_literal(SENDER).unwrap();
    let account = AccountResource::new(
        sequence_number,
        vec![0; 32],
        EventHandle::new(EventKey::new(0, sender), 0),
        EventHandle::new(EventKey::new(1, sender), 0),
    );
    session
        .state_store()
        .set_resource(sender, &account)
        .unwrap();
}

/// Publishes a package named `chunked_mock` at `object` in the session, with this upgrade policy.
fn set_package_registry(session: &mut Session, object: AccountAddress, policy: UpgradePolicy) {
    let registry = PackageRegistry {
        packages: vec![PackageMetadata {
            name: "chunked_mock".to_string(),
            upgrade_policy: policy,
            upgrade_number: 0,
            source_digest: String::new(),
            manifest: vec![],
            modules: vec![],
            deps: vec![],
            extension: None,
        }],
    };
    session
        .state_store()
        .set_resource(object, &registry)
        .unwrap();
}

/// The address that the first module of the package was compiled for, from the chunks that the
/// transactions stage.
fn staged_module_address(run: &Run) -> AccountAddress {
    // Part 0 is the package metadata, and part 1 the code of the first module.
    let module: Vec<u8> = run
        .staged_chunks()
        .into_iter()
        .filter(|(part, ..)| *part == 1)
        .flat_map(|(_, _, bytes)| bytes)
        .collect();
    *CompiledModule::deserialize(&module)
        .expect("the staged chunks are a module")
        .self_addr()
}

fn run_create_object(chunked: bool, sequence_number: u64) -> Run {
    let pkg = make_object_test_package();
    let mut args = vec![
        "create-object-and-publish-package",
        "--address-name",
        "chunked_mock",
        "--package-dir",
        pkg.path().to_str().unwrap(),
        "--skip-fetch-latest-git-deps",
    ];
    if chunked {
        args.extend(["--chunked-publish", "--chunk-size", "64"]);
    }
    run_command_with_setup(
        &args,
        Staged::Nothing,
        move |session| set_sender_sequence_number(session, sequence_number),
        |_| summary(true),
    )
}

#[test]
fn chunked_object_publish_derives_address_from_session_sequence_number() {
    let sender = AccountAddress::from_hex_literal(SENDER).unwrap();

    let run = run_create_object(true, 5);

    assert!(run.result.is_ok(), "{:?}", run.result);
    let names = run.names();
    assert_eq!(names.last().unwrap(), "stage_chunks_and_publish_to_object");
    // The sender has sent 5 transactions in the session, and the object is created by the last of
    // the transactions of this publish, so it is derived from the sequence number after that one.
    let expected = create_object_code_deployment_address(sender, 5 + run.submitted.len() as u64);
    assert_eq!(staged_module_address(&run), expected);
}

#[test]
fn object_publish_derives_address_from_session_sequence_number() {
    let sender = AccountAddress::from_hex_literal(SENDER).unwrap();

    let run = run_create_object(false, 5);

    assert!(run.result.is_ok(), "{:?}", run.result);
    assert_eq!(run.submitted.len(), 1);
    let args = entry_function(&run.submitted[0]).args();
    let code: Vec<Vec<u8>> = bcs::from_bytes(&args[1]).unwrap();
    let address = *CompiledModule::deserialize(&code[0]).unwrap().self_addr();
    assert_eq!(address, create_object_code_deployment_address(sender, 6));
}

fn run_upgrade_object_package(policy: UpgradePolicy) -> Run {
    let pkg = make_object_test_package();
    let object = AccountAddress::from_hex_literal("0xabc123").unwrap();
    run_command_with_setup(
        &[
            "upgrade-object-package",
            "--object-address",
            "0xabc123",
            "--named-addresses",
            "chunked_mock=0xabc123",
            "--package-dir",
            pkg.path().to_str().unwrap(),
            "--skip-fetch-latest-git-deps",
            "--chunked-publish",
            "--chunk-size",
            "64",
        ],
        Staged::Nothing,
        move |session| set_package_registry(session, object, policy),
        |_| summary(true),
    )
}

#[test]
fn upgrade_object_package_reads_the_package_from_the_session() {
    let run = run_upgrade_object_package(UpgradePolicy::compat());

    assert!(run.result.is_ok(), "{:?}", run.result);
    assert_eq!(
        run.names().last().unwrap(),
        "stage_chunks_and_upgrade_object_code"
    );
}

#[test]
fn upgrade_object_package_with_immutable_package_in_session_fails() {
    let run = run_upgrade_object_package(UpgradePolicy::immutable());

    let names = run.names();
    let error = run
        .result
        .expect_err("an immutable package can't be upgraded");
    assert!(error.contains("immutable"), "{}", error);
    assert!(names.is_empty(), "{:?}", names);
}
