// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! End-to-end coverage for the transaction read sets recorded for hot-state promotion.
//!
//! Each test drives a small block through real Move execution and inspects the block epilogue's
//! `to_make_hot` set. The invariants under test are: a slot only *read* in the block is promoted, a
//! slot the block *writes* (by any transaction) is not, and the promotion set is identical under
//! sequential and parallel execution. Targeted tests cover each kind of read the VM records (plain
//! resources, resource-group members, table items, modules, on-chain configs) plus the `exists`
//! behavior and discard handling, and — for each write kind enumerated by `storage_keys_written`
//! (plain resources, resource groups, aggregator v1 materializations/deltas, in-place delayed
//! fields, and modules) — that a written slot is excluded from promotion.

use crate::{aggregator, assert_success, tests::common, MoveHarness};
use aptos_block_executor::txn_provider::default::DefaultTxnProvider;
use aptos_cached_packages::aptos_stdlib;
use aptos_crypto::HashValue;
use aptos_framework::{BuildOptions, BuiltPackage};
use aptos_language_e2e_tests::account::Account;
use aptos_types::{
    account_config::{AccountResource, AggregatorV1Resource, CoinInfoResource},
    block_executor::{
        config::{BlockExecutorConfig, BlockExecutorConfigFromOnchain, BlockExecutorLocalConfig},
        transaction_slice_metadata::TransactionSliceMetadata,
    },
    chain_id::ChainId,
    move_utils::MemberId,
    on_chain_config::{CurrentTimeMicroseconds, Features},
    state_store::{
        state_key::{inner::StateKeyInner, StateKey},
        table::TableHandle,
    },
    transaction::{
        signature_verified_transaction::into_signature_verified_block, ExecutionStatus,
        Transaction, TransactionArgument, TransactionStatus,
    },
    utility_coin::AptosCoinType,
};
use aptos_vm::{aptos_vm::AptosVMBlockExecutor, VMBlockExecutor};
use move_core_types::{
    account_address::AccountAddress, ident_str, language_storage::StructTag,
    parser::parse_struct_tag,
};
use std::collections::BTreeSet;

const HELPER_ADDR: &str = "0xcafe";

/// Executes the block against the harness state (without applying it) and returns the per-txn
/// statuses, the epilogue's `to_make_hot` set, and all keys the block's outputs value-write.
fn execute_and_get_hot_state_promotions(
    h: &MoveHarness,
    txns: Vec<Transaction>,
    concurrency_level: usize,
) -> (
    Vec<TransactionStatus>,
    BTreeSet<StateKey>,
    BTreeSet<StateKey>,
) {
    let config = BlockExecutorConfig {
        local: BlockExecutorLocalConfig::default_with_concurrency_level(concurrency_level),
        // The hot state accumulator requires `add_block_limit_outcome_onchain`;
        // `with_features` turns on `hotness_in_epilogue` (in default features), which
        // selects the V2 epilogue payload carrying `to_make_hot`.
        onchain: BlockExecutorConfigFromOnchain::on_but_large_for_test()
            .with_features(&Features::default()),
    };
    let txn_provider = DefaultTxnProvider::new_without_info(into_signature_verified_block(txns));
    let block_output = AptosVMBlockExecutor::new()
        .execute_block_with_config(
            &txn_provider,
            h.executor.get_state_view(),
            config,
            TransactionSliceMetadata::block(HashValue::zero(), HashValue::new([1; 32])),
        )
        .expect("Block execution should succeed");
    let (outputs, epilogue_txn) = block_output.into_inner();

    let statuses = outputs
        .iter()
        .map(|output| output.status().clone())
        .collect();

    let written_keys = outputs
        .iter()
        .flat_map(|output| {
            output
                .write_set()
                .write_op_iter()
                .map(|(key, _)| key.clone())
        })
        .collect();

    let to_make_hot = match epilogue_txn
        .expect("Block epilogue must be created")
        .into_inner()
    {
        Transaction::BlockEpilogue(payload) => payload
            .try_get_keys_to_make_hot()
            .expect("Hotness must be enabled")
            .clone(),
        txn => panic!("Expected block epilogue, got: {:?}", txn),
    };
    (statuses, to_make_hot, written_keys)
}

