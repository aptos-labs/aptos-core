// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Tests [`TransactionalSession`]: publishing replaces stored bytes on a
//! compatible upgrade and preserves them when V1 rejects an incompatible
//! upgrade; running commits resource writes only when the call succeeds.

use bytes::Bytes;
use mono_move_testsuite::{
    compile_move_source, function_def_index, ArgumentError, PublishError, RunError, RunOutcome,
    TransactionalSession,
};
use move_binary_format::{compatibility::Compatibility, CompiledModule};
use move_core_types::{
    account_address::AccountAddress,
    ident_str,
    identifier::IdentStr,
    language_storage::{ModuleId, TypeTag},
    value::MoveValue,
    vm_status::{AbortLocation, StatusCode},
};
use move_transactional_test_runner::vm_test_harness::TestRunConfig;
use move_vm_types::code::ModuleBytesStorage;

const ADDRESS: AccountAddress = AccountAddress::new([
    0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
    0x42,
]);

const ORIGINAL: &str = "module 0x42::m { public fun f(): u64 { 1 } }";
/// Keeps `f` and adds a function: a compatible upgrade.
const COMPATIBLE: &str = "module 0x42::m { public fun f(): u64 { 2 } public fun g(): u64 { 3 } }";
/// Drops the public `f`: an incompatible upgrade.
const INCOMPATIBLE: &str = "module 0x42::m { public fun g(): u64 { 3 } }";

/// A counter resource with functions that create, bump, read, and bump-then-abort.
const COUNTER: &str = r#"module 0x42::m {
    struct Counter has key { value: u64 }
    public fun init(account: &signer) { move_to(account, Counter { value: 1 }) }
    public fun bump(addr: address) acquires Counter {
        let counter = borrow_global_mut<Counter>(addr);
        counter.value = counter.value + 1;
    }
    public fun read(addr: address): u64 acquires Counter { borrow_global<Counter>(addr).value }
    public fun bump_then_abort(addr: address) acquires Counter { bump(addr); abort 7 }
}"#;

fn session() -> TransactionalSession {
    TransactionalSession::new(&TestRunConfig::default().vm_config)
}

fn compiled(source: &str) -> CompiledModule {
    compile_move_source(source)
        .expect("the source compiles")
        .pop()
        .expect("the source defines one module")
}

fn serialized(module: &CompiledModule) -> Bytes {
    let mut bytes = vec![];
    module.serialize(&mut bytes).expect("the module serializes");
    bytes.into()
}

/// A session with `source` published, and the module it compiled to.
fn session_with(source: &str) -> (TransactionalSession, CompiledModule) {
    let module = compiled(source);
    let mut session = session();
    session
        .publish(&ADDRESS, Compatibility::full_check(), vec![serialized(
            &module,
        )])
        .expect("the module publishes");
    (session, module)
}

fn stored(session: &TransactionalSession) -> Option<Bytes> {
    session
        .storage()
        .fetch_module_bytes(&ADDRESS, ident_str!("m"))
        .expect("the store is readable")
}

fn module() -> ModuleId {
    ModuleId::new(ADDRESS, ident_str!("m").to_owned())
}

fn address_arg() -> Vec<u8> {
    MoveValue::Address(ADDRESS)
        .simple_serialize()
        .expect("an address serializes")
}

fn run(
    session: &mut TransactionalSession,
    function: &IdentStr,
    signers: &[AccountAddress],
    args: &[Vec<u8>],
) -> Result<RunOutcome, RunError> {
    session.run(&module(), function, &[], signers, args)
}

/// Runs `function` with `args` and returns its single `u64` result.
fn run_u64(session: &mut TransactionalSession, function: &IdentStr, args: &[Vec<u8>]) -> u64 {
    match run(session, function, &[], args).unwrap_or_else(|err| panic!("`{function}` runs: {err}"))
    {
        RunOutcome::Success { return_values } => {
            let [(TypeTag::U64, bytes)] = return_values.as_slice() else {
                panic!("`{function}` returns one u64, got {return_values:?}");
            };
            bcs::from_bytes(bytes).expect("a u64 decodes")
        },
        RunOutcome::Aborted { code, .. } => panic!("`{function}` aborted with {code}"),
    }
}

/// The counter's value.
fn read_counter(session: &mut TransactionalSession) -> u64 {
    run_u64(session, ident_str!("read"), &[address_arg()])
}

