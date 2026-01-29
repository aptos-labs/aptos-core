// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Tests for the pre-write optimization feature.
//!
//! Pre-writes allow the executor to pre-populate MVHashMap with expected writes before
//! parallel execution, reducing contention for frequently accessed resources like timestamps.
//!
//! These tests verify:
//! - Transactions with matching pre-writes and actual writes succeed
//! - Transactions without pre-writes succeed normally
//! - Transactions with pre-writes that don't produce matching outputs trigger fallback

use crate::{
    code_cache_global_manager::AptosModuleCacheManagerGuard,
    combinatorial_tests::{
        mock_executor::{MockEvent, MockOutput, MockTask},
        resource_tests::create_executor_thread_pool,
        types::{KeyType, MockIncarnation, MockTransaction, ValueType},
    },
    executor::BlockExecutor,
    txn_commit_hook::NoOpTransactionCommitHook,
    txn_provider::{default::DefaultTxnProvider, TxnProvider},
};
use aptos_types::{
    block_executor::{
        config::BlockExecutorConfig, transaction_slice_metadata::TransactionSliceMetadata,
    },
    state_store::{state_value::StateValueMetadata, MockStateView},
    transaction::{AuxiliaryInfo, BlockOutput},
    write_set::WriteOpKind,
};

type TestKey = KeyType<[u8; 32]>;
type TestTransaction = MockTransaction<TestKey, MockEvent>;
type TestOutput = MockOutput<TestKey, MockEvent>;

/// Helper function to execute a block with pre-write configuration.
fn execute_block_with_pre_write_config<Provider>(
    txn_provider: &Provider,
    data_view: &MockStateView<TestKey>,
    block_stm_v2: bool,
) -> Result<BlockOutput<TestTransaction, TestOutput>, ()>
where
    Provider: TxnProvider<TestTransaction, AuxiliaryInfo> + Sync + 'static,
{
    let executor_thread_pool = create_executor_thread_pool();
    let mut guard = AptosModuleCacheManagerGuard::none();

    let config = BlockExecutorConfig::new_maybe_block_limit(num_cpus::get(), None);
    let block_executor = BlockExecutor::<
        TestTransaction,
        MockTask<TestKey, MockEvent>,
        MockStateView<TestKey>,
        NoOpTransactionCommitHook<usize>,
        Provider,
        AuxiliaryInfo,
    >::new(config, executor_thread_pool, None);

    if block_stm_v2 {
        block_executor.execute_transactions_parallel_v2(
            txn_provider,
            data_view,
            &TransactionSliceMetadata::unknown(),
            &mut guard,
        )
    } else {
        block_executor.execute_transactions_parallel(
            txn_provider,
            data_view,
            &TransactionSliceMetadata::unknown(),
            &mut guard,
        )
    }
}

/// Creates a test key from a byte value.
fn make_key(byte: u8) -> TestKey {
    let mut key = [0u8; 32];
    key[0] = byte;
    KeyType(key)
}

/// Creates a test value with the given byte pattern.
fn make_value(byte: u8) -> ValueType {
    ValueType::new(
        Some(vec![byte; 16].into()),
        StateValueMetadata::none(),
        WriteOpKind::Modification,
    )
}

/// Test that a transaction with pre-writes matching its actual writes succeeds.
///
/// This test creates a transaction that:
/// 1. Declares a pre-write for key A
/// 2. Actually writes to key A during execution
///
/// The pre-write verification should pass and execution should succeed.
#[test]
fn pre_write_matching_actual_writes_succeeds_v1() {
    pre_write_matching_actual_writes_succeeds(false);
}

#[test]
fn pre_write_matching_actual_writes_succeeds_v2() {
    pre_write_matching_actual_writes_succeeds(true);
}

fn pre_write_matching_actual_writes_succeeds(block_stm_v2: bool) {
    let key_a = make_key(1);
    let value = make_value(42);

    // Create a transaction that writes to key_a
    let behavior = MockIncarnation::new(
        vec![],                              // no reads
        vec![(key_a, value.clone(), false)], // write to key_a
        vec![],                              // no deltas
        vec![],                              // no events
        1,                                   // gas
    );

    // Create transaction with pre-writes that match actual writes
    let txn = MockTransaction::from_behavior(behavior)
        .with_pre_writes(vec![(key_a, value.clone())]);

    let txn_provider = DefaultTxnProvider::new_without_info(vec![txn]);
    let state_view = MockStateView::empty();

    let result = execute_block_with_pre_write_config(&txn_provider, &state_view, block_stm_v2);

    // Execution should succeed because pre-writes match actual writes
    assert!(
        result.is_ok(),
        "Execution should succeed when pre-writes match actual writes"
    );
}

