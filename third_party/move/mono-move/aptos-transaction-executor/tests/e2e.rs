// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! End-to-end differential test: a p2p transfer executed by the legacy
//! AptosVM (via `FakeExecutor`) and by the MonoMove-backed transaction
//! executor against the same starting state.
//!
//! Gas amounts intentionally differ between the two VMs, so writes that embed
//! the fee (the fee payer's fungible store, the APT supply) and fee events are
//! allowed to differ in *content*; everything else must match byte-for-byte.
//
// TODO(testing): revisit once the executor is more mature/wired up. See if we
// want to switch to other payloads that are less gas-dependent, or get this
// covered by other tests, such as the e2e move tests. Note that the sender's
// store is masked, so a wrong debit there is not caught.

use aptos_language_e2e_tests::executor::FakeExecutor;
use aptos_types::{
    on_chain_config::{Features, OnChainConfig},
    state_store::StateView,
    transaction::{
        ExecutionStatus, SignedTransaction, TransactionAuxiliaryData, TransactionOutput,
        TransactionStatus,
    },
};
use mono_move_aptos_state_view_providers::{
    StateViewModuleProvider, StateViewResourceProvider, DEFAULT_RESOURCE_ARENA_BYTES,
};
use mono_move_aptos_transaction_executor::{production_natives, AptosTransactionExecutor};
use mono_move_global_context::GlobalContext;
use std::collections::BTreeMap;

/// Event types whose payload embeds gas amounts.
const GAS_DEPENDENT_EVENTS: &[&str] = &[
    "0x1::transaction_fee::FeeStatement",
    "0x1::fungible_asset::Withdraw",
    "0x1::coin::CoinWithdraw",
];

/// Runs one transaction through the MonoMove executor against `state`,
/// materialized into the legacy output formats.
fn execute_v2<S: StateView>(state: &S, txn: &SignedTransaction) -> TransactionOutput {
    let global_ctx = GlobalContext::with_num_execution_workers(1);
    let guard = global_ctx
        .try_execution_context(0)
        .expect("execution context is available");
    let natives = production_natives(&guard);
    let module_provider = StateViewModuleProvider::new(state);
    let data_provider = StateViewResourceProvider::new(&guard, state, DEFAULT_RESOURCE_ARENA_BYTES);
    let features = Features::fetch_config(state)
        .ok()
        .flatten()
        .unwrap_or_default();
    let usage = state.get_usage().expect("usage is readable");
    let executor = AptosTransactionExecutor::new(
        &guard,
        &natives,
        &module_provider,
        &data_provider,
        &features,
        usage,
    );
    executor
        .execute_user_transaction(txn)
        .materialize(
            &guard,
            &data_provider,
            &features,
            TransactionAuxiliaryData::default(),
        )
        .expect("the transaction output materializes")
}

#[test]
fn p2p_transfer_matches_v1() {
    let mut fx = FakeExecutor::from_head_genesis();
    let alice = fx.create_raw_account_data(1_000_000_000, 10);
    fx.add_account_data(&alice);
    let bob = fx.create_raw_account_data(100_000_000, 0);
    fx.add_account_data(&bob);

    let txn = alice
        .account()
        .transaction()
        .payload(aptos_cached_packages::aptos_stdlib::aptos_account_transfer(
            *bob.address(),
            1_000,
        ))
        .sequence_number(10)
        .gas_unit_price(100)
        .max_gas_amount(1_000_000)
        .sign();

    // V1 (reference), on the starting state.
    let v1_output = fx.execute_transaction(txn.clone());
    assert_eq!(
        v1_output.status(),
        &TransactionStatus::Keep(ExecutionStatus::Success),
        "v1 rejected the transfer: {:?}",
        v1_output.status()
    );

    // V2, on the same starting state.
    let v2_output = execute_v2(fx.get_state_view(), &txn);
    assert_eq!(
        v2_output.status(),
        &TransactionStatus::Keep(ExecutionStatus::Success),
        "v2 failed the transfer"
    );

    compare_outputs(&v1_output, &v2_output, *alice.address());
}

