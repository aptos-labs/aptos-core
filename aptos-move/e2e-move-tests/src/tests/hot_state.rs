// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use crate::{assert_success, tests::common, MoveHarness};
use aptos_block_executor::txn_provider::default::DefaultTxnProvider;
use aptos_cached_packages::aptos_stdlib;
use aptos_crypto::HashValue;
use aptos_types::{
    account_config::{AccountResource, CoinInfoResource},
    block_executor::{
        config::{BlockExecutorConfig, BlockExecutorConfigFromOnchain, BlockExecutorLocalConfig},
        transaction_slice_metadata::TransactionSliceMetadata,
    },
    chain_id::ChainId,
    on_chain_config::{CurrentTimeMicroseconds, Features},
    state_store::state_key::{inner::StateKeyInner, StateKey},
    transaction::{signature_verified_transaction::into_signature_verified_block, Transaction},
    utility_coin::AptosCoinType,
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
    // Charlie signs nothing in the block below, so nothing writes his account resource
    // and the helper's read of it must surface as a promotion.
    let charlie = h.new_account_with_key_pair();

    // Published ahead of the block under test, so within the block the module is just
    // another read.
    let cafe = h.new_account_at(AccountAddress::from_hex_literal("0xcafe").unwrap());
    assert_success!(h.publish_package(&cafe, &common::test_dir_path("hot_state.data/pack")));

    let txns = vec![
        Transaction::UserTransaction(h.create_transaction_payload(
            &alice,
            aptos_stdlib::aptos_account_transfer(*bob.address(), 100),
        )),
        Transaction::UserTransaction(h.create_transaction_payload(
            &bob,
            aptos_stdlib::aptos_account_transfer(*alice.address(), 50),
        )),
        Transaction::UserTransaction(h.create_entry_function(
            &alice,
            str::parse("0xcafe::read_helper::read_only").unwrap(),
            vec![],
            vec![bcs::to_bytes(charlie.address()).unwrap()],
        )),
    ];

    let (sequential, written) = execute_and_get_hot_state_promotions(&h, txns.clone(), 1);
    let (parallel, _) = execute_and_get_hot_state_promotions(&h, txns, 4);
    assert_eq!(sequential, parallel);

    // Modules executed by the transactions are read but not written in this block, so they
    // must be promoted.
    assert!(sequential.contains(&StateKey::module(
        &AccountAddress::ONE,
        ident_str!("aptos_account")
    )));
    // Same for the user-published module.
    assert!(sequential.contains(&StateKey::module(cafe.address(), ident_str!("read_helper"))));

    // The following are read but never written in this block, so they must be promoted.
    for key in [
        StateKey::on_chain_config::<ChainId>().unwrap(),
        StateKey::on_chain_config::<CurrentTimeMicroseconds>().unwrap(),
        StateKey::resource_typed::<AccountResource>(charlie.address()).unwrap(),
        StateKey::resource_typed::<CoinInfoResource<AptosCoinType>>(&AccountAddress::ONE).unwrap(),
    ] {
        assert!(
            sequential.contains(&key),
            "Expected promotion for {:?}",
            key
        );
    }

    // The coin-to-fungible-asset conversion map is a table keyed by coin type, read for
    // paired metadata lookups but never written in this block, so a table item must be
    // promoted.
    assert!(
        sequential
            .iter()
            .any(|key| matches!(key.inner(), StateKeyInner::TableItem { .. })),
        "Expected the coin conversion map table item to be promoted",
    );

    // Keys written in the block become hot at the version they are written, so promoting
    // them again is redundant and they must not show up.
    let promoted_and_written: Vec<_> = sequential.intersection(&written).collect();
    assert!(
        promoted_and_written.is_empty(),
        "Promoted keys also written in the block: {:?}",
        promoted_and_written,
    );
}