/// Test that a transaction without pre-writes succeeds normally.
///
/// This verifies that the pre-write verification doesn't interfere with
/// normal transactions that don't declare any pre-writes.
#[test]
fn no_pre_writes_succeeds_v1() {
    no_pre_writes_succeeds(false);
}

#[test]
fn no_pre_writes_succeeds_v2() {
    no_pre_writes_succeeds(true);
}

fn no_pre_writes_succeeds(block_stm_v2: bool) {
    let key_a = make_key(1);
    let value = make_value(42);

    // Create a transaction that writes to key_a (no pre-writes)
    let behavior = MockIncarnation::new(
        vec![],                              // no reads
        vec![(key_a, value.clone(), false)], // write to key_a
        vec![],                              // no deltas
        vec![],                              // no events
        1,                                   // gas
    );

    // Create transaction without pre-writes (default)
    let txn = MockTransaction::from_behavior(behavior);

    let txn_provider = DefaultTxnProvider::new_without_info(vec![txn]);
    let state_view = MockStateView::empty();

    let result = execute_block_with_pre_write_config(&txn_provider, &state_view, block_stm_v2);

    // Execution should succeed for transactions without pre-writes
    assert!(
        result.is_ok(),
        "Execution should succeed for transactions without pre-writes"
    );
}

/// Test that a transaction with pre-writes that don't match actual writes triggers fallback.
///
/// This test creates a transaction that:
/// 1. Declares a pre-write for key A
/// 2. Actually writes to key B during execution (different key)
///
/// The pre-write verification should fail and trigger fallback (Err).
#[test]
fn pre_write_mismatched_keys_triggers_fallback_v1() {
    pre_write_mismatched_keys_triggers_fallback(false);
}

#[test]
fn pre_write_mismatched_keys_triggers_fallback_v2() {
    pre_write_mismatched_keys_triggers_fallback(true);
}

fn pre_write_mismatched_keys_triggers_fallback(block_stm_v2: bool) {
    let key_a = make_key(1);
    let key_b = make_key(2);
    let value = make_value(42);

    // Create a transaction that writes to key_b (not key_a)
    let behavior = MockIncarnation::new(
        vec![],                              // no reads
        vec![(key_b, value.clone(), false)], // write to key_b
        vec![],                              // no deltas
        vec![],                              // no events
        1,                                   // gas
    );

    // Create transaction with pre-writes for key_a (mismatched)
    let txn = MockTransaction::from_behavior(behavior)
        .with_pre_writes(vec![(key_a, value.clone())]);

    let txn_provider = DefaultTxnProvider::new_without_info(vec![txn]);
    let state_view = MockStateView::empty();

    let result = execute_block_with_pre_write_config(&txn_provider, &state_view, block_stm_v2);

    // Execution should fail (trigger fallback) due to pre-write verification failure
    assert!(
        result.is_err(),
        "Execution should trigger fallback when pre-writes don't match actual writes"
    );
}

/// Test that a transaction with pre-writes but empty output triggers fallback.
///
/// This test creates a transaction that:
/// 1. Declares a pre-write for key A
/// 2. Produces no writes during execution
///
/// The pre-write verification should fail and trigger fallback (Err).
#[test]
fn pre_write_with_empty_output_triggers_fallback_v1() {
    pre_write_with_empty_output_triggers_fallback(false);
}

#[test]
fn pre_write_with_empty_output_triggers_fallback_v2() {
    pre_write_with_empty_output_triggers_fallback(true);
}