/// Executes `txns` as a block at sequential (concurrency 1) and parallel (concurrency 4) settings,
/// asserts both promote exactly the same keys, and returns the sequential run's statuses, the
/// shared promotion set, and the keys the block wrote. Routing every test through here gives
/// sequential/parallel parity coverage for free.
fn promotions(
    h: &MoveHarness,
    txns: Vec<Transaction>,
) -> (
    Vec<TransactionStatus>,
    BTreeSet<StateKey>,
    BTreeSet<StateKey>,
) {
    let (statuses, sequential, written) = execute_and_get_hot_state_promotions(h, txns.clone(), 1);
    let (_, parallel, _) = execute_and_get_hot_state_promotions(h, txns, 4);
    assert_eq!(
        sequential, parallel,
        "sequential and parallel execution must promote the same keys",
    );
    (statuses, sequential, written)
}

fn assert_all_success(statuses: &[TransactionStatus]) {
    for status in statuses {
        assert!(
            matches!(status, TransactionStatus::Keep(ExecutionStatus::Success)),
            "expected all transactions to succeed, got {:?}",
            statuses,
        );
    }
}

fn helper_address() -> AccountAddress {
    AccountAddress::from_hex_literal(HELPER_ADDR).unwrap()
}

/// A `MemberId` for the entry function `0xcafe::read_helper::<name>`.
fn helper_fn(name: &str) -> MemberId {
    str::parse(&format!("{HELPER_ADDR}::read_helper::{name}")).unwrap()
}

/// The `StructTag` for `0xcafe::read_helper::<name>`.
fn helper_struct(name: &str) -> StructTag {
    parse_struct_tag(&format!("{HELPER_ADDR}::read_helper::{name}")).unwrap()
}

/// A user transaction calling `0xcafe::read_helper::<name>(target)`.
fn read_txn(
    h: &mut MoveHarness,
    signer: &Account,
    name: &str,
    target: &AccountAddress,
) -> Transaction {
    Transaction::UserTransaction(
        h.create_entry_function(signer, helper_fn(name), vec![], vec![
            bcs::to_bytes(target).unwrap()
        ]),
    )
}

/// Creates a harness with the `read_helper` package published at `0xcafe`, returning the harness
/// and the publisher account (needed to republish the package within a block).
fn new_harness_with_package() -> (MoveHarness, Account) {
    let mut h = MoveHarness::new();
    let publisher = h.new_account_at(helper_address());
    assert_success!(h.publish_package(&publisher, &common::test_dir_path("hot_state.data/pack")));
    (h, publisher)
}

/// As [`new_harness_with_package`], plus an `owner` account that already holds the read targets
/// (`Plain`, `InGroup`, `TableHolder`), created in a prior applied transaction so a later block
/// only reads them.
fn setup() -> (MoveHarness, Account) {
    let (mut h, _publisher) = new_harness_with_package();
    let owner = h.new_account_with_key_pair();
    assert_success!(h.run_entry_function(&owner, helper_fn("init"), vec![], vec![]));
    (h, owner)
}

/// A plain (non-group) resource that is only read is promoted at its own state key.
#[test]
fn test_plain_resource_read_is_promoted() {
    let (mut h, owner) = setup();
    let reader = h.new_account_with_key_pair();
    let txns = vec![read_txn(&mut h, &reader, "read_plain", owner.address())];

    let (statuses, promoted, written) = promotions(&h, txns);
    assert_all_success(&statuses);

    let plain = StateKey::resource(owner.address(), &helper_struct("Plain")).unwrap();
    assert!(
        promoted.contains(&plain),
        "a read-only resource must be promoted"
    );
    assert!(!written.contains(&plain));
}

/// Reading a resource-group member records the enclosing *group* key, not the member's own key.
#[test]
fn test_resource_group_member_read_promotes_group_key() {
    let (mut h, owner) = setup();
    let reader = h.new_account_with_key_pair();
    let txns = vec![read_txn(
        &mut h,
        &reader,
        "read_group_member",
        owner.address(),
    )];

    let (statuses, promoted, written) = promotions(&h, txns);
    assert_all_success(&statuses);

    let group = StateKey::resource_group(owner.address(), &helper_struct("Group"));
    let member = StateKey::resource(owner.address(), &helper_struct("InGroup")).unwrap();
    assert!(promoted.contains(&group), "the group key must be promoted");
    assert!(
        !promoted.contains(&member),
        "the per-member key is never used for group resources",
    );
    assert!(!written.contains(&group));
}