#[test]
fn a_compatible_republish_replaces_the_module() {
    let mut session = session();
    let original = serialized(&compiled(ORIGINAL));
    let upgrade = serialized(&compiled(COMPATIBLE));

    session
        .publish(
            &ADDRESS,
            Compatibility::full_check(),
            vec![original.clone()],
        )
        .expect("the first publish succeeds");
    assert_eq!(stored(&session), Some(original));

    session
        .publish(&ADDRESS, Compatibility::full_check(), vec![upgrade.clone()])
        .expect("a compatible upgrade succeeds");
    assert_eq!(stored(&session), Some(upgrade));
}

/// Each operation resets the context's caches, so a run after a republish
/// executes the new code rather than the code loaded for an earlier run.
#[test]
fn a_run_after_a_compatible_republish_executes_the_new_code() {
    let (mut session, _) = session_with(ORIGINAL);
    assert_eq!(run_u64(&mut session, ident_str!("f"), &[]), 1);

    session
        .publish(&ADDRESS, Compatibility::full_check(), vec![serialized(
            &compiled(COMPATIBLE),
        )])
        .expect("a compatible upgrade succeeds");
    assert_eq!(run_u64(&mut session, ident_str!("f"), &[]), 2);
}

#[test]
fn an_incompatible_republish_is_rejected_and_keeps_the_original() {
    let mut session = session();
    let original = serialized(&compiled(ORIGINAL));
    session
        .publish(
            &ADDRESS,
            Compatibility::full_check(),
            vec![original.clone()],
        )
        .expect("the first publish succeeds");

    let error = session
        .publish(&ADDRESS, Compatibility::full_check(), vec![serialized(
            &compiled(INCOMPATIBLE),
        )])
        .expect_err("dropping a public function is incompatible");
    match error {
        PublishError::Staging(error) => assert_eq!(
            error.major_status(),
            StatusCode::BACKWARD_INCOMPATIBLE_MODULE_UPDATE
        ),
        PublishError::MonoLoad { module, error } => {
            panic!("V1 should have rejected the upgrade before MonoVM saw {module}: {error}")
        },
    }
    assert_eq!(stored(&session), Some(original));
}

#[test]
fn a_successful_run_commits_its_writes_for_later_runs() {
    let (mut session, _) = session_with(COUNTER);
    run(&mut session, ident_str!("init"), &[ADDRESS], &[]).expect("`init` runs");
    assert_eq!(read_counter(&mut session), 1);

    run(&mut session, ident_str!("bump"), &[], &[address_arg()]).expect("`bump` runs");
    assert_eq!(read_counter(&mut session), 2);
}

#[test]
fn an_abort_discards_the_run_writes() {
    let (mut session, counter) = session_with(COUNTER);
    run(&mut session, ident_str!("init"), &[ADDRESS], &[]).expect("`init` runs");

    let outcome = run(&mut session, ident_str!("bump_then_abort"), &[], &[
        address_arg(),
    ])
    .expect("an abort is an outcome, not an error");
    match outcome {
        RunOutcome::Aborted {
            code,
            location,
            offset,
            ..
        } => {
            assert_eq!(code, 7);
            assert_eq!(location, AbortLocation::Module(module()));
            let (function, _) = offset.expect("a Move-level abort names its instruction");
            assert_eq!(
                Some(function),
                function_def_index(&counter, "bump_then_abort")
            );
        },
        RunOutcome::Success { .. } => panic!("`bump_then_abort` aborts"),
    }
    assert_eq!(read_counter(&mut session), 1);
}

/// V1 encodes a signer as a one-variant enum, so a signer handed to an
/// `address` parameter does not decode.
#[test]
fn a_signer_on_an_address_parameter_is_undecodable() {
    let (mut session, _) = session_with(COUNTER);
    let error = run(&mut session, ident_str!("read"), &[ADDRESS], &[])
        .expect_err("`read` takes an address, not a signer");
    match error {
        RunError::Arguments(error) => assert_eq!(error, ArgumentError::Undecodable),
        other => panic!("expected an argument error, got {other}"),
    }
}

/// The count check precedes the call, so a miscounted `bump` leaves the
/// counter untouched.
#[test]
fn an_argument_count_mismatch_is_reported_before_running() {
    let (mut session, _) = session_with(COUNTER);
    run(&mut session, ident_str!("init"), &[ADDRESS], &[]).expect("`init` runs");

    let error =
        run(&mut session, ident_str!("bump"), &[], &[]).expect_err("`bump` takes one argument");
    match error {
        RunError::Arguments(error) => {
            assert_eq!(error, ArgumentError::CountMismatch {
                expected: 1,
                actual: 0
            })
        },
        other => panic!("expected an argument error, got {other}"),
    }
    assert_eq!(read_counter(&mut session), 1);
}
