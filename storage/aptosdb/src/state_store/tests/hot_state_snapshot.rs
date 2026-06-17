// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Tests for the hot state snapshot read APIs used by fast sync to serve hot state.

use crate::{db::test_helper::arb_blocks_to_commit_with_params, AptosDB};
use aptos_crypto::{hash::CryptoHash, HashValue};
use aptos_storage_interface::{DbReader, Result as DbResult};
use aptos_temppath::TempPath;
use aptos_types::{
    ledger_info::LedgerInfoWithSignatures,
    state_store::{hot_state::HotStateValue, state_key::StateKey, state_value::StateValue},
    transaction::{
        BlockEndInfo, ExecutionStatus, Transaction, TransactionAuxiliaryData, TransactionInfo,
        TransactionToCommit, Version,
    },
    write_set::WriteSet,
};
use proptest::prelude::*;
use std::collections::BTreeMap;

fn key(seed: &str) -> StateKey {
    StateKey::raw(seed.as_bytes())
}

fn val(seed: &[u8]) -> StateValue {
    StateValue::new_legacy(seed.to_vec().into())
}

/// Builds the block-epilogue transaction that ends a block, carrying `writes` in its write set.
/// A block epilogue is a state checkpoint, so committing it persists a hot state Merkle snapshot
/// at its version.
fn block_epilogue_txn(writes: Vec<(StateKey, Option<StateValue>)>) -> TransactionToCommit {
    // `Some(..)` state checkpoint hash is what marks the version as a checkpoint; the hash values
    // are irrelevant to the hot state reads under test. `None` auxiliary info hash keeps the
    // commit path from resolving per-transaction auxiliary info.
    let txn_info = TransactionInfo::new(
        HashValue::random(),
        HashValue::random(),
        HashValue::random(),
        Some(HashValue::random()),
        0,
        ExecutionStatus::Success,
        None,
    );
    TransactionToCommit::new(
        Transaction::block_epilogue_v0(HashValue::zero(), BlockEndInfo::new_empty()),
        txn_info,
        WriteSet::new_for_test(writes),
        vec![],                              /* events */
        false,                               /* is_reconfig */
        TransactionAuxiliaryData::default(), /* transaction_auxiliary_data */
    )
}

/// Commits each element of `blocks` as a one-transaction (block-epilogue) block and returns the
/// last synced version.
fn commit_blocks(db: &AptosDB, blocks: Vec<Vec<(StateKey, Option<StateValue>)>>) -> Version {
    let mut version = 0;
    for writes in blocks {
        db.save_transactions_for_test(&[block_epilogue_txn(writes)], version, None, true)
            .unwrap();
        version += 1;
    }
    version - 1
}

fn collect_chunk(
    db: &AptosDB,
    version: Version,
    first_index: usize,
    chunk_size: usize,
) -> Vec<(StateKey, HotStateValue)> {
    db.get_hot_state_value_chunk_iter(version, first_index, chunk_size)
        .unwrap()
        .collect::<DbResult<Vec<_>>>()
        .unwrap()
}

/// Walks the whole hot state at `version` by repeatedly fetching `chunk_size` items at a time,
/// advancing the start index past what each call returned.
fn collect_paged(
    db: &AptosDB,
    version: Version,
    chunk_size: usize,
) -> Vec<(StateKey, HotStateValue)> {
    let mut out = vec![];
    loop {
        let chunk = collect_chunk(db, version, out.len(), chunk_size);
        if chunk.is_empty() {
            break;
        }
        out.extend(chunk);
    }
    out
}

