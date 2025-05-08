// Copyright © Aptos Foundation
// Parts of the project are originally copyright © Meta Platforms, Inc.
// SPDX-License-Identifier: Apache-2.0

//! This module provides reusable helpers in tests.

use crate::AptosDB;
use aptos_crypto::{
    hash::{CryptoHash, SPARSE_MERKLE_PLACEHOLDER_HASH},
    HashValue,
};
use aptos_scratchpad::SparseMerkleTree;
use aptos_storage_interface::{DbReader, Order, Result};
use aptos_temppath::TempPath;
use aptos_types::{
    account_address::AccountAddress,
    contract_event::ContractEvent,
    event::EventKey,
    ledger_info::{generate_ledger_info_with_sig, LedgerInfo, LedgerInfoWithSignatures},
    proof::accumulator::{InMemoryEventAccumulator, InMemoryTransactionAccumulator},
    proptest_types::{AccountInfoUniverse, BlockGen},
    state_store::{state_key::StateKey, state_value::StateValue},
    transaction::{
        ReplayProtector, Transaction, TransactionAuxiliaryData, TransactionInfo,
        TransactionToCommit, Version,
    },
    write_set::TransactionWrite,
};
use itertools::Itertools;
use proptest::{
    collection::{hash_set, vec},
    prelude::*,
    sample::Index,
};
use std::{
    collections::{HashMap, HashSet},
    fmt::Debug,
};

/// hack: special keys to guarantee state db has at least two keys
fn kv_genesis_keys() -> Vec<StateKey> {
    vec![StateKey::raw(b"g1"), StateKey::raw(b"g2")]
}

/// hack: special keys to guarantee state db has at least two keys
pub fn kv_store_genesis() -> Vec<(StateKey, Option<StateValue>)> {
    kv_genesis_keys()
        .into_iter()
        .map(|k| (k, Some(StateValue::from(b"genesis_value".to_vec()))))
        .collect()
}

pub fn arb_key_universe(size: usize) -> impl Strategy<Value = Vec<StateKey>> {
    let genesis_keys = kv_genesis_keys();
    hash_set(
        any::<StateKey>().prop_filter(
            "hack: special keys to guarantee state db has at least two keys",
            move |k| genesis_keys.iter().all(|gk| gk != k),
        ),
        size,
    )
    .prop_map(move |keys| keys.into_iter().collect_vec())
}

prop_compose! {
    pub fn arb_state_kv_sets(
        keys: Vec<StateKey>,
        max_update_set_size: usize,
        max_versions: usize,
    )(
        input in vec(
            vec(
                any::<(Index, Option<StateValue>)>(),
                1..=max_update_set_size,
            ),
            1..=max_versions
        )
    ) -> Vec<Vec<(StateKey, Option<StateValue>)>> {
        input
            .into_iter()
            .map(|kvs|
                kvs
                .into_iter()
                .map(|(idx, value)| (idx.get(&keys).clone(), value))
                .collect_vec()
            )
            .collect_vec()
    }
}

prop_compose! {
    pub fn arb_state_kv_sets_with_genesis(
        key_universe_size: usize,
        max_update_set_size: usize,
        max_versions: usize,
    )(
        sets in arb_key_universe(key_universe_size)
            .prop_flat_map(move |keys| {
                arb_state_kv_sets(keys, max_update_set_size, max_versions - 1)
            }),
    ) -> Vec<Vec<(StateKey, Option<StateValue>)>> {
        std::iter::once(kv_store_genesis())
        .chain(sets.into_iter())
        .collect_vec()
    }
}

#[cfg(test)]
pub(crate) fn update_store(
    store: &crate::state_store::StateStore,
    input: impl Iterator<Item = (StateKey, Option<StateValue>)>,
    first_version: Version,
) -> HashValue {
    store.commit_block_for_test(first_version, input.map(|(key, value)| [(key, value)]))
}

