// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Block-level differential tests for the MonoMove Block-STM integration.
//!
//! Each test runs the same block twice through `FakeExecutor::execute_block`,
//! which routes through `AptosVMBlockExecutorWrapper` and thus selects MonoMove
//! when `ENABLE_MONO_MOVE` is on: once with the legacy VM (reference), once with
//! MonoMove on the same starting account state. The block executor forces
//! sequential execution for MonoMove (its milestone-1 scope), so this exercises
//! the sequential Block-STM path end to end.
//!
//! MonoMove runs without gas metering, so it charges no transaction fee. It
//! reports zero gas and does not write the fee-only slots: the APT supply, and
//! the fee payer's store when the transaction makes no other change to it. The
//! legacy reference charges a fee, so those slots are legacy-only or differ in
//! content, and legacy emits fee events MonoMove does not. Every other write
//! must match byte for byte, and non-fee events must match in sequence and
//! payload.

use crate::{tests::common, MoveHarness};
use aptos_language_e2e_tests::{
    account::{Account, AccountData},
    executor::{ExecutorMode, FakeExecutor},
};
use aptos_types::{
    on_chain_config::{FeatureFlag, Features, OnChainConfig},
    state_store::state_key::StateKey,
    transaction::{ExecutionStatus, SignedTransaction, TransactionOutput, TransactionStatus},
    write_set::TransactionWrite,
};
use move_core_types::account_address::AccountAddress;
use std::{collections::BTreeMap, str::FromStr};

/// Event types whose payload embeds gas amounts.
const GAS_DEPENDENT_EVENTS: &[&str] = &[
    "0x1::transaction_fee::FeeStatement",
    "0x1::fungible_asset::Withdraw",
    "0x1::coin::CoinWithdraw",
];

/// A funded sender and receiver on a fresh genesis state.
fn setup() -> (FakeExecutor, AccountData, AccountData) {
    // Sequential only: MonoMove forces sequential in the block executor, and the
    // legacy reference should run the same way for a faithful comparison.
    let mut fx = FakeExecutor::from_head_genesis().set_executor_mode(ExecutorMode::SequentialOnly);
    let alice = fx.create_raw_account_data(1_000_000_000, 10);
    fx.add_account_data(&alice);
    let bob = fx.create_raw_account_data(100_000_000, 0);
    fx.add_account_data(&bob);
    (fx, alice, bob)
}

/// A transfer of `amount` from `alice` to `bob` at sequence number `seq`.
fn transfer(alice: &AccountData, bob: &AccountData, seq: u64, amount: u64) -> SignedTransaction {
    alice
        .account()
        .transaction()
        .payload(aptos_cached_packages::aptos_stdlib::aptos_account_transfer(
            *bob.address(),
            amount,
        ))
        .sequence_number(seq)
        .gas_unit_price(100)
        .max_gas_amount(1_000_000)
        .sign()
}

/// Runs `block` on the legacy VM (reference) and then, from the same account
/// state, on MonoMove, and returns both output vectors. Asserts MonoMove was
/// actually selected for the second run.
fn run_both(fx: &mut FakeExecutor, block: Vec<SignedTransaction>) -> BothOutputs {
    let v1 = fx
        .execute_block(block.clone())
        .expect("legacy block executes");

    fx.enable_features(
        &AccountAddress::ONE,
        vec![FeatureFlag::ENABLE_MONO_MOVE],
        vec![],
    );
    let features = Features::fetch_config(fx.get_state_view())
        .ok()
        .flatten()
        .unwrap_or_default();
    assert!(
        features.is_mono_move_enabled(),
        "ENABLE_MONO_MOVE did not take effect; the mono path would not run"
    );

    let v2 = fx.execute_block(block).expect("mono block executes");
    assert_eq!(
        v1.len(),
        v2.len(),
        "the two runs produced different output counts"
    );
    BothOutputs { v1, v2 }
}

struct BothOutputs {
    v1: Vec<TransactionOutput>,
    v2: Vec<TransactionOutput>,
}

/// A block of one transfer: validates the sequential Block-STM plumbing (execute
/// -> apply -> materialize) end to end for the mono path.
#[test]
fn single_transfer_block_matches_v1() {
    let (mut fx, alice, bob) = setup();
    let block = vec![transfer(&alice, &bob, 10, 1_000)];

    let both = run_both(&mut fx, block);
    assert_kept_success(&both.v1, "legacy");
    assert_kept_success(&both.v2, "mono");
    for (i, (o1, o2)) in both.v1.iter().zip(&both.v2).enumerate() {
        compare_outputs(o1, o2, *alice.address(), i);
    }
}