/// Writing a resource-group member writes the enclosing *group* slot (the same key a group read
/// records), so that slot must be excluded from promotion.
#[test]
fn test_written_group_member_excludes_group_key() {
    let (mut h, owner) = setup();
    // `write_group_member` reads (loads the group) and then mutates the owner's own `InGroup`.
    let txns = vec![Transaction::UserTransaction(h.create_entry_function(
        &owner,
        helper_fn("write_group_member"),
        vec![],
        vec![],
    ))];

    let (statuses, promoted, written) = promotions(&h, txns);
    assert_all_success(&statuses);

    let group = StateKey::resource_group(owner.address(), &helper_struct("Group"));
    assert!(
        written.contains(&group),
        "writing a group member must write the enclosing group slot",
    );
    assert!(
        !promoted.contains(&group),
        "a group slot written in the block must not be promoted, even though it was also read",
    );
}

/// `exists<T>` loads the slot to answer, so a read of an absent resource is still recorded and the
/// slot is promoted. This is the intended behavior change from the old summary-based derivation.
#[test]
fn test_exists_on_absent_resource_is_recorded() {
    let (mut h, _owner) = setup();
    let reader = h.new_account_with_key_pair();
    // A fresh account that never received a `Plain`.
    let absent_owner = h.new_account_with_key_pair();
    let txns = vec![read_txn(
        &mut h,
        &reader,
        "check_exists",
        absent_owner.address(),
    )];

    let (statuses, promoted, written) = promotions(&h, txns);
    assert_all_success(&statuses);

    let absent = StateKey::resource(absent_owner.address(), &helper_struct("Plain")).unwrap();
    assert!(
        promoted.contains(&absent),
        "exists<T> consults the slot, so even an absent resource counts as a read",
    );
    assert!(!written.contains(&absent));
}

/// A table item that is only read is promoted at its exact `TableItem` key.
#[test]
fn test_table_item_read_is_promoted() {
    let (mut h, owner) = setup();
    let reader = h.new_account_with_key_pair();

    // `TableHolder { entries: Table<u64, u64> }` serializes to just the table handle; recover it to
    // construct the exact item key for the entry `init` wrote (key 7).
    #[derive(serde::Deserialize)]
    struct TableHolderRepr {
        entries: TableHandle,
    }
    let handle = h
        .read_resource::<TableHolderRepr>(owner.address(), helper_struct("TableHolder"))
        .expect("TableHolder must exist after init")
        .entries;
    let item = StateKey::table_item(&handle, &bcs::to_bytes(&7u64).unwrap());

    let txns = vec![read_txn(
        &mut h,
        &reader,
        "read_table_item",
        owner.address(),
    )];
    let (statuses, promoted, written) = promotions(&h, txns);
    assert_all_success(&statuses);

    assert!(
        promoted.contains(&item),
        "a read-only table item must be promoted"
    );
    assert!(!written.contains(&item));
}

fn setup_aggregator_v1() -> (MoveHarness, Account, StateKey) {
    let (mut h, framework) = aggregator::initialize(common::test_dir_path("aggregator.data/pack"));
    let txn = aggregator::new(&mut h, &framework, 0);
    assert_success!(h.run(txn));

    #[derive(serde::Deserialize)]
    struct AggregatorStoreRepr {
        aggregators: TableHandle,
    }

    let handle = h
        .read_resource::<AggregatorStoreRepr>(
            framework.address(),
            parse_struct_tag("0x1::aggregator_test::AggregatorStore").unwrap(),
        )
        .expect("AggregatorStore must exist after initialize")
        .aggregators;
    let aggregator_item = StateKey::table_item(&handle, &bcs::to_bytes(&0u64).unwrap());
    let aggregator_key = bcs::from_bytes::<AggregatorV1Resource>(
        &h.read_state_value_bytes(&aggregator_item)
            .expect("aggregator table item must exist"),
    )
    .expect("aggregator table item must deserialize")
    .state_key();

    (h, framework, aggregator_key)
}

/// Aggregator v1 reads materialize the value and therefore write the underlying aggregator key;
/// the key must not be promoted separately by the block epilogue.
#[test]
fn test_aggregator_v1_materialized_read_excludes_promotion() {
    let (mut h, framework, aggregator_key) = setup_aggregator_v1();
    let txns = vec![Transaction::UserTransaction(aggregator::materialize(
        &mut h, &framework, 0,
    ))];

    let (statuses, promoted, written) = promotions(&h, txns);
    assert_all_success(&statuses);

    assert!(
        written.contains(&aggregator_key),
        "materializing an aggregator v1 read must write the aggregator key",
    );
    assert!(
        !promoted.contains(&aggregator_key),
        "a materialized aggregator v1 read must not also promote the written key",
    );
}