pub fn update_in_memory_state(
    smt: &SparseMerkleTree,
    root_smt: &SparseMerkleTree,
    txns_to_commit: &[TransactionToCommit],
) -> SparseMerkleTree {
    let updates = txns_to_commit
        .iter()
        .flat_map(|txn_to_commit| txn_to_commit.write_set().state_update_refs())
        .collect::<HashMap<_, _>>()
        .into_iter()
        .map(|(k, u)| (k.hash(), u.map(CryptoHash::hash)))
        .collect_vec();
    smt.freeze(root_smt)
        .batch_update(updates.iter(), &())
        .unwrap()
        .unfreeze()
}

prop_compose! {
    /// This returns a [`proptest`](https://altsysrq.github.io/proptest-book/intro.html)
    /// [`Strategy`](https://docs.rs/proptest/0/proptest/strategy/trait.Strategy.html) that yields an
    /// arbitrary number of arbitrary batches of transactions to commit.
    ///
    /// It is used in tests for both transaction block committing during normal running and
    /// transaction syncing during start up.
    fn arb_blocks_to_commit_impl(
        num_accounts: usize,
        max_user_txns_per_block: usize,
        min_blocks: usize,
        max_blocks: usize,
    )(
        mut universe in any_with::<AccountInfoUniverse>(num_accounts).no_shrink(),
        block_gens in vec(any_with::<BlockGen>(max_user_txns_per_block), min_blocks..=max_blocks),
    ) -> Vec<(Vec<TransactionToCommit>, LedgerInfoWithSignatures)> {
        let mut txn_accumulator = InMemoryTransactionAccumulator::new_empty();
        let root_smt = SparseMerkleTree::new(*SPARSE_MERKLE_PLACEHOLDER_HASH);
        let mut smt = root_smt.clone();

        let mut result = Vec::new();

        for block_gen in block_gens {
            let (mut txns_to_commit, mut ledger_info) = block_gen.materialize(&mut universe);
            smt = update_in_memory_state(&smt, &root_smt, &txns_to_commit);
            let state_checkpoint_root_hash = smt.root_hash();

            // make real txn_info's
            for txn in txns_to_commit.iter_mut() {
                let placeholder_txn_info = txn.transaction_info();

                // calculate event root hash
                let event_hashes: Vec<_> = txn.events().iter().map(CryptoHash::hash).collect();
                let event_root_hash = InMemoryEventAccumulator::from_leaves(&event_hashes).root_hash();

                // calculate state checkpoint hash and this must be the last txn
                let state_checkpoint_hash = if txn.has_state_checkpoint_hash() {
                    Some(state_checkpoint_root_hash)
                } else {
                    None
                };

                let txn_info = TransactionInfo::new(
                    txn.transaction().onchain_hash(),
                    txn.write_set().hash(),
                    event_root_hash,
                    state_checkpoint_hash,
                    placeholder_txn_info.gas_used(),
                    placeholder_txn_info.status().clone(),
                );
                txn_accumulator = txn_accumulator.append(&[txn_info.hash()]);
                txn.set_transaction_info(txn_info);
            }

            // updated ledger info with real root hash and sign
            ledger_info.set_executed_state_id(txn_accumulator.root_hash());
            let validator_set = universe.get_validator_set(ledger_info.epoch());
            let ledger_info_with_sigs = generate_ledger_info_with_sig(validator_set, ledger_info);

            result.push((txns_to_commit, ledger_info_with_sigs))
        }
        result
    }
}

pub fn arb_blocks_to_commit(
) -> impl Strategy<Value = Vec<(Vec<TransactionToCommit>, LedgerInfoWithSignatures)>> {
    arb_blocks_to_commit_impl(
        5,  /* num_accounts */
        2,  /* max_user_txn_per_block */
        1,  /* min_blocks */
        10, /* max_blocks */
    )
}

pub fn arb_blocks_to_commit_with_block_nums(
    min_blocks: usize,
    max_blocks: usize,
) -> impl Strategy<
    Value = (
        Vec<(Vec<TransactionToCommit>, LedgerInfoWithSignatures)>,
        bool,
    ),
> {
    (
        arb_blocks_to_commit_impl(
            5, /* num_accounts */
            2, /* max_user_txn_per_block */
            min_blocks, max_blocks,
        ),
        proptest::bool::ANY,
    )
}