/// A block of three dependent transfers from the same sender: each transaction's
/// prologue only passes if it observes the prior one's sequence-number bump, and
/// its balance debit only succeeds against the prior one's store. This exercises
/// cross-transaction reads through the multi-version map (own-slot account
/// resource and the fungible-store resource group).
#[test]
fn dependent_transfers_block_matches_v1() {
    let (mut fx, alice, bob) = setup();
    let block: Vec<_> = (10..13)
        .map(|seq| transfer(&alice, &bob, seq, 1_000))
        .collect();

    let both = run_both(&mut fx, block);
    assert_kept_success(&both.v1, "legacy");
    assert_kept_success(&both.v2, "mono");
    for (i, (o1, o2)) in both.v1.iter().zip(&both.v2).enumerate() {
        compare_outputs(o1, o2, *alice.address(), i);
    }
}

/// A block whose transfer targets an account that does not yet exist: the mono
/// path must create the recipient's primary-store object group for the first
/// time. Exercises the block-STM group path where the group is *born* in the
/// block -- no stored blob exists, so `group_members` reports no stored members
/// and materialize reassembles the transaction's own member ops as a group
/// creation.
#[test]
fn transfer_to_fresh_recipient_block_matches_v1() {
    use move_core_types::language_storage::StructTag;

    let (mut fx, alice, _bob) = setup();
    // An address with no account, so the transfer must create the recipient's
    // primary store from scratch.
    let fresh = AccountAddress::from_hex_literal("0xf00dcafe").unwrap();
    let txn = alice
        .account()
        .transaction()
        .payload(aptos_cached_packages::aptos_stdlib::aptos_account_transfer(
            fresh, 1_000,
        ))
        .sequence_number(10)
        .gas_unit_price(100)
        .max_gas_amount(1_000_000)
        .sign();

    let both = run_both(&mut fx, vec![txn]);
    assert_kept_success(&both.v1, "legacy");
    assert_kept_success(&both.v2, "mono");
    compare_outputs(&both.v1[0], &both.v2[0], *alice.address(), 0);

    // The recipient's object group must be freshly created (not modified), so
    // the test actually exercises the group-slot creation path.
    let object_group = StructTag::from_str("0x1::object::ObjectGroup").unwrap();
    let recipient_store = aptos_types::account_config::fungible_store::primary_apt_store(fresh);
    let recipient_group = StateKey::resource_group(&recipient_store, &object_group);
    let v2_writes: BTreeMap<_, _> = both.v2[0].write_set().write_op_iter().collect();
    let op = v2_writes
        .get(&recipient_group)
        .expect("mono must write the recipient's object group");
    assert!(
        op.is_creation(),
        "recipient object group must be a creation, was {:?}",
        op.write_op_kind()
    );
}