/// Aggregator v1 deltas count as writes, so delta-written aggregator keys are excluded from
/// promotion.
#[test]
fn test_aggregator_v1_delta_write_excludes_promotion() {
    let (mut h, framework, aggregator_key) = setup_aggregator_v1();
    let txns = vec![Transaction::UserTransaction(aggregator::add(
        &mut h, &framework, 0, 1,
    ))];

    let (statuses, promoted, written) = promotions(&h, txns);
    assert_all_success(&statuses);

    assert!(
        written.contains(&aggregator_key),
        "aggregator v1 add must write the aggregator key",
    );
    assert!(
        !promoted.contains(&aggregator_key),
        "an aggregator v1 key written by a delta in the block must not be promoted",
    );
}

/// Mutating an aggregator-v2 field records an in-place delayed-field write to the enclosing
/// resource slot. `storage_keys_written` enumerates those writes (unlike the old summary-based
/// derivation), so the slot — though read to load the resource — must not be promoted.
#[test]
fn test_delayed_field_write_excludes_promotion() {
    let (mut h, owner) = setup();
    // Give the owner a `Counter` (aggregator-v2 field) in its own applied transaction.
    assert_success!(h.run_entry_function(&owner, helper_fn("init_counter"), vec![], vec![]));
    let bumper = h.new_account_with_key_pair();
    let txns = vec![read_txn(&mut h, &bumper, "bump_counter", owner.address())];

    let (statuses, promoted, written) = promotions(&h, txns);
    assert_all_success(&statuses);

    let counter = StateKey::resource(owner.address(), &helper_struct("Counter")).unwrap();
    assert!(
        written.contains(&counter),
        "mutating the aggregator must write the Counter slot",
    );
    assert!(
        !promoted.contains(&counter),
        "a slot written via an in-place delayed-field change must not be promoted",
    );
}

/// Modules executed by a transaction are reads; ones not (re)published in the block are promoted.
/// Covers both the user module served from the state view and a framework module served from the
/// global module cache (recorded via `ReadRecordingCodeStorage`).
#[test]
fn test_module_reads_are_promoted() {
    let (mut h, owner) = setup();
    let reader = h.new_account_with_key_pair();
    let txns = vec![read_txn(&mut h, &reader, "read_only", owner.address())];

    let (statuses, promoted, _written) = promotions(&h, txns);
    assert_all_success(&statuses);

    // User module, served from the state view.
    assert!(promoted.contains(&StateKey::module(
        &helper_address(),
        ident_str!("read_helper")
    )));
    // Framework module, served from the global module cache.
    assert!(promoted.contains(&StateKey::module(&AccountAddress::ONE, ident_str!("coin"))));
}

/// A script transaction's declared module dependencies are recorded as reads and promoted. This
/// exercises the recording hook in `validate_and_execute_script`, which records the script's
/// immediate dependencies directly from the loaded script rather than from the loader's
/// verified-script-cache-gated dependency fetches — the property that keeps the promotion set
/// independent of execution schedule. (A block test can't force the committing incarnation to hit
/// a warm script cache, so this asserts the deps are recorded and promoted and that sequential and
/// parallel agree, not the schedule-independence directly.)
#[test]
fn test_script_module_dependencies_are_promoted() {
    let (mut h, owner) = setup();
    let reader = h.new_account_with_key_pair();

    let package = BuiltPackage::build(
        common::test_dir_path("hot_state.data/pack"),
        BuildOptions::move_2().set_latest_language(),
    )
    .expect("package must build");
    let script = package
        .extract_script_code()
        .pop()
        .expect("read_script must exist");
    let script_txn = Transaction::UserTransaction(h.create_script(&reader, script, vec![], vec![
        TransactionArgument::Address(*owner.address()),
    ]));

    let (statuses, promoted, written) = promotions(&h, vec![script_txn]);
    assert_all_success(&statuses);

    // The user module and framework module the script directly depends on are read (not written),
    // so both must be promoted.
    let read_helper = StateKey::module(&helper_address(), ident_str!("read_helper"));
    let coin = StateKey::module(&AccountAddress::ONE, ident_str!("coin"));
    assert!(
        promoted.contains(&read_helper),
        "the script's user module dependency must be promoted",
    );
    assert!(
        promoted.contains(&coin),
        "the script's framework module dependency must be promoted",
    );
    // The plain resource the script reads via the helper must also be promoted.
    let plain = StateKey::resource(owner.address(), &helper_struct("Plain")).unwrap();
    assert!(
        promoted.contains(&plain),
        "the read resource must be promoted"
    );

    for key in [read_helper, coin, plain] {
        assert!(!written.contains(&key));
    }
}