#[test]
fn test_hot_state_chunk_iter_basic() {
    let tmp = TempPath::new();
    let db = AptosDB::new_for_test(&tmp);

    // v0: write a, b. v1: delete a, re-write b, write c.
    let last = commit_blocks(&db, vec![
        vec![(key("a"), Some(val(b"a0"))), (key("b"), Some(val(b"b0")))],
        vec![
            (key("a"), None),
            (key("b"), Some(val(b"b1"))),
            (key("c"), Some(val(b"c1"))),
        ],
    ]);
    assert_eq!(last, 1);

    // All three keys are hot: `a` vacant (deleted), `b`/`c` occupied with their latest values.
    assert_eq!(db.get_hot_state_item_count(last).unwrap(), 3);

    let items = collect_chunk(&db, last, 0, usize::MAX);
    let by_key: BTreeMap<_, _> = items.iter().cloned().collect();

    assert_eq!(by_key[&key("a")].value_opt(), None);
    assert_eq!(by_key[&key("a")].hot_since_version(), 1);
    assert_eq!(by_key[&key("b")].value_opt(), Some(&val(b"b1")));
    assert_eq!(by_key[&key("b")].hot_since_version(), 1);
    assert_eq!(by_key[&key("c")].value_opt(), Some(&val(b"c1")));
    assert_eq!(by_key[&key("c")].hot_since_version(), 1);

    // Returned in strictly increasing key-hash (JMT) order.
    let hashes: Vec<_> = items.iter().map(|(k, _)| k.hash()).collect();
    assert!(hashes.windows(2).all(|w| w[0] < w[1]));
}

#[test]
fn test_hot_state_chunk_iter_pagination() {
    let tmp = TempPath::new();
    let db = AptosDB::new_for_test(&tmp);

    let writes = (0..5u8)
        .map(|i| (key(&format!("k{i}")), Some(val(&[i]))))
        .collect();
    let version = commit_blocks(&db, vec![writes]);
    let count = db.get_hot_state_item_count(version).unwrap();
    assert_eq!(count, 5);

    let full = collect_chunk(&db, version, 0, usize::MAX);
    assert_eq!(full.len(), count);

    // Any chunk size reconstructs the full, single-pass result.
    for chunk_size in [1, 2, 3, 5, 7] {
        assert_eq!(collect_paged(&db, version, chunk_size), full);
    }

    // `first_index` offsets into the result; at/after the end it yields nothing.
    assert_eq!(
        collect_chunk(&db, version, 2, usize::MAX),
        full[2..].to_vec()
    );
    assert_eq!(collect_chunk(&db, version, count, usize::MAX), vec![]);
    assert_eq!(collect_chunk(&db, version, count + 1, usize::MAX), vec![]);
    // A zero-sized chunk yields nothing.
    assert_eq!(collect_chunk(&db, version, 0, 0), vec![]);
}

#[test]
fn test_hot_state_empty() {
    let tmp = TempPath::new();
    let db = AptosDB::new_for_test(&tmp);

    // A checkpoint that writes nothing leaves the hot state empty.
    let version = commit_blocks(&db, vec![vec![]]);
    assert_eq!(db.get_hot_state_item_count(version).unwrap(), 0);
    assert_eq!(collect_chunk(&db, version, 0, usize::MAX), vec![]);
    assert_eq!(collect_chunk(&db, version, 5, usize::MAX), vec![]);
}