/// A block that adds a new member to an existing resource group and then, in a
/// later transaction, modifies a *different* member of the same group. The
/// second transaction's emitted group blob must include the member the first
/// added. Seeding the blob from storage alone would drop it, so this exercises
/// the running merged group state that accumulates across transactions in the
/// block. Uses the `0x1::resource_groups_test` module, whose `MyGroup` has four
/// independent members, so a block can grow the group without a fresh account.
#[test]
fn group_member_added_then_sibling_modified_matches_v1() {
    let executor =
        FakeExecutor::from_head_genesis().set_executor_mode(ExecutorMode::SequentialOnly);
    let mut harness = MoveHarness::new_with_executor(executor);

    // Publish the resource-group test module at its declared address, create the
    // resource account that owns the group, and store one member so the group
    // already exists before the measured block.
    let publisher = harness.new_account_at(AccountAddress::ONE);
    let path = common::test_dir_path("resource_groups.data/pack");
    assert_eq!(
        harness.publish_package(&publisher, &path),
        TransactionStatus::Keep(ExecutionStatus::Success),
    );
    let owner = *publisher.address();
    let init = harness.create_entry_function(
        &publisher,
        str::parse("0x1::resource_groups_test::init_signer").unwrap(),
        vec![],
        vec![bcs::to_bytes(&b"mono-group".to_vec()).unwrap()],
    );
    let seed_member = set_resource(&mut harness, &publisher, owner, 1, "base", 1);
    for status in harness.run_block(vec![init, seed_member]) {
        assert_eq!(status, TransactionStatus::Keep(ExecutionStatus::Success));
    }

    // A funded sender for the two measured transactions. They are dependent (same
    // sender, consecutive sequence numbers), so they commit in order: transaction
    // 0 adds member 2, then transaction 1 modifies member 1.
    let user = Account::new();
    harness.store_and_fund_account(&user, 1_000_000_000_000, 0);
    let add_member = set_resource(&mut harness, &user, owner, 2, "added", 2);
    let modify_sibling = set_resource(&mut harness, &user, owner, 1, "changed", 3);
    let block = vec![add_member, modify_sibling];

    let fx = &mut harness.executor;
    let v1 = fx
        .execute_block(block.clone())
        .expect("legacy block executes");
    fx.enable_features(
        &AccountAddress::ONE,
        vec![FeatureFlag::ENABLE_MONO_MOVE],
        vec![],
    );
    let features = Features::fetch_config(fx.get_state_view())
        .ok()
        .flatten()
        .unwrap_or_default();
    assert!(
        features.is_mono_move_enabled(),
        "ENABLE_MONO_MOVE did not take effect; the mono path would not run"
    );
    let v2 = fx.execute_block(block).expect("mono block executes");
    assert_eq!(v1.len(), v2.len(), "the two runs produced different counts");

    assert_kept_success(&v1, "legacy");
    assert_kept_success(&v2, "mono");

    // The group is owned by the resource account `init_signer` derived from 0x1
    // with the seed. `set_resource` also mut-borrows `MainResource` without
    // changing it; mono emits that unchanged copy as a write while legacy elides
    // it, so a whole-output diff would flag noise unrelated to the group. Assert
    // on the group blob directly instead.
    use move_core_types::language_storage::StructTag;
    let owner =
        aptos_types::account_address::create_resource_address(AccountAddress::ONE, b"mono-group");
    let my_group = StructTag::from_str("0x1::resource_groups_test::MyGroup").unwrap();
    let group_key = StateKey::resource_group(&owner, &my_group);

    // The group write is not fee dependent, so legacy and mono must emit a
    // byte-identical blob for each transaction.
    for (i, (o1, o2)) in v1.iter().zip(&v2).enumerate() {
        let g1 = group_write(o1, &group_key);
        let g2 = group_write(o2, &group_key);
        assert_eq!(
            g1.write_op_kind(),
            g2.write_op_kind(),
            "transaction {i}: group op kind differs"
        );
        assert_eq!(
            g1.bytes(),
            g2.bytes(),
            "transaction {i}: group blob differs"
        );
    }

    // Transaction 1 modifies member 1; its blob must still carry member 2 that
    // transaction 0 added. Seeding from storage alone would drop it -- only the
    // running merged group state, seeded once and mutated across transactions,
    // preserves it.
    let members = decode_group(group_write(&v2[1], &group_key));
    let member_1 = StructTag::from_str("0x1::resource_groups_test::MyResource1").unwrap();
    let member_2 = StructTag::from_str("0x1::resource_groups_test::MyResource2").unwrap();
    assert!(
        members.contains_key(&member_1),
        "transaction 1 group blob is missing the modified member 1"
    );
    assert!(
        members.contains_key(&member_2),
        "transaction 1 group blob is missing member 2 added by transaction 0"
    );
}

/// The write op for the resource group at `key` in `output`, panicking if the
/// transaction did not write it.
fn group_write<'a>(
    output: &'a TransactionOutput,
    key: &StateKey,
) -> &'a aptos_types::write_set::WriteOp {
    output
        .write_set()
        .write_op_iter()
        .find(|(k, _)| *k == key)
        .map(|(_, op)| op)
        .unwrap_or_else(|| panic!("group write {key:?} missing from output"))
}

/// Decodes a resource-group blob into its member map. Only the member tags are
/// inspected, so the values are kept opaque.
fn decode_group(
    op: &aptos_types::write_set::WriteOp,
) -> BTreeMap<move_core_types::language_storage::StructTag, Vec<u8>> {
    let bytes = op.bytes().expect("group write has no bytes");
    bcs::from_bytes(bytes).expect("group blob decodes")
}

/// Builds a `set_resource` transaction targeting the group owned by the resource
/// account of `main_account`: member `index` is created if absent, else updated.
fn set_resource(
    harness: &mut MoveHarness,
    sender: &Account,
    main_account: AccountAddress,
    index: u32,
    name: &str,
    value: u32,
) -> SignedTransaction {
    harness.create_entry_function(
        sender,
        str::parse("0x1::resource_groups_test::set_resource").unwrap(),
        vec![],
        vec![
            bcs::to_bytes(&main_account).unwrap(),
            bcs::to_bytes(&index).unwrap(),
            bcs::to_bytes(&name.to_string()).unwrap(),
            bcs::to_bytes(&value).unwrap(),
        ],
    )
}

fn assert_kept_success(outputs: &[TransactionOutput], label: &str) {
    for (i, output) in outputs.iter().enumerate() {
        assert_eq!(
            output.status(),
            &TransactionStatus::Keep(ExecutionStatus::Success),
            "{label} transaction {i} was not kept-success: {:?}",
            output.status()
        );
    }
}