/// Compares the two outputs' write sets and events, masking only what gas
/// divergence explains.
fn compare_outputs(
    v1: &TransactionOutput,
    v2: &TransactionOutput,
    fee_payer: move_core_types::account_address::AccountAddress,
) {
    let v1_writes: BTreeMap<_, _> = v1.write_set().write_op_iter().collect();
    let v2_writes: BTreeMap<_, _> = v2.write_set().write_op_iter().collect();

    let v1_keys: Vec<_> = v1_writes.keys().collect();
    let v2_keys: Vec<_> = v2_writes.keys().collect();
    assert_eq!(
        v1_keys, v2_keys,
        "the two VMs wrote different sets of state keys"
    );

    // The only slots allowed to differ are the two that embed the fee: the
    // fee payer's primary store (its object group) and the APT metadata
    // object at 0xa (supply).
    let gas_dependent = gas_dependent_keys(fee_payer);
    let mut unexplained = vec![];
    let mut num_diffs = 0;
    for (key, v1_op) in &v1_writes {
        let v2_op = &v2_writes[*key];
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
        "writes differ beyond gas-dependent slots:\n{}",
        unexplained.join("\n")
    );
    // Both VMs must actually have charged a fee (otherwise the mask above is
    // vacuous and something upstream is broken).
    assert!(
        num_diffs >= 1,
        "no write differed; was a fee charged at all?"
    );
    assert!(v1.gas_used() > 0, "v1 charged no gas");
    assert!(v2.gas_used() > 0, "v2 charged no gas");

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
    assert_eq!(v1_types, v2_types, "event sequences differ");
    for (e1, e2) in v1_events.iter().zip(v2_events) {
        let ty = e1.type_tag().to_canonical_string();
        if GAS_DEPENDENT_EVENTS.contains(&ty.as_str()) {
            continue;
        }
        assert_eq!(
            e1.event_data(),
            e2.event_data(),
            "event payload differs for {ty}"
        );
    }
}

/// A transfer exceeding the sender's balance: the payload aborts inside
/// `0x1::fungible_asset`, the payload's effects roll back, and the failure
/// epilogue still charges the fee and bumps the sequence number.
#[test]
fn p2p_transfer_insufficient_balance_aborts_like_v1() {
    let mut fx = FakeExecutor::from_head_genesis();
    let alice = fx.create_raw_account_data(1_000_000_000, 10);
    fx.add_account_data(&alice);
    let bob = fx.create_raw_account_data(100_000_000, 0);
    fx.add_account_data(&bob);

    let txn = alice
        .account()
        .transaction()
        .payload(aptos_cached_packages::aptos_stdlib::aptos_account_transfer(
            *bob.address(),
            u64::MAX,
        ))
        .sequence_number(10)
        .gas_unit_price(100)
        .max_gas_amount(1_000_000)
        .sign();

    let v1_output = fx.execute_transaction(txn.clone());
    let TransactionStatus::Keep(ExecutionStatus::MoveAbort {
        code: v1_code,
        location: v1_location,
        ..
    }) = v1_output.status()
    else {
        panic!("v1 did not abort: {:?}", v1_output.status());
    };

    let v2_output = execute_v2(fx.get_state_view(), &txn);
    // TODO(correctness): compare the abort info too, once the executor resolves
    // it from the aborting module's metadata the way the legacy VM does.
    let TransactionStatus::Keep(ExecutionStatus::MoveAbort {
        code: v2_code,
        location: v2_location,
        ..
    }) = v2_output.status()
    else {
        panic!("v2 did not abort: {:?}", v2_output.status());
    };
    assert_eq!(v1_code, v2_code, "abort codes differ");
    assert_eq!(v1_location, v2_location, "abort locations differ");

    compare_outputs(&v1_output, &v2_output, *alice.address());
}

/// A transfer that drains the fee payer's entire balance: the payload
/// succeeds, but the success epilogue cannot collect the fee, so the payload
/// rolls back and the fee is charged from the restored balance. The
/// transaction is kept as the can't-pay-fee abort, matching the legacy VM's
/// failure cleanup — including the abort location.
#[test]
fn p2p_transfer_draining_fee_payer_aborts_like_v1() {
    let mut fx = FakeExecutor::from_head_genesis();
    let alice = fx.create_raw_account_data(1_000_000_000, 10);
    fx.add_account_data(&alice);
    let bob = fx.create_raw_account_data(100_000_000, 0);
    fx.add_account_data(&bob);

    // Send the entire balance: nothing is left for the fee at epilogue time.
    let txn = alice
        .account()
        .transaction()
        .payload(aptos_cached_packages::aptos_stdlib::aptos_account_transfer(
            *bob.address(),
            1_000_000_000,
        ))
        .sequence_number(10)
        .gas_unit_price(100)
        .max_gas_amount(1_000_000)
        .sign();

    let v1_output = fx.execute_transaction(txn.clone());
    let TransactionStatus::Keep(ExecutionStatus::MoveAbort {
        code: v1_code,
        location: v1_location,
        ..
    }) = v1_output.status()
    else {
        panic!("v1 did not abort: {:?}", v1_output.status());
    };

    let v2_output = execute_v2(fx.get_state_view(), &txn);
    let TransactionStatus::Keep(ExecutionStatus::MoveAbort {
        code: v2_code,
        location: v2_location,
        ..
    }) = v2_output.status()
    else {
        panic!("v2 did not abort: {:?}", v2_output.status());
    };
    assert_eq!(v1_code, v2_code, "abort codes differ");
    assert_eq!(v1_location, v2_location, "abort locations differ");

    compare_outputs(&v1_output, &v2_output, *alice.address());
}

