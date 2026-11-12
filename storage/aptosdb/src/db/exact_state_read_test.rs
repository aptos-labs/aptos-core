// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use super::BlockRangeCache;
use crate::{
    db::AptosDB, pruner::PrunerManager, schema::state_value_by_key_hash::StateValueByKeyHashSchema,
};
use aptos_crypto::{hash::CryptoHash, HashValue};
use aptos_storage_interface::{AptosDbError, DbReader};
use aptos_temppath::TempPath;
use aptos_types::{
    account_address::AccountAddress,
    account_config::{new_block_event_key, NewBlockEvent},
    contract_event::ContractEvent,
    state_store::{state_key::StateKey, state_value::StateValue},
    transaction::{
        ExecutionStatus, Transaction, TransactionAuxiliaryData, TransactionInfo,
        TransactionToCommit, Version,
    },
    validator_txn::ValidatorTransaction,
    write_set::WriteSet,
};
use move_core_types::{language_storage::TypeTag, move_resource::MoveStructType};
use proptest::{
    collection::{btree_map, vec},
    option,
    prelude::*,
};
use std::collections::BTreeMap;

#[test]
fn test_block_range_cache() {
    let cache = BlockRangeCache::new(3);
    assert_eq!(cache.get(0), None);

    cache.insert_blocks(0, &[0, 5, 9]);
    assert_eq!(cache.get(0), Some((0, 0)));
    assert_eq!(cache.get(1), Some((1, 5)));
    assert_eq!(cache.get(5), Some((1, 5)));
    assert_eq!(cache.get(9), Some((6, 9)));
    assert_eq!(cache.get(10), None);

    // Evicts the least recently used, [0, 0].
    cache.insert(20, 30);
    assert_eq!(cache.get(0), None);
    assert_eq!(cache.get(15), None);
    assert_eq!(cache.get(25), Some((20, 30)));

    // [1, 5] was used last, so [6, 9] goes next.
    assert_eq!(cache.get(3), Some((1, 5)));
    cache.insert(31, 40);
    assert_eq!(cache.get(7), None);
    assert_eq!(cache.get(3), Some((1, 5)));
    assert_eq!(cache.get(40), Some((31, 40)));

    // Inserting an existing start replaces its end and evicts nothing.
    cache.insert(1, 4);
    assert_eq!(cache.get(5), None);
    assert_eq!(cache.get(4), Some((1, 4)));
    assert_eq!(cache.get(25), Some((20, 30)));
    assert_eq!(cache.get(31), Some((31, 40)));
}

const NUM_KEYS: usize = 4;

#[derive(Clone, Debug)]
struct TestBlock {
    has_block_event: bool,
    /// Writes of each transaction, key index -> value. `None` value means deletion.
    txns: Vec<BTreeMap<usize, Option<u8>>>,
}

fn arb_block() -> impl Strategy<Value = TestBlock> {
    (
        any::<bool>(),
        vec(
            btree_map(0..NUM_KEYS, option::weighted(0.7, any::<u8>()), 0..=3),
            1..=6,
        ),
    )
        .prop_map(|(has_block_event, txns)| TestBlock {
            has_block_event,
            txns,
        })
}

fn key(index: usize) -> StateKey {
    StateKey::raw(&[index as u8])
}

fn value(byte: u8) -> StateValue {
    StateValue::new_legacy(vec![byte].into())
}

fn new_block_event(height: u64) -> ContractEvent {
    let event = NewBlockEvent::new(
        AccountAddress::ZERO,
        0,      /* epoch */
        height, /* round */
        height,
        vec![], /* previous_block_votes_bitvec */
        AccountAddress::ZERO,
        vec![], /* failed_proposer_indices */
        0,      /* timestamp */
    );
    ContractEvent::new_v1(
        new_block_event_key(),
        height,
        TypeTag::Struct(Box::new(NewBlockEvent::struct_tag())),
        bcs::to_bytes(&event).unwrap(),
    )
    .unwrap()
}

