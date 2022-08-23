// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

///! This module provides reusable helpers in tests.
use super::*;
use crate::{
    jellyfish_merkle_node::JellyfishMerkleNodeSchema, schema::state_value::StateValueSchema,
};
use aptos_types::ledger_info::generate_ledger_info_with_sig;

use aptos_crypto::hash::{CryptoHash, EventAccumulatorHasher, TransactionAccumulatorHasher};
use aptos_jellyfish_merkle::node_type::{Node, NodeKey};
use aptos_temppath::TempPath;
use aptos_types::{
    contract_event::ContractEvent,
    ledger_info::{LedgerInfo, LedgerInfoWithSignatures},
    proof::accumulator::InMemoryAccumulator,
    proptest_types::{AccountInfoUniverse, BlockGen},
};
use executor_types::ProofReader;
use proptest::sample::Index;
use proptest::{collection::vec, prelude::*};
use scratchpad::SparseMerkleTree;

prop_compose! {
    pub fn arb_state_kv_sets(
        key_universe_size: usize,
        max_update_set_size: usize,
        max_versions: usize
    )
    (
        keys in vec(any::<StateKey>(), key_universe_size),
        input in vec(vec((any::<Index>(), any::<Option<StateValue>>()), 1..max_update_set_size), 1..max_versions)
    ) -> Vec<Vec<(StateKey, Option<StateValue>)>> {
            input
            .into_iter()
            .map(|kvs|
                kvs
                .into_iter()
                .map(|(idx, value)| (idx.get(&keys).clone(), value))
                .collect::<Vec<_>>()
            )
            .collect::<Vec<_>>()
    }
}

#[cfg(test)]
pub(crate) fn update_store(
    store: &StateStore,
    input: impl Iterator<Item = (StateKey, Option<StateValue>)>,
    first_version: Version,
) -> HashValue {
    use storage_interface::{jmt_update_refs, jmt_updates};
    let mut root_hash = *aptos_crypto::hash::SPARSE_MERKLE_PLACEHOLDER_HASH;
    for (i, (key, value)) in input.enumerate() {
        let value_state_set = vec![(key, value)].into_iter().collect();
        let jmt_updates = jmt_updates(&value_state_set);
        let version = first_version + i as Version;
        root_hash = store
            .merklize_value_set(
                jmt_update_refs(&jmt_updates),
                None,
                version,
                version.checked_sub(1),
            )
            .unwrap();
        let mut batch = SchemaBatch::new();
        store
            .put_value_sets(
                vec![&value_state_set],
                version,
                StateStorageUsage::new_untracked(),
                &mut batch,
            )
            .unwrap();
        store.ledger_db.write_schemas(batch).unwrap();
    }
    root_hash
}