/// Compares one transaction's outputs from the two VMs. MonoMove charges no fee,
/// so it may omit the fee-only slots and reports zero gas; everything else must
/// match. Adapted from the single-transaction differential test.
fn compare_outputs(
    v1: &TransactionOutput,
    v2: &TransactionOutput,
    fee_payer: AccountAddress,
    idx: usize,
) {
    let v1_writes: BTreeMap<_, _> = v1.write_set().write_op_iter().collect();
    let v2_writes: BTreeMap<_, _> = v2.write_set().write_op_iter().collect();

    // MonoMove charges no fee, so it never writes the fee-only slots (the APT
    // supply, and the fee payer's store when the transaction otherwise leaves it
    // untouched). A key legacy wrote that mono did not must be one of these.
    let fee_related = fee_related_keys(fee_payer);
    for key in v1_writes.keys() {
        if !v2_writes.contains_key(key) {
            assert!(
                fee_related.contains(key),
                "transaction {idx}: legacy wrote {key:?} but mono did not, and it is not fee-only"
            );
        }
    }
    // Mono must not invent writes the legacy VM does not produce.
    for key in v2_writes.keys() {
        assert!(
            v1_writes.contains_key(key),
            "transaction {idx}: mono wrote {key:?} but legacy did not"
        );
    }

    // Shared keys: the write-op kind must match, and the bytes must match unless
    // the slot embeds the fee. For a resource group the kind is the slot's
    // create-vs-modify decision, reassembled from the map's per-member entries
    // without a pre-image snapshot.
    let mut unexplained = vec![];
    for (key, v1_op) in &v1_writes {
        let Some(v2_op) = v2_writes.get(*key) else {
            continue;
        };
        assert_eq!(
            v1_op.write_op_kind(),
            v2_op.write_op_kind(),
            "transaction {idx}: write op kind differs at {key:?}"
        );
        if v1_op.bytes() == v2_op.bytes() || fee_related.contains(key) {
            continue;
        }
        unexplained.push(format!(
            "{key:?}:\n  v1: {:?}\n  v2: {:?}",
            v1_op.bytes(),
            v2_op.bytes()
        ));
    }
    assert!(
        unexplained.is_empty(),
        "transaction {idx}: writes differ beyond fee-only slots:\n{}",
        unexplained.join("\n")
    );
    // Legacy always meters; mono never does. The gas split also proves the mono
    // path actually ran -- a silent fallback to legacy would report equal gas.
    assert!(
        v1.gas_used() > 0,
        "transaction {idx}: legacy charged no gas"
    );
    assert_eq!(
        v2.gas_used(),
        0,
        "transaction {idx}: mono metered (expected no metering)"
    );

    // Events: mono emits no fee events, so filter the gas-dependent types from
    // both sides and require the remainder to match in sequence and payload.
    let v1_events: Vec<_> = v1.events().iter().filter(|e| !is_gas_event(e)).collect();
    let v2_events: Vec<_> = v2.events().iter().filter(|e| !is_gas_event(e)).collect();
    let v1_types: Vec<_> = v1_events
        .iter()
        .map(|e| format!("{:?}", e.type_tag()))
        .collect();
    let v2_types: Vec<_> = v2_events
        .iter()
        .map(|e| format!("{:?}", e.type_tag()))
        .collect();
    assert_eq!(
        v1_types, v2_types,
        "transaction {idx}: non-fee event sequences differ"
    );
    for (e1, e2) in v1_events.iter().zip(&v2_events) {
        assert_eq!(
            e1.event_data(),
            e2.event_data(),
            "transaction {idx}: event payload differs for {:?}",
            e1.type_tag()
        );
    }
}

/// Whether an event's payload embeds gas amounts (so mono, charging no fee, does
/// not emit it).
fn is_gas_event(event: &aptos_types::contract_event::ContractEvent) -> bool {
    GAS_DEPENDENT_EVENTS.contains(&event.type_tag().to_canonical_string().as_str())
}

/// The state keys whose content embeds the transaction fee: the fee payer's
/// primary fungible store group and the APT metadata object group (supply).
fn fee_related_keys(fee_payer: AccountAddress) -> Vec<StateKey> {
    use move_core_types::language_storage::StructTag;

    let object_group = StructTag::from_str("0x1::object::ObjectGroup").unwrap();
    let store = aptos_types::account_config::fungible_store::primary_apt_store(fee_payer);
    vec![
        StateKey::resource_group(&store, &object_group),
        StateKey::resource_group(&AccountAddress::TEN, &object_group),
    ]
}