fn verify_epochs(db: &AptosDB, ledger_infos_with_sigs: &[LedgerInfoWithSignatures]) {
    const LIMIT: usize = 2;
    let mut actual_epoch_change_lis = Vec::new();
    let latest_epoch = ledger_infos_with_sigs
        .last()
        .unwrap()
        .ledger_info()
        .next_block_epoch();

    let mut cursor = 0;
    loop {
        let (chunk, more) = db
            .get_epoch_ending_ledger_infos_impl(cursor, latest_epoch, LIMIT)
            .unwrap();
        actual_epoch_change_lis.extend(chunk);
        if more {
            cursor = actual_epoch_change_lis
                .last()
                .unwrap()
                .ledger_info()
                .next_block_epoch();
        } else {
            break;
        }
    }

    let expected_epoch_change_lis: Vec<_> = ledger_infos_with_sigs
        .iter()
        .filter(|info| info.ledger_info().ends_epoch())
        .cloned()
        .collect();
    assert_eq!(actual_epoch_change_lis, expected_epoch_change_lis);

    let mut last_ver = 0;
    for li in ledger_infos_with_sigs {
        let this_ver = li.ledger_info().version();

        // a version potentially without ledger_info ever committed
        let v1 = (last_ver + this_ver) / 2;
        if v1 != last_ver && v1 != this_ver {
            assert!(db.get_epoch_ending_ledger_info(v1).is_err());
        }

        // a version where there was a ledger_info once
        if li.ledger_info().ends_epoch() {
            assert_eq!(db.get_epoch_ending_ledger_info(this_ver).unwrap(), *li);
        } else {
            assert!(db.get_epoch_ending_ledger_info(this_ver).is_err());
        }
        last_ver = this_ver;
    }
}

fn count_state_updates(txns_to_commit: &[TransactionToCommit]) -> usize {
    txns_to_commit
        .iter()
        .flat_map(|t| t.write_set.state_update_refs())
        .map(|(k, _v)| k)
        .collect::<HashSet<_>>()
        .len()
}

fn gen_snapshot_version(
    estimated_buffer_size: &mut usize,
    txns_to_commit: &[TransactionToCommit],
    first_version: Version,
    threshold: usize,
) -> Option<Version> {
    let last_checkpoint = txns_to_commit
        .iter()
        .rposition(TransactionToCommit::has_state_checkpoint_hash);

    if let Some(idx) = last_checkpoint {
        *estimated_buffer_size += count_state_updates(&txns_to_commit[0..=idx]);
        if idx + 1 != txns_to_commit.len() {
            *estimated_buffer_size += count_state_updates(&txns_to_commit[idx + 1..])
        }
    } else {
        *estimated_buffer_size += count_state_updates(txns_to_commit)
    }

    if let Some(idx) = last_checkpoint {
        if *estimated_buffer_size >= threshold || txns_to_commit[idx].is_reconfig {
            *estimated_buffer_size = 0;
            return Some(first_version + idx as Version);
        }
    }

    None
}