fn pre_write_with_empty_output_triggers_fallback(block_stm_v2: bool) {
    let key_a = make_key(1);
    let value = make_value(42);

    // Create a transaction with empty writes
    let behavior = MockIncarnation::new(
        vec![], // no reads
        vec![], // no writes (empty!)
        vec![], // no deltas
        vec![], // no events
        1,      // gas
    );

    // Create transaction with pre-writes for key_a
    let txn = MockTransaction::from_behavior(behavior)
        .with_pre_writes(vec![(key_a, value.clone())]);

    let txn_provider = DefaultTxnProvider::new_without_info(vec![txn]);
    let state_view = MockStateView::empty();

    let result = execute_block_with_pre_write_config(&txn_provider, &state_view, block_stm_v2);

    // Execution should fail (trigger fallback) because transaction has pre-writes but no output
    assert!(
        result.is_err(),
        "Execution should trigger fallback when transaction with pre-writes produces no output"
    );
}

/// Test mixed transactions where one has mismatched pre-writes.
///
/// This test creates multiple transactions where:
/// - Transaction 0: Normal write (no pre-writes) - should succeed
/// - Transaction 1: Pre-write for key A, but writes key B - should trigger fallback
///
/// The entire block should fail due to the verification failure.
#[test]
fn mixed_transactions_one_fails_verification_v1() {
    mixed_transactions_one_fails_verification(false);
}

#[test]
fn mixed_transactions_one_fails_verification_v2() {
    mixed_transactions_one_fails_verification(true);
}

fn mixed_transactions_one_fails_verification(block_stm_v2: bool) {
    let key_a = make_key(1);
    let key_b = make_key(2);
    let key_c = make_key(3);
    let value = make_value(42);

    // Transaction 0: Normal write without pre-writes
    let behavior0 = MockIncarnation::new(
        vec![],
        vec![(key_c, value.clone(), false)],
        vec![],
        vec![],
        1,
    );
    let txn0 = MockTransaction::from_behavior(behavior0);

    // Transaction 1: Pre-write for key_a but writes key_b (mismatch)
    let behavior1 = MockIncarnation::new(
        vec![],
        vec![(key_b, value.clone(), false)],
        vec![],
        vec![],
        1,
    );
    let txn1 = MockTransaction::from_behavior(behavior1)
        .with_pre_writes(vec![(key_a, value.clone())]);

    let txn_provider = DefaultTxnProvider::new_without_info(vec![txn0, txn1]);
    let state_view = MockStateView::empty();

    let result = execute_block_with_pre_write_config(&txn_provider, &state_view, block_stm_v2);

    // Execution should fail due to pre-write verification failure in txn1
    assert!(
        result.is_err(),
        "Execution should trigger fallback when any transaction fails pre-write verification"
    );
}

/// Test multiple transactions all with matching pre-writes.
///
/// Verifies that multiple transactions with pre-writes all succeed when
/// their pre-writes match actual writes.
#[test]
fn multiple_transactions_all_matching_pre_writes_v1() {
    multiple_transactions_all_matching_pre_writes(false);
}

#[test]
fn multiple_transactions_all_matching_pre_writes_v2() {
    multiple_transactions_all_matching_pre_writes(true);
}

fn multiple_transactions_all_matching_pre_writes(block_stm_v2: bool) {
    let key_a = make_key(1);
    let key_b = make_key(2);
    let value_a = make_value(42);
    let value_b = make_value(84);

    // Transaction 0: Pre-write and actual write to key_a
    let behavior0 = MockIncarnation::new(
        vec![],
        vec![(key_a, value_a.clone(), false)],
        vec![],
        vec![],
        1,
    );
    let txn0 = MockTransaction::from_behavior(behavior0)
        .with_pre_writes(vec![(key_a, value_a.clone())]);

    // Transaction 1: Pre-write and actual write to key_b
    let behavior1 = MockIncarnation::new(
        vec![],
        vec![(key_b, value_b.clone(), false)],
        vec![],
        vec![],
        1,
    );
    let txn1 = MockTransaction::from_behavior(behavior1)
        .with_pre_writes(vec![(key_b, value_b.clone())]);

    let txn_provider = DefaultTxnProvider::new_without_info(vec![txn0, txn1]);
    let state_view = MockStateView::empty();

    let result = execute_block_with_pre_write_config(&txn_provider, &state_view, block_stm_v2);

    // All transactions have matching pre-writes, should succeed
    assert!(
        result.is_ok(),
        "Execution should succeed when all transactions have matching pre-writes"
    );
}