/// A nonexistent entry function: kept (and charged) on both VMs, because the
/// function-values feature makes a missing function runtime-reachable. This
/// exercises the kept non-abort payload failure path: rollback plus failure
/// epilogue.
#[test]
fn nonexistent_entry_function_kept_like_v1() {
    use aptos_types::transaction::{EntryFunction, TransactionPayload};
    use move_core_types::{ident_str, language_storage::ModuleId};

    let mut fx = FakeExecutor::from_head_genesis();
    let alice = fx.create_raw_account_data(1_000_000_000, 10);
    fx.add_account_data(&alice);

    let txn = alice
        .account()
        .transaction()
        .payload(TransactionPayload::EntryFunction(EntryFunction::new(
            ModuleId::new(
                move_core_types::account_address::AccountAddress::ONE,
                ident_str!("coin").to_owned(),
            ),
            ident_str!("no_such_function").to_owned(),
            vec![],
            vec![],
        )))
        .sequence_number(10)
        .gas_unit_price(100)
        .max_gas_amount(1_000_000)
        .sign();

    let v1_output = fx.execute_transaction(txn.clone());
    assert!(
        matches!(
            v1_output.status(),
            TransactionStatus::Keep(ExecutionStatus::MiscellaneousError(_))
        ),
        "v1 did not keep as a miscellaneous error: {:?}",
        v1_output.status()
    );

    let v2_output = execute_v2(fx.get_state_view(), &txn);
    assert_eq!(v1_output.status(), v2_output.status());

    compare_outputs(&v1_output, &v2_output, *alice.address());
}

/// Two dependent transfers executed sequentially, each transaction's outputs
/// applied to the state before the next: the second transaction's prologue
/// only passes if it observes the first one's sequence-number bump.
#[test]
fn sequential_execution_applies_outputs() {
    use aptos_transaction_simulation::{DeltaStateStore, SimulationStateStore};

    let mut fx = FakeExecutor::from_head_genesis();
    let alice = fx.create_raw_account_data(1_000_000_000, 10);
    fx.add_account_data(&alice);
    let bob = fx.create_raw_account_data(100_000_000, 0);
    fx.add_account_data(&bob);

    let transfer = |seq| {
        alice
            .account()
            .transaction()
            .payload(aptos_cached_packages::aptos_stdlib::aptos_account_transfer(
                *bob.address(),
                1_000,
            ))
            .sequence_number(seq)
            .gas_unit_price(100)
            .max_gas_amount(1_000_000)
            .sign()
    };

    let state = DeltaStateStore::new_with_base(fx.get_state_view());
    for (i, txn) in [transfer(10), transfer(11)].iter().enumerate() {
        let output = execute_v2(&state, txn);
        assert_eq!(
            output.status(),
            &TransactionStatus::Keep(ExecutionStatus::Success),
            "transaction {i} failed"
        );
        state
            .apply_write_set(output.write_set())
            .expect("write set applies");
    }
}

/// The state keys whose content embeds the transaction fee: the fee payer's
/// primary fungible store group and the APT metadata object group (supply).
fn gas_dependent_keys(
    fee_payer: move_core_types::account_address::AccountAddress,
) -> Vec<aptos_types::state_store::state_key::StateKey> {
    use aptos_types::state_store::state_key::StateKey;
    use move_core_types::{account_address::AccountAddress, language_storage::StructTag};
    use std::str::FromStr;

    let object_group = StructTag::from_str("0x1::object::ObjectGroup").unwrap();
    let store = aptos_types::account_config::fungible_store::primary_apt_store(fee_payer);
    vec![
        StateKey::resource_group(&store, &object_group),
        StateKey::resource_group(&AccountAddress::TEN, &object_group),
    ]
}