pub fn update_in_memory_state(state: &mut StateDelta, txns_to_commit: &[TransactionToCommit]) {
    let mut next_version = state.current_version.map_or(0, |v| v + 1);
    for txn_to_commit in txns_to_commit {
        txn_to_commit
            .state_updates()
            .iter()
            .for_each(|(key, value)| {
                state.updates_since_base.insert(key.clone(), value.clone());
            });
        next_version += 1;
        if txn_to_commit.is_state_checkpoint() {
            state.current = state
                .current
                .clone()
                .freeze()
                .batch_update(
                    state
                        .updates_since_base
                        .iter()
                        .map(|(k, v)| (k.hash(), v.as_ref()))
                        .collect(),
                    StateStorageUsage::new_untracked(),
                    &ProofReader::new_empty(),
                )
                .unwrap()
                .unfreeze();
            state.current_version = next_version.checked_sub(1);
            state.base = state.current.clone();
            state.base_version = state.current_version;
            state.updates_since_base.clear();
        }
    }

    if next_version.checked_sub(1) != state.current_version {
        state.current = state
            .current
            .clone()
            .freeze()
            .batch_update(
                state
                    .updates_since_base
                    .iter()
                    .map(|(k, v)| (k.hash(), v.as_ref()))
                    .collect(),
                StateStorageUsage::new_untracked(),
                &ProofReader::new_empty(),
            )
            .unwrap()
            .unfreeze();
        state.current_version = next_version.checked_sub(1);
    }
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
        max_blocks: usize,
    )(
        mut universe in any_with::<AccountInfoUniverse>(num_accounts).no_shrink(),
        block_gens in vec(any_with::<BlockGen>(max_user_txns_per_block), 1..=max_blocks),
    ) -> Vec<(Vec<TransactionToCommit>, LedgerInfoWithSignatures)> {
        type EventAccumulator = InMemoryAccumulator<EventAccumulatorHasher>;
        type TxnAccumulator = InMemoryAccumulator<TransactionAccumulatorHasher>;

        let mut txn_accumulator = TxnAccumulator::new_empty();
        let mut result = Vec::new();

        let mut in_memory_state = StateDelta::new_empty();
        let _ancester = in_memory_state.current.clone().freeze();

        for block_gen in block_gens {
            let (mut txns_to_commit, mut ledger_info) = block_gen.materialize(&mut universe);
            update_in_memory_state(&mut in_memory_state, &txns_to_commit);
            let state_checkpoint_root_hash = in_memory_state.root_hash();

            // make real txn_info's
            for txn in txns_to_commit.iter_mut() {
                let placeholder_txn_info = txn.transaction_info();

                // calculate event root hash
                let event_hashes: Vec<_> = txn.events().iter().map(CryptoHash::hash).collect();
                let event_root_hash = EventAccumulator::from_leaves(&event_hashes).root_hash();

                // calculate state checkpoint hash and this must be the last txn
                let state_checkpoint_hash = if txn.is_state_checkpoint() {
                    Some(state_checkpoint_root_hash)
                } else {
                    None
                };

                let txn_info = TransactionInfo::new(
                    txn.transaction().hash(),
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
        10, /* max_blocks */
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

fn gen_snapshot_version(
    updates: &mut HashMap<StateKey, Option<StateValue>>,
    txns_to_commit: &[TransactionToCommit],
    cur_ver: Version,
    threshold: usize,
) -> Option<Version> {
    let mut snapshot_version = None;
    let last_checkpoint = txns_to_commit
        .iter()
        .enumerate()
        .filter(|(_idx, x)| x.is_state_checkpoint())
        .last()
        .map(|(idx, _)| idx);
    if let Some(idx) = last_checkpoint {
        updates.extend(
            txns_to_commit[0..=idx]
                .iter()
                .flat_map(|x| x.state_updates().clone())
                .collect::<HashMap<_, _>>(),
        );
        if updates.len() >= threshold {
            snapshot_version = Some(cur_ver + idx as u64);
            updates.clear();
        }
        updates.extend(
            txns_to_commit[idx + 1..]
                .iter()
                .flat_map(|x| x.state_updates().clone())
                .collect::<HashMap<_, _>>(),
        );
    } else {
        updates.extend(
            txns_to_commit
                .iter()
                .flat_map(|x| x.state_updates().clone())
                .collect::<HashMap<_, _>>(),
        );
    }
    snapshot_version
}

pub fn test_save_blocks_impl(
    input: Vec<(Vec<TransactionToCommit>, LedgerInfoWithSignatures)>,
    snapshot_size_threshold: usize,
) {
    let tmp_dir = TempPath::new();
    let db = AptosDB::new_for_test_with_target_snapshot_size(&tmp_dir, snapshot_size_threshold);

    let mut in_memory_state = db
        .state_store
        .buffered_state()
        .lock()
        .current_state()
        .clone();
    let _ancester = in_memory_state.current.clone();
    let num_batches = input.len();
    let mut cur_ver: Version = 0;
    let mut all_committed_txns = vec![];
    let mut updates = HashMap::new();
    let mut snapshot_versions = vec![];
    for (batch_idx, (txns_to_commit, ledger_info_with_sigs)) in input.iter().enumerate() {
        update_in_memory_state(&mut in_memory_state, txns_to_commit.as_slice());
        db.save_transactions(
            txns_to_commit,
            cur_ver,                /* first_version */
            cur_ver.checked_sub(1), /* base_state_version */
            Some(ledger_info_with_sigs),
            false, /* sync_commit */
            in_memory_state.clone(),
        )
        .unwrap();

        if let Some(v) = gen_snapshot_version(
            &mut updates,
            txns_to_commit,
            cur_ver,
            snapshot_size_threshold,
        ) {
            snapshot_versions.push(v);
        }

        assert_eq!(
            db.ledger_store.get_latest_ledger_info().unwrap(),
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
    let mut updates: HashMap<&StateKey, Option<&StateValue>> = HashMap::new();
    for snapshot_version in snapshot_versions {
        let start = (cur_version - start_version) as usize;
        let end = (snapshot_version - start_version) as usize;
        assert!(txns_to_commit[end].is_state_checkpoint());
        let expected_root_hash = db
            .ledger_store
            .get_transaction_info(snapshot_version)
            .unwrap()
            .state_checkpoint_hash()
            .unwrap();
        updates.extend(
            txns_to_commit[start..=end]
                .iter()
                .flat_map(|x| {
                    x.state_updates()
                        .iter()
                        .map(|(k, v_opt)| (k, v_opt.as_ref()))
                })
                .collect::<HashMap<&StateKey, Option<&StateValue>>>(),
        );
        for (state_key, state_value) in &updates {
            let (state_value_in_db, proof) = db
                .get_state_value_with_proof_by_version(state_key, snapshot_version)
                .unwrap();
            assert_eq!(state_value_in_db.as_ref(), *state_value);
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
        u64::max_value()
    } else {
        last_seq_num
    };

    let mut ret = Vec::new();
    loop {
        let events =
            db.get_events_by_event_key(event_key, cursor, order, LIMIT, ledger_info.version())?;

        let num_events = events.len() as u64;
        if cursor == u64::max_value() {
            cursor = last_seq_num;
        }
        let expected_seq_nums: Vec<_> = if order == Order::Ascending {
            (cursor..cursor + num_events).collect()
        } else {
            (cursor + 1 - num_events..=cursor).rev().collect()
        };

        let events: Vec<_> = itertools::zip_eq(events, expected_seq_nums)
            .map(|(e, _)| (e.transaction_version, e.event))
            .collect();

        let num_results = events.len() as u64;
        if num_results == 0 {
            break;
        }
        assert_eq!(events.first().unwrap().1.sequence_number(), cursor);

        if order == Order::Ascending {
            if cursor + num_results > last_seq_num {
                ret.extend(
                    events
                        .into_iter()
                        .take((last_seq_num - cursor + 1) as usize),
                );
                break;
            } else {
                ret.extend(events.into_iter());
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
                ret.extend(events.into_iter());
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
                .sequence_number();
            let last_seq = events
                .last()
                .expect("Shouldn't be empty")
                .1
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
            event_key_to_events
                .entry(*event.key())
                .or_default()
                .push((first_version + batch_idx as u64, event.clone()));
        }
    }
    event_key_to_events.into_iter().collect()
}

fn verify_account_txns(
    db: &AptosDB,
    expected_txns_by_account: HashMap<AccountAddress, Vec<(Transaction, Vec<ContractEvent>)>>,
    ledger_info: &LedgerInfo,
) {
    let actual_txns_by_account = expected_txns_by_account
        .iter()
        .map(|(account, txns_and_events)| {
            let account = *account;
            let first_seq_num = if let Some((txn, _)) = txns_and_events.first() {
                txn.as_signed_user_txn().unwrap().sequence_number()
            } else {
                return (account, Vec::new());
            };

            let last_txn = &txns_and_events.last().unwrap().0;
            let last_seq_num = last_txn.as_signed_user_txn().unwrap().sequence_number();
            let limit = last_seq_num + 1;

            let acct_txns_with_proof = db
                .get_account_transactions(
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

    assert_eq!(actual_txns_by_account, expected_txns_by_account);
}

fn group_txns_by_account(
    txns_to_commit: &[TransactionToCommit],
) -> HashMap<AccountAddress, Vec<(Transaction, Vec<ContractEvent>)>> {
    let mut account_to_txns = HashMap::new();
    for txn in txns_to_commit {
        if let Ok(signed_txn) = txn.transaction().as_signed_user_txn() {
            let account = signed_txn.sender();
            account_to_txns
                .entry(account)
                .or_insert_with(Vec::new)
                .push((txn.transaction().clone(), txn.events().to_vec()));
        }
    }
    account_to_txns
}

pub fn verify_committed_transactions(
    db: &AptosDB,
    txns_to_commit: &[TransactionToCommit],
    first_version: Version,
    ledger_info_with_sigs: &LedgerInfoWithSignatures,
    is_latest: bool,
) {
    let ledger_info = ledger_info_with_sigs.ledger_info();
    let ledger_version = ledger_info.version();
    assert_eq!(
        db.get_accumulator_root_hash(ledger_version).unwrap(),
        ledger_info.transaction_accumulator_hash()
    );

    let mut cur_ver = first_version;
    let mut updates = HashMap::new();
    for txn_to_commit in txns_to_commit {
        let txn_info = db.ledger_store.get_transaction_info(cur_ver).unwrap();

        // Verify transaction hash.
        assert_eq!(
            txn_info.transaction_hash(),
            txn_to_commit.transaction().hash()
        );

        // Fetch and verify account states.
        for (state_key, state_value) in txn_to_commit.state_updates() {
            updates.insert(state_key, state_value);
            let state_value_in_db = db.get_state_value_by_version(state_key, cur_ver).unwrap();
            assert_eq!(state_value_in_db, *state_value);
        }

        if !txn_to_commit.is_state_checkpoint() {
            // Fetch and verify transaction itself.
            let txn = txn_to_commit.transaction().as_signed_user_txn().unwrap();
            let txn_with_proof = db
                .get_transaction_by_hash(txn_to_commit.transaction().hash(), ledger_version, true)
                .unwrap()
                .unwrap();
            assert_eq!(
                txn_with_proof.transaction.hash(),
                txn_to_commit.transaction().hash()
            );
            txn_with_proof
                .verify_user_txn(ledger_info, cur_ver, txn.sender(), txn.sequence_number())
                .unwrap();
            let txn_with_proof = db
                .get_transaction_with_proof(cur_ver, ledger_version, true)
                .unwrap();
            txn_with_proof
                .verify_user_txn(ledger_info, cur_ver, txn.sender(), txn.sequence_number())
                .unwrap();

            let txn_with_proof = db
                .get_account_transaction(txn.sender(), txn.sequence_number(), true, ledger_version)
                .unwrap()
                .expect("Should exist.");
            txn_with_proof
                .verify_user_txn(ledger_info, cur_ver, txn.sender(), txn.sequence_number())
                .unwrap();

            let acct_txns_with_proof = db
                .get_account_transactions(
                    txn.sender(),
                    txn.sequence_number(),
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
    verify_account_txns(db, group_txns_by_account(txns_to_commit), ledger_info);
}

pub fn put_transaction_info(db: &AptosDB, version: Version, txn_info: &TransactionInfo) {
    let mut batch = SchemaBatch::new();
    db.ledger_store
        .put_transaction_infos(version, &[txn_info.clone()], &mut batch)
        .unwrap();
    db.ledger_db.write_schemas(batch).unwrap();
}

pub fn put_as_state_root(db: &AptosDB, version: Version, key: StateKey, value: StateValue) {
    let leaf_node = Node::new_leaf(key.hash(), value.hash(), (key.clone(), version));
    db.state_merkle_db
        .put::<JellyfishMerkleNodeSchema>(&NodeKey::new_empty_path(version), &leaf_node)
        .unwrap();
    let smt = SparseMerkleTree::<StateValue>::default()
        .batch_update(vec![(key.hash(), Some(&value))], &ProofReader::new_empty())
        .unwrap();
    db.ledger_db
        .put::<StateValueSchema>(&(key.clone(), version), &Some(value.clone()))
        .unwrap();
    let mut in_memory_state = db
        .state_store
        .buffered_state()
        .lock()
        .current_state()
        .clone();
    in_memory_state.current = smt;
    in_memory_state.current_version = Some(version);
    in_memory_state.updates_since_base.insert(key, Some(value));
    db.state_store
        .buffered_state()
        .lock()
        .update(None, in_memory_state, true)
        .unwrap();
}

pub fn test_sync_transactions_impl(
    input: Vec<(Vec<TransactionToCommit>, LedgerInfoWithSignatures)>,
    snapshot_size_threshold: usize,
) {
    let tmp_dir = TempPath::new();
    let db = AptosDB::new_for_test_with_target_snapshot_size(&tmp_dir, snapshot_size_threshold);

    let mut in_memory_state = db
        .state_store
        .buffered_state()
        .lock()
        .current_state()
        .clone();
    let _ancester = in_memory_state.current.clone();
    let num_batches = input.len();
    let mut cur_ver: Version = 0;
    let mut updates = HashMap::new();
    let mut snapshot_versions = vec![];
    for (batch_idx, (txns_to_commit, ledger_info_with_sigs)) in input.iter().enumerate() {
        // if batch has more than 2 transactions, save them in two batches
        let batch1_len = txns_to_commit.len() / 2;
        let base_state_version = cur_ver.checked_sub(1);
        if batch1_len > 0 {
            update_in_memory_state(&mut in_memory_state, &txns_to_commit[..batch1_len]);
            db.save_transactions(
                &txns_to_commit[..batch1_len],
                cur_ver, /* first_version */
                base_state_version,
                None,
                false, /* sync_commit */
                in_memory_state.clone(),
            )
            .unwrap();
        }
        update_in_memory_state(&mut in_memory_state, &txns_to_commit[batch1_len..]);
        db.save_transactions(
            &txns_to_commit[batch1_len..],
            cur_ver + batch1_len as u64, /* first_version */
            base_state_version,
            Some(ledger_info_with_sigs),
            false, /* sync_commit */
            in_memory_state.clone(),
        )
        .unwrap();

        if let Some(v) = gen_snapshot_version(
            &mut updates,
            txns_to_commit,
            cur_ver,
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
