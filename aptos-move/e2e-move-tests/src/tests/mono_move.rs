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
//! Gas amounts differ between the two VMs, so writes that embed the fee (the fee
//! payer's fungible store, the APT supply) and fee events are allowed to differ
//! in content; everything else must match byte for byte. This mirrors the
//! single-transaction differential test in `mono-move-aptos-transaction-executor`.

use aptos_language_e2e_tests::{
    account::AccountData,
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

/// Compares one transaction's outputs from the two VMs, masking only what gas
/// divergence explains. Adapted from the single-transaction differential test.
fn compare_outputs(
    v1: &TransactionOutput,
    v2: &TransactionOutput,
    fee_payer: AccountAddress,
    idx: usize,
) {
    let v1_writes: BTreeMap<_, _> = v1.write_set().write_op_iter().collect();
    let v2_writes: BTreeMap<_, _> = v2.write_set().write_op_iter().collect();

    let v1_keys: Vec<_> = v1_writes.keys().collect();
    let v2_keys: Vec<_> = v2_writes.keys().collect();
    assert_eq!(
        v1_keys, v2_keys,
        "transaction {idx}: the two VMs wrote different sets of state keys"
    );

    let gas_dependent = gas_dependent_keys(fee_payer);
    let mut unexplained = vec![];
    let mut num_diffs = 0;
    for (key, v1_op) in &v1_writes {
        let v2_op = &v2_writes[*key];
        // The write-op kind (creation / modification / deletion) must match even
        // where the fee makes the bytes diverge. For a resource group this is
        // the slot's create-vs-modify decision, reassembled from the map's
        // per-member entries without a pre-image snapshot.
        assert_eq!(
            v1_op.write_op_kind(),
            v2_op.write_op_kind(),
            "transaction {idx}: write op kind differs at {key:?}"
        );
        if v1_op.bytes() == v2_op.bytes() {
            continue;
        }
        num_diffs += 1;
        if !gas_dependent.contains(key) {
            unexplained.push(format!(
                "{key:?}:\n  v1: {:?}\n  v2: {:?}",
                v1_op.bytes(),
                v2_op.bytes()
            ));
        }
    }
    assert!(
        unexplained.is_empty(),
        "transaction {idx}: writes differ beyond gas-dependent slots:\n{}",
        unexplained.join("\n")
    );
    // If the fee were not charged (or the mono path silently fell back to
    // legacy), nothing would differ and the mask would be vacuous.
    assert!(
        num_diffs >= 1,
        "transaction {idx}: no write differed; did the mono path run and charge a fee?"
    );
    assert!(
        v1.gas_used() > 0,
        "transaction {idx}: legacy charged no gas"
    );
    assert!(v2.gas_used() > 0, "transaction {idx}: mono charged no gas");

    // Events: same sequence of types; payloads equal except gas-dependent ones.
    let v1_events = v1.events();
    let v2_events = v2.events();
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
        "transaction {idx}: event sequences differ"
    );
    for (e1, e2) in v1_events.iter().zip(v2_events) {
        let ty = e1.type_tag().to_canonical_string();
        if GAS_DEPENDENT_EVENTS.contains(&ty.as_str()) {
            continue;
        }
        assert_eq!(
            e1.event_data(),
            e2.event_data(),
            "transaction {idx}: event payload differs for {ty}"
        );
    }
}

/// The state keys whose content embeds the transaction fee: the fee payer's
/// primary fungible store group and the APT metadata object group (supply).
fn gas_dependent_keys(fee_payer: AccountAddress) -> Vec<StateKey> {
    use move_core_types::language_storage::StructTag;

    let object_group = StructTag::from_str("0x1::object::ObjectGroup").unwrap();
    let store = aptos_types::account_config::fungible_store::primary_apt_store(fee_payer);
    vec![
        StateKey::resource_group(&store, &object_group),
        StateKey::resource_group(&AccountAddress::TEN, &object_group),
    ]
}