pub fn test_save_blocks_impl(
    input: Vec<(Vec<TransactionToCommit>, LedgerInfoWithSignatures)>,
    snapshot_size_threshold: usize,
) {
    let tmp_dir = TempPath::new();
    let db =
        AptosDB::new_for_test_with_buffered_state_target_items(&tmp_dir, snapshot_size_threshold);

    let num_batches = input.len();
    let mut cur_ver: Version = 0;
    let mut all_committed_txns = vec![];
    let mut estimated_buffer_size = 0;
    let mut snapshot_versions = vec![];
    for (batch_idx, (txns_to_commit, ledger_info_with_sigs)) in input.iter().enumerate() {
        db.save_transactions_for_test(
            txns_to_commit,
            cur_ver, /* first_version */
            Some(ledger_info_with_sigs),
            false, /* sync_commit */
        )
        .unwrap();

        if let Some(v) = gen_snapshot_version(
            &mut estimated_buffer_size,
            txns_to_commit,
            cur_ver,
            snapshot_size_threshold,
        ) {
            snapshot_versions.push(v);
        }

        assert_eq!(
            db.ledger_db.metadata_db().get_latest_ledger_info().unwrap(),
            *ledger_info_with_sigs
        );
        verify_committed_transactions(
            &db,
            txns_to_commit,
            cur_ver,
            ledger_info_with_sigs,
            batch_idx + 1 == num_batches, /* is_latest */
        );

        // check getting all events by version for all committed transactions
        // up to this point using the current ledger info
        all_committed_txns.extend_from_slice(txns_to_commit);

        cur_ver += txns_to_commit.len() as u64;
    }
    db.state_store.buffered_state().lock().sync_commit();

    let first_batch = input.first().unwrap().0.clone();
    let first_batch_ledger_info = input.first().unwrap().1.clone();
    let latest_ledger_info = input.last().unwrap().1.clone();
    // Verify an old batch with the latest LedgerInfo.
    verify_committed_transactions(
        &db,
        &first_batch,
        0,
        &latest_ledger_info,
        false, /* is_latest */
    );
    // Verify an old batch with an old LedgerInfo.
    verify_committed_transactions(
        &db,
        &first_batch,
        0,
        &first_batch_ledger_info,
        true, /* is_latest */
    );
    let (_, ledger_infos_with_sigs): (Vec<_>, Vec<_>) = input.iter().cloned().unzip();
    verify_epochs(&db, &ledger_infos_with_sigs);

    // sync the commits and verify the states
    db.state_store.buffered_state().lock().sync_commit();
    verify_snapshots(
        &db,
        0, /* first_version */
        snapshot_versions,
        input
            .iter()
            .flat_map(|(txns_to_commit, _)| txns_to_commit.iter())
            .collect(),
    );
}

fn verify_snapshots(
    db: &AptosDB,
    start_version: Version,
    snapshot_versions: Vec<Version>,
    txns_to_commit: Vec<&TransactionToCommit>,
) {
    let mut cur_version = start_version;
    let mut updates: HashMap<StateKey, Option<StateValue>> = HashMap::new();
    for snapshot_version in snapshot_versions {
        let start = (cur_version - start_version) as usize;
        let end = (snapshot_version - start_version) as usize;
        assert!(txns_to_commit[end].has_state_checkpoint_hash());
        let expected_root_hash = db
            .ledger_db
            .transaction_info_db()
            .get_transaction_info(snapshot_version)
            .unwrap()
            .state_checkpoint_hash()
            .unwrap();
        updates.extend(
            txns_to_commit[start..=end]
                .iter()
                .flat_map(|x| x.write_set().iter())
                .map(|(k, op)| (k.clone(), op.as_state_value())),
        );
        for (state_key, state_value) in &updates {
            let (state_value_in_db, proof) = db
                .get_state_value_with_proof_by_version(state_key, snapshot_version)
                .unwrap();
            assert_eq!(state_value_in_db.as_ref(), state_value.as_ref());
            proof
                .verify(
                    expected_root_hash,
                    state_key.hash(),
                    state_value_in_db.as_ref(),
                )
                .unwrap();
        }
        cur_version = snapshot_version + 1;
    }
}

fn get_events_by_event_key(
    db: &AptosDB,
    ledger_info: &LedgerInfo,
    event_key: &EventKey,
    first_seq_num: u64,
    last_seq_num: u64,
    order: Order,
    is_latest: bool,
) -> Result<Vec<(Version, ContractEvent)>> {
    const LIMIT: u64 = 3;

    let mut cursor = if order == Order::Ascending {
        first_seq_num
    } else if is_latest {
        // Test the ability to get the latest.
        u64::MAX
    } else {
        last_seq_num
    };

    let mut ret = Vec::new();
    loop {
        let events =
            db.get_events_by_event_key(event_key, cursor, order, LIMIT, ledger_info.version())?;

        let num_events = events.len() as u64;
        if cursor == u64::MAX {
            cursor = last_seq_num;
        }
        let expected_seq_nums: Vec<_> = if order == Order::Ascending {
            (cursor..cursor + num_events).collect()
        } else {
            (cursor + 1 - num_events..=cursor).rev().collect()
        };

        let events: Vec<_> = itertools::zip_eq(events, expected_seq_nums)
            .map(|(e, _)| Ok((e.transaction_version, e.event)))
            .collect::<Result<_>>()
            .unwrap();

        let num_results = events.len() as u64;
        if num_results == 0 {
            break;
        }
        assert_eq!(
            events
                .first()
                .unwrap()
                .1
                .clone()
                .v1()
                .unwrap()
                .sequence_number(),
            cursor
        );

        if order == Order::Ascending {
            if cursor + num_results > last_seq_num {
                ret.extend(
                    events
                        .into_iter()
                        .take((last_seq_num - cursor + 1) as usize),
                );
                break;
            } else {
                ret.extend(events);
                cursor += num_results;
            }
        } else {
            // descending
            if first_seq_num + num_results > cursor {
                ret.extend(
                    events
                        .into_iter()
                        .take((cursor - first_seq_num + 1) as usize),
                );
                break;
            } else {
                ret.extend(events);
                cursor -= num_results;
            }
        }
    }

    if order == Order::Descending {
        ret.reverse();
    }

    Ok(ret)
}