/// The last transaction is the block's state checkpoint, and the first carries the
/// `NewBlockEvent` if any.
fn block_txns(block: &TestBlock, height: u64) -> Vec<TransactionToCommit> {
    let last_index = block.txns.len() - 1;
    block
        .txns
        .iter()
        .enumerate()
        .map(|(index, writes)| {
            let is_checkpoint = index == last_index;
            let transaction = if is_checkpoint {
                Transaction::StateCheckpoint(HashValue::random())
            } else {
                Transaction::ValidatorTransaction(ValidatorTransaction::dummy(vec![]))
            };
            // No auxiliary info hash, so committing doesn't need to resolve auxiliary info.
            let transaction_info = TransactionInfo::builder_v0()
                .transaction_hash(HashValue::random())
                .state_change_hash(HashValue::random())
                .event_root_hash(HashValue::random())
                .maybe_state_checkpoint_hash(is_checkpoint.then(HashValue::random))
                .gas_used(0)
                .status(ExecutionStatus::Success)
                .build();
            let events = if index == 0 && block.has_block_event {
                vec![new_block_event(height)]
            } else {
                vec![]
            };
            TransactionToCommit::new(
                transaction,
                transaction_info,
                WriteSet::new_for_test(
                    writes
                        .iter()
                        .map(|(&key_index, &byte)| (key(key_index), byte.map(value))),
                ),
                events,
                false, /* is_reconfig */
                TransactionAuxiliaryData::default(),
            )
        })
        .collect()
}

struct TestChain {
    blocks: Vec<TestBlock>,
    /// `(first_version, last_version)` of each block.
    block_ranges: Vec<(Version, Version)>,
    /// Writes to each key in version order.
    writes: Vec<Vec<(Version, Option<StateValue>)>>,
}

impl TestChain {
    fn new(blocks: Vec<TestBlock>) -> Self {
        let mut block_ranges = Vec::new();
        let mut writes = vec![Vec::new(); NUM_KEYS];
        let mut version = 0;
        for block in &blocks {
            let first_version = version;
            for txn in &block.txns {
                for (&key_index, &byte) in txn {
                    writes[key_index].push((version, byte.map(value)));
                }
                version += 1;
            }
            block_ranges.push((first_version, version - 1));
        }
        Self {
            blocks,
            block_ranges,
            writes,
        }
    }

    fn last_version(&self) -> Version {
        self.block_ranges.last().unwrap().1
    }

    /// Commits the blocks, ending a chunk after each block where `chunk_ends` is true.
    fn commit(&self, db: &AptosDB, chunk_ends: &[bool]) {
        let mut chunk = Vec::new();
        let mut first_version = 0;
        for (height, block) in self.blocks.iter().enumerate() {
            chunk.extend(block_txns(block, height as u64));
            if height == self.blocks.len() - 1 || chunk_ends[height] {
                db.save_transactions_for_test(&chunk, first_version, None, true)
                    .unwrap();
                first_version += chunk.len() as Version;
                chunk.clear();
            }
        }
    }

    fn expected_value(&self, key_index: usize, version: Version) -> Option<(Version, StateValue)> {
        self.writes[key_index]
            .iter()
            .rev()
            .find(|(write_version, _)| *write_version <= version)
            .and_then(|(write_version, value)| value.clone().map(|value| (*write_version, value)))
    }

    fn block_index(&self, version: Version) -> usize {
        self.block_ranges
            .iter()
            .position(|(first, last)| (*first..=*last).contains(&version))
            .unwrap()
    }

    /// The range resolved for `version`. The block index spans from the latest block with a
    /// `NewBlockEvent` to the next one, which only matches a block if all blocks have one.
    fn expected_block_range(&self, version: Version) -> (Version, Version) {
        let index = self.block_index(version);
        let Some(start_index) = self.blocks[..=index]
            .iter()
            .rposition(|block| block.has_block_event)
        else {
            return self.block_ranges[index];
        };
        let end = match self.blocks[index + 1..]
            .iter()
            .position(|block| block.has_block_event)
        {
            Some(offset) => self.block_ranges[index + 1 + offset].0 - 1,
            None => self.block_ranges[index].1,
        };
        (self.block_ranges[start_index].0, end)
    }