/// A module (re)published in the block is written, so it must be excluded from promotion even when
/// another transaction in the same block executes (reads) it. Mirrors `test_module_reads_are_promoted`,
/// where the same module — only read — is promoted.
#[test]
fn test_republished_module_not_promoted() {
    let (mut h, publisher) = new_harness_with_package();
    let owner = h.new_account_with_key_pair();
    assert_success!(h.run_entry_function(&owner, helper_fn("init"), vec![], vec![]));
    let reader = h.new_account_with_key_pair();

    // One txn executes `read_helper` (recording its module as a read); a later txn republishes the
    // package (a compatible upgrade, writing every module slot). The module is read and written in
    // the same block.
    let republish = h.create_publish_package(
        &publisher,
        &common::test_dir_path("hot_state.data/pack"),
        None,
        |_| {},
    );
    let txns = vec![
        read_txn(&mut h, &reader, "read_plain", owner.address()),
        Transaction::UserTransaction(republish),
    ];

    let (statuses, promoted, written) = promotions(&h, txns);
    assert_all_success(&statuses);

    let module = StateKey::module(&helper_address(), ident_str!("read_helper"));
    assert!(
        written.contains(&module),
        "republishing must write the module slot",
    );
    assert!(
        !promoted.contains(&module),
        "a module written in the block must not be promoted, even though another txn read it",
    );
}

/// On-chain configs read during the prologue/execution are promoted when the block does not write
/// them.
#[test]
fn test_config_reads_are_promoted() {
    let (mut h, owner) = setup();
    let reader = h.new_account_with_key_pair();
    let txns = vec![read_txn(&mut h, &reader, "read_only", owner.address())];

    let (statuses, promoted, written) = promotions(&h, txns);
    assert_all_success(&statuses);

    for key in [
        StateKey::on_chain_config::<ChainId>().unwrap(),
        StateKey::on_chain_config::<CurrentTimeMicroseconds>().unwrap(),
    ] {
        assert!(promoted.contains(&key), "config {:?} must be promoted", key);
        assert!(!written.contains(&key));
    }
}

/// A key both read and written by the same transaction is excluded from promotion.
#[test]
fn test_written_key_in_same_txn_not_promoted() {
    let (mut h, owner) = setup();
    // `write_plain` reads (via `exists`) and then mutates the caller's own `Plain`.
    let txns = vec![Transaction::UserTransaction(h.create_entry_function(
        &owner,
        helper_fn("write_plain"),
        vec![],
        vec![],
    ))];

    let (statuses, promoted, written) = promotions(&h, txns);
    assert_all_success(&statuses);

    let plain = StateKey::resource(owner.address(), &helper_struct("Plain")).unwrap();
    assert!(
        written.contains(&plain),
        "the test must actually write the key"
    );
    assert!(
        !promoted.contains(&plain),
        "a key written in the block must not be promoted",
    );
}

/// Block-level exclusion: a key read by one transaction but written by another in the same block is
/// excluded from promotion.
#[test]
fn test_key_written_by_another_txn_not_promoted() {
    let (mut h, owner) = setup();
    let reader = h.new_account_with_key_pair();
    let txns = vec![
        read_txn(&mut h, &reader, "read_plain", owner.address()),
        Transaction::UserTransaction(h.create_entry_function(
            &owner,
            helper_fn("write_plain"),
            vec![],
            vec![],
        )),
    ];

    let (statuses, promoted, written) = promotions(&h, txns);
    assert_all_success(&statuses);

    let plain = StateKey::resource(owner.address(), &helper_struct("Plain")).unwrap();
    assert!(written.contains(&plain));
    assert!(
        !promoted.contains(&plain),
        "a key written by any txn in the block must not be promoted, even if another txn read it",
    );
}

