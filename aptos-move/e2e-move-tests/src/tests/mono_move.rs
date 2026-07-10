// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! End-to-end test for the mono VM as a Block-STM execution task: with
//! `FeatureFlag::MONO_MOVE` enabled, a block of hundreds of conflicting
//! entry-function transactions executes through the parallel block executor
//! on the mono path. Committed outputs are stubs (empty write sets, zero
//! gas) except for the emitted events, which are compared per transaction
//! against a legacy run of the same block over the same base state.

use crate::MoveHarness;
use aptos_language_e2e_tests::executor::{ExecutorMode, FakeExecutor};
use aptos_types::{
    account_address::AccountAddress,
    contract_event::ContractEvent,
    on_chain_config::FeatureFlag,
    transaction::{ExecutionStatus, SignedTransaction, TransactionStatus},
};
use move_core_types::language_storage::TypeTag;

/// Senders issuing transfers to fresh addresses: every transfer creates the
/// recipient account and its primary fungible store.
const NUM_CREATORS: usize = 10;
const CREATIONS_PER_CREATOR: usize = 10;
/// Senders all paying the same hot recipient: write-write conflicts on one
/// fungible store, plus each sender's own read-modify-write chain.
const NUM_SPAMMERS: usize = 10;
const TRANSFERS_PER_SPAMMER: usize = 20;

#[test]
fn mono_move_executes_block_with_creations_and_conflicts() {
    // The mono path supports parallel execution only; the executor's
    // parallel run uses its own concurrency level (num_cpus).
    let executor = FakeExecutor::from_head_genesis().set_executor_mode(ExecutorMode::ParallelOnly);
    let mut harness = MoveHarness::new_with_executor(executor);

    // Funded accounts are created while the legacy path is active (account
    // creation must actually change state).
    let hot_recipient = *harness.new_account_at(AccountAddress::random()).address();
    let creators = (0..NUM_CREATORS)
        .map(|_| harness.new_account_at(AccountAddress::random()))
        .collect::<Vec<_>>();
    let spammers = (0..NUM_SPAMMERS)
        .map(|_| harness.new_account_at(AccountAddress::random()))
        .collect::<Vec<_>>();

    // Per-sender transaction chains (in-block sequence-number order). Each
    // creator pays fresh random addresses (account + store creation); each
    // spammer repeatedly pays the hot recipient.
    let mut chains = vec![];
    for creator in &creators {
        chains.push(
            (0..CREATIONS_PER_CREATOR)
                .map(|_| transfer(&mut harness, creator, AccountAddress::random()))
                .collect::<Vec<_>>(),
        );
    }
    for spammer in &spammers {
        chains.push(
            (0..TRANSFERS_PER_SPAMMER)
                .map(|_| transfer(&mut harness, spammer, hot_recipient))
                .collect::<Vec<_>>(),
        );
    }

    // Interleave the chains round-robin, so conflicting transactions overlap
    // in the schedule instead of forming independent runs.
    let mut txns = vec![];
    let mut chains = chains.into_iter().map(Vec::into_iter).collect::<Vec<_>>();
    loop {
        let mut emitted = false;
        for chain in &mut chains {
            if let Some(txn) = chain.next() {
                txns.push(txn);
                emitted = true;
            }
        }
        if !emitted {
            break;
        }
    }
    let num_txns = NUM_CREATORS * CREATIONS_PER_CREATOR + NUM_SPAMMERS * TRANSFERS_PER_SPAMMER;
    assert_eq!(txns.len(), num_txns);

    // Reference run on the legacy VM, without applying its write sets: both
    // runs must see the same base state, or the creation transactions take
    // different paths (a created recipient no longer emits creation events).
    // The mono re-run has no prologue, so sequence numbers are not a concern.
    let legacy_outputs = harness.executor.execute_block(txns.clone()).unwrap();

    harness.enable_features(vec![FeatureFlag::MONO_MOVE], vec![]);

    let outputs = harness.executor.execute_block(txns).unwrap();
    assert_eq!(legacy_outputs.len(), num_txns);
    assert_eq!(outputs.len(), num_txns);

    let mut compared_events = 0;
    for (output, legacy_output) in outputs.iter().zip(&legacy_outputs) {
        // The workload must genuinely succeed on the legacy VM, or the
        // comparison proves nothing.
        assert_eq!(
            legacy_output.status(),
            &TransactionStatus::Keep(ExecutionStatus::Success)
        );

        // Stub outputs with real events: success status, no writes, no gas.
        // Zero gas also proves the block ran on the mono path — the legacy VM
        // always charges gas for a kept user transaction.
        assert_eq!(
            output.status(),
            &TransactionStatus::Keep(ExecutionStatus::Success)
        );
        assert!(output.write_set().is_empty());
        assert_eq!(output.gas_used(), 0);

        // The events must match the legacy run's, except those emitted by the
        // epilogue (fee statement), which the mono path does not execute.
        let legacy_events = legacy_output
            .events()
            .iter()
            .filter(|event| !is_fee_statement(event))
            .cloned()
            .collect::<Vec<_>>();
        assert!(!legacy_events.is_empty());
        assert_eq!(output.events(), &legacy_events);
        compared_events += legacy_events.len();
    }
    // Every transfer emits at least a withdraw and a deposit; creations more.
    assert!(compared_events >= 2 * num_txns);
}

fn transfer(
    harness: &mut MoveHarness,
    sender: &aptos_language_e2e_tests::account::Account,
    recipient: AccountAddress,
) -> SignedTransaction {
    harness.create_entry_function(
        sender,
        str::parse("0x1::aptos_account::transfer").unwrap(),
        vec![],
        vec![
            bcs::to_bytes(&recipient).unwrap(),
            bcs::to_bytes(&1u64).unwrap(),
        ],
    )
}

fn is_fee_statement(event: &ContractEvent) -> bool {
    let TypeTag::Struct(tag) = event.type_tag() else {
        return false;
    };
    tag.address == AccountAddress::ONE
        && tag.module.as_str() == "transaction_fee"
        && tag.name.as_str() == "FeeStatement"
}
