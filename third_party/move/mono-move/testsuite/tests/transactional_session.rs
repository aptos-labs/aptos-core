// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Tests that [`TransactionalSession::publish`] replaces stored bytes on a
//! compatible upgrade and preserves them when V1 rejects an incompatible upgrade.

use bytes::Bytes;
use mono_move_testsuite::{compile_move_source, PublishError, TransactionalSession};
use move_binary_format::compatibility::Compatibility;
use move_core_types::{account_address::AccountAddress, ident_str, vm_status::StatusCode};
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

fn session() -> TransactionalSession {
    TransactionalSession::new(&TestRunConfig::default().vm_config)
}

fn serialized(source: &str) -> Bytes {
    let module = compile_move_source(source)
        .expect("the source compiles")
        .pop()
        .expect("the source defines one module");
    let mut bytes = vec![];
    module.serialize(&mut bytes).expect("the module serializes");
    bytes.into()
}

fn stored(session: &TransactionalSession) -> Option<Bytes> {
    session
        .storage()
        .fetch_module_bytes(&ADDRESS, ident_str!("m"))
        .expect("the store is readable")
}

#[test]
fn a_compatible_republish_replaces_the_module() {
    let mut session = session();
    let original = serialized(ORIGINAL);
    let upgrade = serialized(COMPATIBLE);

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

#[test]
fn an_incompatible_republish_is_rejected_and_keeps_the_original() {
    let mut session = session();
    let original = serialized(ORIGINAL);
    session
        .publish(
            &ADDRESS,
            Compatibility::full_check(),
            vec![original.clone()],
        )
        .expect("the first publish succeeds");

    let error = session
        .publish(&ADDRESS, Compatibility::full_check(), vec![serialized(
            INCOMPATIBLE,
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
