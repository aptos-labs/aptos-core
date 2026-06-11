// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use crate::MoveHarness;
use aptos_block_executor::txn_provider::default::DefaultTxnProvider;
use aptos_cached_packages::aptos_stdlib;
use aptos_crypto::HashValue;
use aptos_types::{
    block_executor::{
        config::{BlockExecutorConfig, BlockExecutorConfigFromOnchain, BlockExecutorLocalConfig},
        transaction_slice_metadata::TransactionSliceMetadata,
    },
    on_chain_config::Features,
    state_store::state_key::StateKey,
    transaction::{signature_verified_transaction::into_signature_verified_block, Transaction},
};
use aptos_vm::{aptos_vm::AptosVMBlockExecutor, VMBlockExecutor};
use move_core_types::{account_address::AccountAddress, ident_str};
use std::collections::BTreeSet;

/// Executes the block against the harness state (without applying it) and returns the
/// epilogue's `to_make_hot` set, together with all keys the block's outputs value-write.
fn execute_and_get_hot_state_promotions(
    h: &MoveHarness,
    txns: Vec<Transaction>,
    concurrency_level: usize,
) -> (BTreeSet<StateKey>, BTreeSet<StateKey>) {
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
    (to_make_hot, written_keys)
}

#[test]
fn test_hot_state_promotions() {
    let mut h = MoveHarness::new();
    let alice = h.new_account_with_key_pair();
    let bob = h.new_account_with_key_pair();

    let txns = vec![
        Transaction::UserTransaction(h.create_transaction_payload(
            &alice,
            aptos_stdlib::aptos_account_transfer(*bob.address(), 100),
        )),
        Transaction::UserTransaction(h.create_transaction_payload(
            &bob,
            aptos_stdlib::aptos_account_transfer(*alice.address(), 50),
        )),
    ];

    let (sequential, written) = execute_and_get_hot_state_promotions(&h, txns.clone(), 1);
    let (parallel, _) = execute_and_get_hot_state_promotions(&h, txns, 4);

    // The read set is recorded at the VM boundary, so the promotions must not depend on the
    // executor mode.
    assert_eq!(sequential, parallel);

    // Modules executed by the transactions are read but not written in this block, so they
    // must be promoted. This covers reads served by the module caches rather than the state
    // view.
    assert!(sequential.contains(&StateKey::module(
        &AccountAddress::ONE,
        ident_str!("aptos_account")
    )));

    // Keys written in the block become hot at the version they are written, so promoting
    // them again is redundant and they must not show up. This covers writes invisible to
    // the conflict summary, e.g. total supply updates via aggregators.
    let promoted_and_written: Vec<_> = sequential.intersection(&written).collect();
    assert!(
        promoted_and_written.is_empty(),
        "Promoted keys also written in the block: {:?}",
        promoted_and_written,
    );
}