/// A discarded transaction commits no state changes, so the keys it read during its aborted
/// prologue must not be promoted to hot state. Exercises both the parallel commit path (read set
/// dropped in the VM wrapper) and the sequential one (the accumulator is fed only past the
/// bcs-fallback discard point).
#[test]
fn test_discarded_txn_reads_not_promoted() {
    let mut h = MoveHarness::new();
    let alice = h.new_account_with_key_pair();
    let bob = h.new_account_with_key_pair();
    // Dave appears only in the transaction we force to discard, so his account resource is read by
    // the aborted prologue but written by nothing in the block.
    let dave = h.new_account_with_key_pair();

    // A committed transfer, so the block has a real promotion set.
    let good = Transaction::UserTransaction(h.create_transaction_payload(
        &alice,
        aptos_stdlib::aptos_account_transfer(*bob.address(), 100),
    ));
    // Sequence number one ahead of the account's: the prologue reads Dave's account resource, then
    // aborts with SEQUENCE_NUMBER_TOO_NEW, discarding the transaction.
    let discarded = Transaction::UserTransaction(
        h.create_transaction_without_sign(
            &dave,
            aptos_stdlib::aptos_account_transfer(*bob.address(), 1),
        )
        .sequence_number(1)
        .sign(),
    );
    let dave_account = StateKey::resource_typed::<AccountResource>(dave.address()).unwrap();

    let (statuses, promoted, written) = promotions(&h, vec![good, discarded]);
    // Guard against a vacuous test: the block must actually contain a discard.
    assert!(
        statuses
            .iter()
            .any(|status| matches!(status, TransactionStatus::Discard(_))),
        "expected a discarded transaction, got {:?}",
        statuses,
    );
    assert!(
        !promoted.contains(&dave_account),
        "a discarded transaction's reads must not be promoted",
    );
    // A discard writes nothing, so the key is absent on its own merits, not because it was written.
    assert!(!written.contains(&dave_account));
}

/// Whole-block smoke test: mixed conflicting transfers plus a read-only call, asserting the broad
/// invariant that every promoted key was read but not written, that reads of each kind surface, and
/// that sequential and parallel execution agree.
#[test]
fn test_block_promotions_cover_reads_and_exclude_writes() {
    let (mut h, _publisher) = new_harness_with_package();
    let alice = h.new_account_with_key_pair();
    let bob = h.new_account_with_key_pair();
    // Charlie signs nothing in the block below, so nothing writes his account resource and the
    // helper's read of it must surface as a promotion.
    let charlie = h.new_account_with_key_pair();

    let txns = vec![
        Transaction::UserTransaction(h.create_transaction_payload(
            &alice,
            aptos_stdlib::aptos_account_transfer(*bob.address(), 100),
        )),
        Transaction::UserTransaction(h.create_transaction_payload(
            &bob,
            aptos_stdlib::aptos_account_transfer(*alice.address(), 50),
        )),
        read_txn(&mut h, &alice, "read_only", charlie.address()),
    ];

    let (statuses, promoted, written) = promotions(&h, txns);
    assert_all_success(&statuses);

    // Modules executed by the transactions are read but not written in this block, so they must be
    // promoted (a framework module and the user-published one).
    assert!(promoted.contains(&StateKey::module(
        &AccountAddress::ONE,
        ident_str!("aptos_account")
    )));
    assert!(promoted.contains(&StateKey::module(
        &helper_address(),
        ident_str!("read_helper")
    )));

    // The following are read but never written in this block, so they must be promoted.
    for key in [
        StateKey::on_chain_config::<ChainId>().unwrap(),
        StateKey::on_chain_config::<CurrentTimeMicroseconds>().unwrap(),
        StateKey::resource_typed::<AccountResource>(charlie.address()).unwrap(),
        StateKey::resource_typed::<CoinInfoResource<AptosCoinType>>(&AccountAddress::ONE).unwrap(),
    ] {
        assert!(promoted.contains(&key), "Expected promotion for {:?}", key);
    }

    // The coin-to-fungible-asset conversion map is a table keyed by coin type, read for paired
    // metadata lookups but never written in this block, so a table item must be promoted.
    assert!(
        promoted
            .iter()
            .any(|key| matches!(key.inner(), StateKeyInner::TableItem { .. })),
        "Expected the coin conversion map table item to be promoted",
    );

    // Keys written in the block become hot at the version they are written, so promoting them again
    // is redundant and they must not show up.
    let promoted_and_written: Vec<_> = promoted.intersection(&written).collect();
    assert!(
        promoted_and_written.is_empty(),
        "Promoted keys also written in the block: {:?}",
        promoted_and_written,
    );
}