fn verify_events_by_event_key(
    db: &AptosDB,
    events: Vec<(EventKey, Vec<(Version, ContractEvent)>)>,
    ledger_info: &LedgerInfo,
    is_latest: bool,
) {
    events
        .into_iter()
        .map(|(access_path, events)| {
            let first_seq = events
                .first()
                .expect("Shouldn't be empty")
                .1
                .clone()
                .v1()
                .unwrap()
                .sequence_number();
            let last_seq = events
                .last()
                .expect("Shouldn't be empty")
                .1
                .clone()
                .v1()
                .unwrap()
                .sequence_number();

            let traversed = get_events_by_event_key(
                db,
                ledger_info,
                &access_path,
                first_seq,
                last_seq,
                Order::Ascending,
                is_latest,
            )
            .unwrap();
            assert_eq!(events, traversed);

            let rev_traversed = get_events_by_event_key(
                db,
                ledger_info,
                &access_path,
                first_seq,
                last_seq,
                Order::Descending,
                is_latest,
            )
            .unwrap();
            assert_eq!(events, rev_traversed);
            Ok(())
        })
        .collect::<Result<Vec<_>>>()
        .unwrap();
}

fn group_events_by_event_key(
    first_version: Version,
    txns_to_commit: &[TransactionToCommit],
) -> Vec<(EventKey, Vec<(Version, ContractEvent)>)> {
    let mut event_key_to_events: HashMap<EventKey, Vec<(Version, ContractEvent)>> = HashMap::new();
    for (batch_idx, txn) in txns_to_commit.iter().enumerate() {
        for event in txn.events() {
            if let ContractEvent::V1(v1) = event {
                event_key_to_events
                    .entry(*v1.key())
                    .or_default()
                    .push((first_version + batch_idx as u64, event.clone()));
            }
        }
    }
    event_key_to_events.into_iter().collect()
}

fn verify_account_txn_summaries(
    db: &AptosDB,
    expected_txns_by_account: HashMap<AccountAddress, Vec<Transaction>>,
    ledger_info: &LedgerInfo,
    first_version: Version,
) {
    let ledger_version = ledger_info.version();
    for (address, expected_txns) in &expected_txns_by_account {
        let actual_txn_summaries = db.get_account_transaction_summaries(
            *address,
            Some(first_version),
            None,
            expected_txns.len() as u64,
            ledger_version,
        );
        for (expected_txn, actual_txn_summary) in
            expected_txns.iter().zip(actual_txn_summaries.unwrap())
        {
            assert_eq!(
                actual_txn_summary.transaction_hash(),
                expected_txn.submitted_txn_hash()
            );
            assert_eq!(
                actual_txn_summary.replay_protector(),
                expected_txn
                    .try_as_signed_user_txn()
                    .unwrap()
                    .replay_protector()
            );
            assert_eq!(
                actual_txn_summary.sender(),
                expected_txn.try_as_signed_user_txn().unwrap().sender()
            );
            let fetched_txn = db
                .get_transaction_by_version(actual_txn_summary.version(), ledger_version, false)
                .unwrap();
            assert_eq!(fetched_txn.transaction, *expected_txn);
            assert_eq!(fetched_txn.version, actual_txn_summary.version());
        }
    }
}