#[test]
fn test_hot_state_chunk_iter_at_older_checkpoint() {
    let tmp = TempPath::new();
    let db = AptosDB::new_for_test(&tmp);

    // v0: write a, b. v1: delete a, write c; b is left untouched.
    commit_blocks(&db, vec![
        vec![(key("a"), Some(val(b"a0"))), (key("b"), Some(val(b"b0")))],
        vec![(key("a"), None), (key("c"), Some(val(b"c1")))],
    ]);

    // Reading the older snapshot sees the state as of v0 only.
    assert_eq!(db.get_hot_state_item_count(0).unwrap(), 2);
    let at_v0: BTreeMap<_, _> = collect_chunk(&db, 0, 0, usize::MAX).into_iter().collect();
    assert_eq!(at_v0[&key("a")].value_opt(), Some(&val(b"a0")));
    assert_eq!(at_v0[&key("a")].hot_since_version(), 0);
    assert_eq!(at_v0[&key("b")].value_opt(), Some(&val(b"b0")));
    assert!(!at_v0.contains_key(&key("c")));

    // At v1, `b` is untouched — its value and `hot_since_version` are still those from v0, even
    // though its JMT leaf now sits under a newer (v1) snapshot.
    assert_eq!(db.get_hot_state_item_count(1).unwrap(), 3);
    let at_v1: BTreeMap<_, _> = collect_chunk(&db, 1, 0, usize::MAX).into_iter().collect();
    assert_eq!(at_v1[&key("a")].value_opt(), None);
    assert_eq!(at_v1[&key("a")].hot_since_version(), 1);
    assert_eq!(at_v1[&key("b")].value_opt(), Some(&val(b"b0")));
    assert_eq!(at_v1[&key("b")].hot_since_version(), 0);
    assert_eq!(at_v1[&key("c")].value_opt(), Some(&val(b"c1")));
    assert_eq!(at_v1[&key("c")].hot_since_version(), 1);
}

/// The hot state expected at the latest checkpoint: every key ever written, keyed by key hash,
/// carrying its latest value (vacant if last written as a deletion) and the version it was last
/// written at (its `hot_since_version`). Mirrors that any write makes a key hot.
fn expected_hot_state(
    blocks: &[(Vec<TransactionToCommit>, LedgerInfoWithSignatures)],
    up_to: Version,
) -> BTreeMap<HashValue, (StateKey, Option<StateValue>, Version)> {
    let mut expected = BTreeMap::new();
    let mut version: Version = 0;
    for (txns, _li) in blocks {
        for txn in txns {
            if version <= up_to {
                for (state_key, state_value) in txn.write_set().state_update_refs() {
                    expected.insert(
                        state_key.hash(),
                        (state_key.clone(), state_value.cloned(), version),
                    );
                }
            }
            version += 1;
        }
    }
    expected
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(20))]

    /// Commits proptest-generated blocks through the real write path (which populates both the
    /// hot state KV DB and the hot state Merkle tree), then checks the read APIs against an
    /// independently computed model of the hot state at the latest checkpoint.
    #[test]
    fn test_hot_state_value_chunk_iter(
        blocks in arb_blocks_to_commit_with_params(
            10,    /* num_accounts */
            3,     /* max_user_txns_per_block */
            2,     /* min_blocks */
            5,     /* max_blocks */
            false, /* make_hot_in_epilogue */
        ),
        chunk_size in 1usize..=8,
    ) {
        let tmp = TempPath::new();
        let db = AptosDB::new_for_test(&tmp);

        let mut version = 0;
        for (txns, li) in &blocks {
            db.save_transactions_for_test(txns, version, Some(li), true).unwrap();
            version += txns.len() as Version;
        }

        let ckpt = db.get_latest_state_checkpoint_version().unwrap().unwrap();
        let expected = expected_hot_state(&blocks, ckpt);

        // Item count matches the model.
        prop_assert_eq!(db.get_hot_state_item_count(ckpt).unwrap(), expected.len());

        // A single full pass yields every key (in key-hash order) with the right value and
        // `hot_since_version`.
        let full = collect_chunk(&db, ckpt, 0, usize::MAX);
        prop_assert_eq!(full.len(), expected.len());
        for ((k, hsv), (key, value, ver)) in full.iter().zip(expected.values()) {
            prop_assert_eq!(k, key);
            prop_assert_eq!(hsv.value_opt(), value.as_ref());
            prop_assert_eq!(hsv.hot_since_version(), *ver);
        }

        // Paging in `chunk_size`-sized steps reconstructs the same sequence.
        prop_assert_eq!(collect_paged(&db, ckpt, chunk_size), full);

        // Reading at/after the end yields nothing.
        prop_assert!(collect_chunk(&db, ckpt, expected.len(), usize::MAX).is_empty());
    }
}