    /// Deletes the rows of all but the last write to each key in each block.
    fn squash_rows(&self, db: &AptosDB) {
        for (key_index, writes) in self.writes.iter().enumerate() {
            let key = key(key_index);
            for pair in writes.windows(2) {
                let (version, next_version) = (pair[0].0, pair[1].0);
                if self.block_index(version) == self.block_index(next_version) {
                    db.state_kv_db
                        .db_shard(key.get_shard_id())
                        .delete::<StateValueByKeyHashSchema>(&(key.hash(), version))
                        .unwrap();
                }
            }
        }
    }

    fn assert_reads(&self, db: &AptosDB) {
        for version in 0..=self.last_version() {
            for key_index in 0..NUM_KEYS {
                let expected = self.expected_value(key_index, version);
                let key = key(key_index);
                assert_eq!(
                    db.get_state_value_with_version_by_version(&key, version)
                        .unwrap(),
                    expected,
                    "key {key_index}, version {version}",
                );
                assert_eq!(
                    db.get_state_value_by_version(&key, version).unwrap(),
                    expected.map(|(_, value)| value),
                    "key {key_index}, version {version}",
                );
            }
        }
    }
}

fn check_exact_reads(
    blocks: Vec<TestBlock>,
    chunk_ends: Vec<bool>,
    prune_index: prop::sample::Index,
) {
    let chain = TestChain::new(blocks);
    let tmp_dir = TempPath::new();
    let db = AptosDB::new_for_test(&tmp_dir);
    chain.commit(&db, &chunk_ends);

    // Rows per version, where a plain seek is the answer.
    for version in 0..=chain.last_version() {
        for key_index in 0..NUM_KEYS {
            assert_eq!(
                db.state_kv_db
                    .get_state_value_with_version_by_version(&key(key_index), version)
                    .unwrap(),
                chain.expected_value(key_index, version),
            );
        }
    }
    // Ranges cached at pre-commit.
    chain.assert_reads(&db);
    db.block_range_cache.clear();
    chain.assert_reads(&db);

    db.block_range_cache.clear();
    for version in 0..=chain.last_version() {
        assert_eq!(
            db.get_block_range(version).unwrap(),
            chain.expected_block_range(version),
            "version {version}",
        );
    }

    // Rows per block.
    chain.squash_rows(&db);
    chain.assert_reads(&db);
    db.block_range_cache.clear();
    chain.assert_reads(&db);

    let min_readable_version = prune_index.index(chain.last_version() as usize + 1) as Version;
    db.ledger_pruner
        .save_min_readable_version(min_readable_version)
        .unwrap();
    db.state_store
        .state_pruner
        .state_kv_pruner
        .save_min_readable_version(min_readable_version)
        .unwrap();
    db.block_range_cache.clear();
    for version in min_readable_version..=chain.last_version() {
        let (start, _) = chain.expected_block_range(version);
        let is_block_end = chain.block_ranges[chain.block_index(version)].1 == version;
        for key_index in 0..NUM_KEYS {
            match db.get_state_value_with_version_by_version(&key(key_index), version) {
                Ok(value) => assert_eq!(
                    value,
                    chain.expected_value(key_index, version),
                    "key {key_index}, version {version}",
                ),
                // Only reads that need the pruned part of their block.
                Err(AptosDbError::LedgerPruned { .. }) => assert!(
                    start <= min_readable_version && !is_block_end,
                    "key {key_index}, version {version}",
                ),
                Err(err) => panic!("key {key_index}, version {version}: {err}"),
            }
        }
    }
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(32))]

    #[test]
    fn test_exact_reads(
        blocks in vec(arb_block(), 1..=10),
        chunk_ends in vec(any::<bool>(), 10),
        prune_index in any::<prop::sample::Index>(),
    ) {
        check_exact_reads(blocks, chunk_ends, prune_index);
    }
}