fn verify_account_ordered_txns(
    db: &AptosDB,
    expected_ordered_txns_by_account: HashMap<
        AccountAddress,
        Vec<(Transaction, Vec<ContractEvent>)>,
    >,
    ledger_info: &LedgerInfo,
) {
    let actual_ordered_txns_by_account = expected_ordered_txns_by_account
        .iter()
        .map(|(account, txns_and_events)| {
            let account = *account;
            let first_seq_num = if let Some((txn, _)) = txns_and_events.first() {
                txn.try_as_signed_user_txn().unwrap().sequence_number()
            } else {
                return (account, Vec::new());
            };

            let last_txn = &txns_and_events.last().unwrap().0;
            let last_seq_num = last_txn.try_as_signed_user_txn().unwrap().sequence_number();
            let limit = last_seq_num + 1;

            let acct_txns_with_proof = db
                .get_account_ordered_transactions(
                    account,
                    first_seq_num,
                    limit,
                    true, /* include_events */
                    ledger_info.version(),
                )
                .unwrap();
            acct_txns_with_proof
                .verify(
                    ledger_info,
                    account,
                    first_seq_num,
                    limit,
                    true,
                    ledger_info.version(),
                )
                .unwrap();

            let txns_and_events = acct_txns_with_proof
                .into_inner()
                .into_iter()
                .map(|txn_with_proof| (txn_with_proof.transaction, txn_with_proof.events.unwrap()))
                .collect::<Vec<_>>();

            (account, txns_and_events)
        })
        .collect::<HashMap<_, _>>();

    assert_eq!(
        actual_ordered_txns_by_account,
        expected_ordered_txns_by_account
    );
}

fn group_txns_by_account(
    txns_to_commit: &[TransactionToCommit],
) -> HashMap<AccountAddress, Vec<Transaction>> {
    let mut account_to_txns = HashMap::new();
    for txn in txns_to_commit {
        if let Some(signed_txn) = txn.transaction().try_as_signed_user_txn() {
            let account = signed_txn.sender();
            account_to_txns
                .entry(account)
                .or_insert_with(Vec::new)
                .push(txn.transaction().clone());
        }
    }
    account_to_txns
}

fn group_ordered_txns_by_account(
    txns_to_commit: &[TransactionToCommit],
) -> HashMap<AccountAddress, Vec<(Transaction, Vec<ContractEvent>)>> {
    let mut account_to_txns = HashMap::new();
    for txn in txns_to_commit {
        if let Some(signed_txn) = txn.transaction().try_as_signed_user_txn() {
            if let ReplayProtector::SequenceNumber(_) = signed_txn.replay_protector() {
                let account = signed_txn.sender();
                account_to_txns
                    .entry(account)
                    .or_insert_with(Vec::new)
                    .push((txn.transaction().clone(), txn.events().to_vec()));
            }
        }
    }
    account_to_txns
}

fn assert_items_equal<'a, T: 'a + Debug + Eq>(
    iter: impl Iterator<Item = &'a T>,
    db_iter_res: Result<impl Iterator<Item = Result<T>>>,
) {
    for (item, db_item) in itertools::zip_eq(iter, db_iter_res.unwrap()) {
        assert_eq!(item, &db_item.unwrap());
    }
}

fn verify_ledger_iterators(
    db: &AptosDB,
    txns_to_commit: &[TransactionToCommit],
    first_version: Version,
    ledger_info_with_sigs: &LedgerInfoWithSignatures,
) {
    let num_txns = txns_to_commit.len() as u64;
    assert_items_equal(
        txns_to_commit.iter().map(|t| t.transaction()),
        db.get_transaction_iterator(first_version, num_txns),
    );
    assert_items_equal(
        txns_to_commit.iter().map(|t| t.transaction_info()),
        db.get_transaction_info_iterator(first_version, num_txns),
    );
    assert_items_equal(
        txns_to_commit
            .iter()
            .map(|t| t.events().to_vec())
            .collect::<Vec<_>>()
            .iter(),
        db.get_events_iterator(first_version, num_txns),
    );
    assert_items_equal(
        txns_to_commit.iter().map(|t| t.write_set()),
        db.get_write_set_iterator(first_version, num_txns),
    );
    let range_proof = db
        .get_transaction_accumulator_range_proof(
            first_version,
            num_txns,
            ledger_info_with_sigs.ledger_info().version(),
        )
        .unwrap();
    range_proof
        .verify(
            ledger_info_with_sigs
                .ledger_info()
                .transaction_accumulator_hash(),
            Some(first_version),
            &db.get_transaction_info_iterator(first_version, num_txns)
                .unwrap()
                .map(|txn_info_res| Ok(txn_info_res?.hash()))
                .collect::<Result<Vec<_>>>()
                .unwrap(),
        )
        .unwrap()
}

pub fn verify_committed_transactions(
    db: &AptosDB,
    txns_to_commit: &[TransactionToCommit],
    first_version: Version,
    ledger_info_with_sigs: &LedgerInfoWithSignatures,
    is_latest: bool,
) {
    verify_ledger_iterators(db, txns_to_commit, first_version, ledger_info_with_sigs);
    let ledger_info = ledger_info_with_sigs.ledger_info();
    let ledger_version = ledger_info.version();
    assert_eq!(
        db.get_accumulator_root_hash(ledger_version).unwrap(),
        ledger_info.transaction_accumulator_hash()
    );

    let mut cur_ver = first_version;
    let mut updates = HashMap::new();
    for txn_to_commit in txns_to_commit {
        let txn_info = db
            .ledger_db
            .transaction_info_db()
            .get_transaction_info(cur_ver)
            .unwrap();

        // Verify the transaction info has the correct onchain hash.
        assert_eq!(
            txn_info.transaction_onchain_hash(),
            txn_to_commit.transaction().onchain_hash()
        );

        // Fetch and verify account states.
        for (state_key, state_value) in txn_to_commit.write_set().state_update_refs() {
            let state_value_in_db = db.get_state_value_by_version(state_key, cur_ver).unwrap();
            assert_eq!(state_value_in_db.as_ref(), state_value);
            updates.insert(state_key, state_value);
        }

        if !txn_to_commit.has_state_checkpoint_hash() {
            // Fetch and verify transaction itself.
            let txn = txn_to_commit
                .transaction()
                .try_as_signed_user_txn()
                .unwrap();
            let txn_with_proof = db
                .get_transaction_by_hash(
                    txn_to_commit.transaction().submitted_txn_hash(),
                    ledger_version,
                    true,
                )
                .unwrap()
                .unwrap();
            assert_eq!(
                txn_with_proof.transaction.onchain_hash(),
                txn_to_commit.transaction().onchain_hash()
            );
            assert_eq!(
                txn_with_proof.transaction.submitted_txn_hash(),
                txn_to_commit.transaction().submitted_txn_hash()
            );
            txn_with_proof
                .verify_user_txn(ledger_info, cur_ver, txn.sender(), txn.replay_protector())
                .unwrap();
            let txn_with_proof = db
                .get_transaction_with_proof(cur_ver, ledger_version, true)
                .unwrap();
            txn_with_proof
                .verify_user_txn(ledger_info, cur_ver, txn.sender(), txn.replay_protector())
                .unwrap();

            if let ReplayProtector::SequenceNumber(seq_num) = txn.replay_protector() {
                let txn_with_proof = db
                    .get_account_ordered_transaction(txn.sender(), seq_num, true, ledger_version)
                    .unwrap()
                    .expect("Should exist.");
                txn_with_proof
                    .verify_user_txn(ledger_info, cur_ver, txn.sender(), txn.replay_protector())
                    .unwrap();

                let acct_txns_with_proof = db
                    .get_account_ordered_transactions(
                        txn.sender(),
                        seq_num,
                        1,
                        true,
                        ledger_version,
                    )
                    .unwrap();
                acct_txns_with_proof
                    .verify(
                        ledger_info,
                        txn.sender(),
                        txn.sequence_number(),
                        1,
                        true,
                        ledger_version,
                    )
                    .unwrap();
                assert_eq!(acct_txns_with_proof.len(), 1);
            }
            let txn_list_with_proof = db
                .get_transactions(cur_ver, 1, ledger_version, true /* fetch_events */)
                .unwrap();
            txn_list_with_proof
                .verify(ledger_info, Some(cur_ver))
                .unwrap();
            assert_eq!(txn_list_with_proof.transactions.len(), 1);

            let txn_output_list_with_proof = db
                .get_transaction_outputs(cur_ver, 1, ledger_version)
                .unwrap();
            txn_output_list_with_proof
                .verify(ledger_info, Some(cur_ver))
                .unwrap();
            assert_eq!(txn_output_list_with_proof.transactions_and_outputs.len(), 1);
        }
        cur_ver += 1;
    }

    // Fetch and verify events.
    verify_events_by_event_key(
        db,
        group_events_by_event_key(first_version, txns_to_commit),
        ledger_info,
        is_latest,
    );

    // Fetch and verify batch transactions by account
    verify_account_txn_summaries(
        db,
        group_txns_by_account(txns_to_commit),
        ledger_info,
        first_version,
    );
    verify_account_ordered_txns(
        db,
        group_ordered_txns_by_account(txns_to_commit),
        ledger_info,
    );
}

pub fn put_transaction_infos(
    db: &AptosDB,
    version: Version,
    txn_infos: &[TransactionInfo],
) -> HashValue {
    db.commit_transaction_infos(version, txn_infos).unwrap();
    db.commit_transaction_accumulator(version, txn_infos)
        .unwrap()
}

pub fn put_transaction_auxiliary_data(
    db: &AptosDB,
    version: Version,
    auxiliary_data: &[TransactionAuxiliaryData],
) {
    db.commit_transaction_auxiliary_data(version, auxiliary_data)
        .unwrap();
}

pub fn test_sync_transactions_impl(
    input: Vec<(Vec<TransactionToCommit>, LedgerInfoWithSignatures)>,
    snapshot_size_threshold: usize,
) {
    let tmp_dir = TempPath::new();
    let db =
        AptosDB::new_for_test_with_buffered_state_target_items(&tmp_dir, snapshot_size_threshold);

    let num_batches = input.len();
    let mut cur_ver: Version = 0;
    let mut estimated_buffer_size = 0;
    let mut snapshot_versions = vec![];
    for (batch_idx, (txns_to_commit, ledger_info_with_sigs)) in input.iter().enumerate() {
        // if batch has more than 2 transactions, save them in two batches
        let batch1_len = txns_to_commit.len() / 2;
        if batch1_len > 0 {
            let txns_to_commit_batch = &txns_to_commit[..batch1_len];
            db.save_transactions_for_test(
                txns_to_commit_batch,
                cur_ver, /* first_version */
                None,    /* ledger_info_with_sigs */
                false,   /* sync_commit */
            )
            .unwrap();

            if let Some(v) = gen_snapshot_version(
                &mut estimated_buffer_size,
                txns_to_commit_batch,
                cur_ver,
                snapshot_size_threshold,
            ) {
                snapshot_versions.push(v);
            }
        }
        let ver = cur_ver + batch1_len as Version;
        let txns_to_commit_batch = &txns_to_commit[batch1_len..];
        db.save_transactions_for_test(
            txns_to_commit_batch,
            ver,
            Some(ledger_info_with_sigs),
            false, /* sync_commit */
        )
        .unwrap();

        if let Some(v) = gen_snapshot_version(
            &mut estimated_buffer_size,
            txns_to_commit_batch,
            ver,
            snapshot_size_threshold,
        ) {
            snapshot_versions.push(v);
        }

        verify_committed_transactions(
            &db,
            txns_to_commit,
            cur_ver,
            ledger_info_with_sigs,
            batch_idx + 1 == num_batches, /* is_latest */
        );

        cur_ver += txns_to_commit.len() as u64;
    }

    // sync the commits and verify the states
    db.state_store.buffered_state().lock().sync_commit();
    verify_snapshots(
        &db,
        0, /* first_version */
        snapshot_versions,
        input
            .iter()
            .flat_map(|(txns_to_commit, _)| txns_to_commit.iter())
            .collect(),
    );
}
