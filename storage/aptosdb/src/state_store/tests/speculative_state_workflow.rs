// Copyright (c) Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{db::test_helper::arb_key_universe, state_store::persisted_state::PersistedState};
use aptos_block_executor::hot_state_op_accumulator::BlockHotStateOpAccumulator;
use aptos_crypto::{hash::CryptoHash, HashValue};
use aptos_infallible::Mutex;
use aptos_scratchpad::test_utils::naive_smt::NaiveSmt;
use aptos_storage_interface::{
    state_store::{
        state::{LedgerState, State},
        state_summary::{LedgerStateSummary, ProvableStateSummary, StateSummary},
        state_update_refs::StateUpdateRefs,
        state_view::cached_state_view::CachedStateView,
        state_with_summary::{LedgerStateWithSummary, StateWithSummary},
    },
    DbReader, Result as DbResult,
};
use aptos_types::{
    proof::SparseMerkleProofExt,
    state_store::{
        hot_state::{HotStateConfig, LRUEntry},
        state_key::StateKey,
        state_slot::StateSlot,
        state_storage_usage::StateStorageUsage,
        state_value::StateValue,
        StateViewId, StateViewResult, TStateView, NUM_STATE_SHARDS,
    },
    transaction::Version,
    write_set::{BaseStateOp, HotStateOp, WriteOp},
};
use itertools::Itertools;
use lru::LruCache;
use proptest::{
    collection::{hash_set, vec},
    num,
    prelude::*,
    sample::Index,
};
use rayon::prelude::*;
use std::{
    collections::{BTreeMap, BTreeSet, HashMap, HashSet},
    fmt::{Debug, Formatter},
    num::NonZeroUsize,
    ops::Deref,
    sync::{
        mpsc::{channel, Receiver, Sender},
        Arc,
    },
    thread::spawn,
};

const NUM_KEYS: usize = 96;
const HOT_STATE_MAX_ITEMS_PER_SHARD: usize = NUM_KEYS / 16 / 2;
const MAX_PROMOTIONS_PER_BLOCK: usize = 10;
const REFRESH_INTERVAL_VERSIONS: usize = 50;

const TEST_CONFIG: HotStateConfig = HotStateConfig {
    max_items_per_shard: HOT_STATE_MAX_ITEMS_PER_SHARD,
};

#[derive(Debug)]
struct UserTxn {
    reads: BTreeSet<StateKey>,
    writes: BTreeMap<StateKey, Option<StateValue>>,
}

#[derive(Debug)]
struct Txn {
    reads: BTreeSet<StateKey>,
    write_set: BTreeMap<StateKey, BaseStateOp>,
    is_checkpoint: bool,
}

#[ouroboros::self_referencing]
struct Chunk {
    txns: Vec<Txn>,
    #[borrows(txns)]
    #[covariant]
    update_refs: StateUpdateRefs<'this>,
}

impl Chunk {
    fn from_txns(txns: Vec<Txn>, first_version: Version) -> Self {
        ChunkBuilder {
            txns,
            update_refs_builder: |txn_outs| {
                StateUpdateRefs::index(
                    first_version,
                    txn_outs.iter().map(|t| t.write_set.iter()),
                    txn_outs.len(),
                    txn_outs.iter().positions(|t| t.is_checkpoint).collect(),
                )
            },
        }
        .build()
    }

    fn all_reads(&self) -> impl Iterator<Item = &StateKey> {
        self.borrow_txns().iter().flat_map(|t| &t.reads)
    }

    fn update_refs(&self) -> &StateUpdateRefs<'_> {
        self.borrow_update_refs()
    }
}

impl Debug for Chunk {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "Block")
    }
}

prop_compose! {
    pub fn arb_user_block(
        keys: Vec<StateKey>,
        max_read_only_set_size: usize,
        max_write_set_size: usize,
        max_block_size: usize,
    )(
        input in vec(
            (
                vec(
                    any::<Index>(),
                    0..=max_read_only_set_size,
                ),
                vec(
                    any::<(Index, Option<StateValue>)>(),
                    1..=max_write_set_size,
                ),
            ),
            1..=max_block_size
        ),
    ) -> Vec<UserTxn> {
        input
            .into_iter()
            .map(|(reads, writes)| {
                let write_set: BTreeMap<_, _> = writes
                    .into_iter()
                    .map(|(idx, value)| (idx.get(&keys).clone(), value))
                    .collect();

                // The read set is a super set of the write set.
                let read_set: BTreeSet<_> = write_set
                    .keys()
                    .cloned()
                    .chain(reads.iter().map(|idx| idx.get(&keys)).cloned())
                    .collect();

                UserTxn {
                    reads: read_set,
                    writes: write_set,
                }
            })
            .collect_vec()
    }
}

#[derive(Clone)]
struct VersionState {
    usage: StateStorageUsage,
    hot_state: [LruCache<StateKey, StateSlot>; NUM_STATE_SHARDS],
    state: HashMap<StateKey, (Version, StateValue)>,
    summary: NaiveSmt,
    next_version: Version,
}

impl VersionState {
    fn new_empty() -> Self {
        Self {
            usage: StateStorageUsage::zero(),
            hot_state: [(); NUM_STATE_SHARDS].map(|_| LruCache::unbounded()),
            state: HashMap::new(),
            summary: NaiveSmt::default(),
            next_version: 0,
        }
    }

    fn update<'a>(
        &self,
        version: Version,
        writes: impl IntoIterator<Item = (&'a StateKey, Option<&'a StateValue>)>,
        promotions: impl IntoIterator<Item = &'a StateKey>,
        is_checkpoint: bool,
    ) -> Self {
        assert_eq!(version, self.next_version);

        let mut hot_state = self.hot_state.clone();
        let mut state = self.state.clone();
        let mut smt_updates = vec![];

        for (k, v_opt) in writes {
            let shard_id = k.get_shard_id();
            match v_opt {
                None => {
                    let slot = StateSlot::HotVacant {
                        hot_since_version: version,
                        lru_info: LRUEntry::uninitialized(),
                    };
                    hot_state[shard_id].put(k.clone(), slot);
                    state.remove(k);
                    smt_updates.push((k.hash(), None));
                },
                Some(v) => {
                    let slot = StateSlot::HotOccupied {
                        value_version: version,
                        value: v.clone(),
                        hot_since_version: version,
                        lru_info: LRUEntry::uninitialized(),
                    };
                    hot_state[shard_id].put(k.clone(), slot);
                    state.insert(k.clone(), (version, v.clone()));
                    smt_updates.push((k.hash(), Some(v.hash())));
                },
            }
        }

        for k in promotions {
            let shard_id = k.get_shard_id();
            if let Some(slot) = hot_state[shard_id].get_mut(k) {
                slot.refresh(version);
                continue;
            }
            let slot = match state.get(k) {
                Some((value_version, value)) => StateSlot::HotOccupied {
                    value_version: *value_version,
                    value: value.clone(),
                    hot_since_version: version,
                    lru_info: LRUEntry::uninitialized(),
                },
                None => StateSlot::HotVacant {
                    hot_since_version: version,
                    lru_info: LRUEntry::uninitialized(),
                },
            };
            hot_state[shard_id].put(k.clone(), slot);
        }

        if is_checkpoint {
            println!(
                "Evicting now. Version: {version}. Before size: {:?}",
                hot_state.iter().map(|shard| shard.len()).collect_vec()
            );
            for shard in hot_state.iter_mut() {
                while shard.len() > HOT_STATE_MAX_ITEMS_PER_SHARD {
                    shard.pop_lru();
                }
            }
            println!(
                "After eviction. After size: {:?}",
                hot_state.iter().map(|shard| shard.len()).collect_vec()
            );
        }

        let summary = self.summary.clone().update(&smt_updates);

        let items = state.len();
        let bytes = state.iter().map(|(k, v)| k.size() + v.1.size()).sum();
        let usage = StateStorageUsage::new(items, bytes);

        Self {
            usage,
            hot_state,
            state,
            summary,
            next_version: version + 1,
        }
    }
}

impl TStateView for VersionState {
    type Key = StateKey;

    fn get_state_slot(&self, key: &Self::Key) -> StateViewResult<StateSlot> {
        let from_cold = StateSlot::from_db_get(self.state.get(key).cloned());
        let shard_id = key.get_shard_id();
        let slot = match self.hot_state[shard_id].peek(key) {
            Some(slot) => {
                assert_eq!(slot.as_state_value_opt(), from_cold.as_state_value_opt());
                slot.clone()
            },
            None => from_cold,
        };
        Ok(slot)
    }

    fn get_usage(&self) -> StateViewResult<StateStorageUsage> {
        Ok(self.usage)
    }

    fn next_version(&self) -> Version {
        self.next_version
    }
}

struct StateByVersion {
    state_by_next_version: Vec<Arc<VersionState>>,
}

impl StateByVersion {
    pub fn get_state(&self, version: Option<Version>) -> &VersionState {
        let next_version = version.map_or(0, |ver| ver + 1);
        &self.state_by_next_version[next_version as usize]
    }

    fn new_empty() -> Self {
        Self {
            state_by_next_version: vec![Arc::new(VersionState::new_empty())],
        }
    }

    fn append_version<'a>(
        &mut self,
        writes: impl IntoIterator<Item = (&'a StateKey, Option<&'a StateValue>)>,
        promotions: impl IntoIterator<Item = &'a StateKey>,
        is_checkpoint: bool,
    ) {
        self.state_by_next_version.push(Arc::new(
            self.state_by_next_version.last().unwrap().update(
                self.next_version(),
                writes,
                promotions,
                is_checkpoint,
            ),
        ));
    }

    fn next_version(&self) -> Version {
        self.state_by_next_version.len() as Version - 1
    }

    fn assert_state_slot(slot1: &StateSlot, slot2: &StateSlot) {
        match (slot1, slot2) {
            (
                StateSlot::HotVacant {
                    hot_since_version: v1,
                    ..
                },
                StateSlot::HotVacant {
                    hot_since_version: v2,
                    ..
                },
            ) => assert_eq!(v1, v2),
            (
                StateSlot::HotOccupied {
                    value_version: vv1,
                    value: v1,
                    hot_since_version: h1,
                    ..
                },
                StateSlot::HotOccupied {
                    value_version: vv2,
                    value: v2,
                    hot_since_version: h2,
                    ..
                },
            ) => {
                assert_eq!(vv1, vv2);
                assert_eq!(v1, v2);
                assert_eq!(h1, h2);
            },
            (s1, s2) => assert_eq!(s1, s2),
        }
    }

    fn assert_state(&self, state: &State) {
        assert_eq!(state.usage(), self.get_state(state.version()).usage);
    }

    pub fn assert_ledger_state(&self, ledger_state: &LedgerState) {
        self.assert_state(ledger_state.last_checkpoint());
        self.assert_state(ledger_state.latest());
    }

    fn assert_state_summary(&self, state_summary: &StateSummary) {
        assert_eq!(
            state_summary.root_hash(),
            self.get_state(state_summary.version())
                .summary
                .get_root_hash()
        );
    }

    pub fn assert_ledger_state_summary(&self, ledger_state_summary: &LedgerStateSummary) {
        self.assert_state_summary(ledger_state_summary.last_checkpoint());
        self.assert_state_summary(ledger_state_summary.latest());
    }

    pub fn assert_jmt_updates(
        &self,
        last_snapshot: &StateWithSummary,
        snapshot: &StateWithSummary,
    ) {
        let base_state = self.get_state(last_snapshot.version()).clone();
        let result_state = self.get_state(snapshot.version()).clone();
        assert_eq!(
            result_state.summary.get_root_hash(),
            snapshot.summary().root_hash()
        );

        let jmt_updates = snapshot
            .make_delta(last_snapshot)
            .shards
            .iter()
            .flat_map(|shard| shard.iter())
            .filter_map(|(key, slot)| slot.maybe_update_jmt(key, last_snapshot.next_version()))
            .map(|(key_hash, value_opt)| (key_hash, value_opt.map(|(val_hash, _key)| val_hash)))
            .collect_vec();

        let base_kv_hashes: HashSet<_> = base_state.summary.leaves.iter().collect();
        let result_kv_hashes: HashSet<_> = result_state.summary.leaves.iter().collect();
        let base_keys: HashSet<_> = base_kv_hashes.iter().map(|(k, _v)| k).collect();
        let result_keys: HashSet<_> = result_kv_hashes.iter().map(|(k, _v)| k).collect();

        let updated_keys: HashSet<_> = jmt_updates
            .iter()
            .filter_map(|(key, value_opt)| {
                value_opt.and_then(|val| (!base_kv_hashes.contains(&(*key, val))).then_some(key))
            })
            .collect();
        let deleted_keys: HashSet<_> = jmt_updates
            .iter()
            .filter_map(|(key, value_opt)| value_opt.is_none().then_some(key))
            .filter(|k| base_keys.contains(*k))
            .collect();

        let expected_updated_keys: HashSet<_> = result_kv_hashes
            .difference(&base_kv_hashes)
            .map(|(k, _v)| k)
            .collect();
        let expected_deleted_keys: HashSet<_> =
            base_keys.difference(&result_keys).cloned().collect();

        if updated_keys != expected_updated_keys {
            let excess = updated_keys
                .difference(&expected_updated_keys)
                .collect_vec();
            let missing = expected_updated_keys
                .difference(&updated_keys)
                .collect_vec();
            eprintln!(
                "bad updated keys: excess: {:?}, missing: {:?}",
                excess, missing
            );
        } else {
            // eprintln!("updated keys good");
        }

        if deleted_keys != expected_deleted_keys {
            let excess = deleted_keys
                .difference(&expected_deleted_keys)
                .collect_vec();
            let missing = expected_deleted_keys
                .difference(&deleted_keys)
                .collect_vec();
            eprintln!(
                "bad deleted keys: excess: {:?}, missing: {:?}",
                excess, missing
            );
        } else {
            // eprintln!("deleted keys good")
        }

        let new_summary = self
            .get_state(last_snapshot.version())
            .summary
            .clone()
            .update(&jmt_updates);

        assert_eq!(new_summary.get_root_hash(), snapshot.summary().root_hash());
    }
}

impl DbReader for StateByVersion {
    fn get_state_value_with_version_by_version(
        &self,
        state_key: &StateKey,
        version: Version,
    ) -> DbResult<Option<(Version, StateValue)>> {
        Ok(self.get_state(Some(version)).state.get(state_key).cloned())
    }

    fn get_state_proof_by_version_ext(
        &self,
        key_hash: &HashValue,
        version: Version,
        _root_depth: usize,
    ) -> DbResult<SparseMerkleProofExt> {
        Ok(self.get_state(Some(version)).summary.get_proof(key_hash))
    }
}

fn update_state(
    blocks: Vec<Chunk>,
    state_by_version: Arc<StateByVersion>,
    empty: LedgerStateWithSummary,
    persisted_state: PersistedState,
    to_summary_update: Sender<(Chunk, LedgerState)>,
) {
    let pool = rayon::ThreadPoolBuilder::new()
        .num_threads(4)
        .build()
        .unwrap();
    let mut parent_state = empty.ledger_state();

    for block in blocks {
        let (hot_state, persisted_state) = persisted_state.get_state();
        let state_view = CachedStateView::new_with_config(
            StateViewId::Miscellaneous,
            state_by_version.clone(),
            hot_state.clone(),
            persisted_state.clone(),
            parent_state.deref().clone(),
        );
        let read_keys = block.all_reads().collect_vec();
        pool.install(|| {
            read_keys.par_iter().for_each(|k| {
                let value = state_view.get_state_value(k).unwrap();
                let expected_value = parent_state.version().and_then(|version| {
                    state_by_version
                        .get_state_value_with_version_by_version(k, version)
                        .unwrap()
                        .map(|(_ver, val)| val)
                });
                assert_eq!(value, expected_value)
            });
        });
        let memorized_reads = state_view.into_memorized_reads();

        let next_state = parent_state.update_with_memorized_reads(
            hot_state.clone(),
            &persisted_state,
            block.update_refs(),
            &memorized_reads,
        );

        state_by_version.assert_ledger_state(&next_state);

        parent_state = next_state.clone();

        to_summary_update
            .send((block, next_state))
            .expect("send() failed.");
    }

    // inform downstream to quit
    drop(to_summary_update)
}

fn update_state_summary(
    state_by_version: Arc<StateByVersion>,
    empty: LedgerStateWithSummary,
    persisted_state: PersistedState,
    from_state_update: Receiver<(Chunk, LedgerState)>,
    to_db_commit: Sender<LedgerStateWithSummary>,
) {
    let mut parent_summary = empty.ledger_state_summary();

    while let Ok((block, ledger_state)) = from_state_update.recv() {
        let persisted_summary = persisted_state.get_state_summary();

        let next_summary = parent_summary
            .update(
                &ProvableStateSummary::new(persisted_summary, state_by_version.as_ref()),
                block.update_refs(),
            )
            .unwrap();

        state_by_version.assert_ledger_state_summary(&next_summary);
        parent_summary = next_summary.clone();

        let ledger_state_with_summary =
            LedgerStateWithSummary::from_state_and_summary(ledger_state, next_summary);

        to_db_commit.send(ledger_state_with_summary).unwrap();
    }

    // inform downstream to quit
    drop(to_db_commit);
}

fn send_to_state_buffer(
    empty: LedgerStateWithSummary,
    state_by_version: Arc<StateByVersion>,
    from_summary_update: Receiver<LedgerStateWithSummary>,
    to_buffered_state_commit: Sender<StateWithSummary>,
    current_state: Arc<Mutex<LedgerStateWithSummary>>,
) {
    let mut last_snapshot = empty.last_checkpoint().clone();

    while let Ok(ledger_state_with_summary) = from_summary_update.recv() {
        *current_state.lock() = ledger_state_with_summary.clone();
        let snapshot = ledger_state_with_summary.last_checkpoint();
        println!(
            "snapshot.version(): {:?}. last_snapshot.version(): {:?}",
            snapshot.version(),
            last_snapshot.version()
        );
        if let Some(checkpoint_version) = snapshot.version() {
            if checkpoint_version % 7 == 0 && Some(checkpoint_version) != last_snapshot.version() {
                state_by_version.assert_jmt_updates(&last_snapshot, snapshot);

                last_snapshot = snapshot.clone();
                to_buffered_state_commit.send(snapshot.clone()).unwrap();
            }
        }
    }

    // inform downstream to quit
    drop(to_buffered_state_commit);
}

fn commit_state_buffer(
    from_buffered_state_commit: Receiver<StateWithSummary>,
    persisted_state: PersistedState,
) {
    while let Ok(snapshot) = from_buffered_state_commit.recv() {
        println!("got snapshot. next_version: {:?}", snapshot.next_version());
        persisted_state.set(snapshot);
    }
}

fn naive_run_blocks(blocks: Vec<(Vec<UserTxn>, bool)>) -> (Vec<Txn>, StateByVersion) {
    let mut all_txns = vec![];
    let mut state_by_version = StateByVersion::new_empty();
    let mut current_version = 0;
    for (block_txns, append_epilogue) in blocks {
        let mut op_accu = BlockHotStateOpAccumulator::<StateKey>::new_with_config(
            MAX_PROMOTIONS_PER_BLOCK,
            REFRESH_INTERVAL_VERSIONS,
        );
        let num_txns = block_txns.len();
        for (idx, txn) in block_txns.into_iter().enumerate() {
            // No promotions except for block epilogue.
            state_by_version.append_version(
                txn.writes.iter().map(|(k, v)| (k, v.as_ref())),
                vec![],
                !append_epilogue && idx + 1 == num_txns,
            );
            op_accu.add_transaction(txn.writes.keys(), txn.reads.iter());
            all_txns.push(Txn {
                reads: txn.reads,
                write_set: txn
                    .writes
                    .into_iter()
                    .map(|(k, v_opt)| match v_opt {
                        None => (k, WriteOp::legacy_deletion().into_base_op()),
                        Some(v) => (k, WriteOp::modification_to_value(v).into_base_op()),
                    })
                    .collect(),
                is_checkpoint: false,
            });
        }
        if append_epilogue {
            let to_make_hot = op_accu.get_keys_to_make_hot();
            state_by_version.append_version(vec![], to_make_hot.iter(), true);

            let reads = to_make_hot.clone();
            let write_set = to_make_hot
                .into_iter()
                .map(|k| (k, HotStateOp::make_hot().into_base_op()))
                .collect();
            all_txns.push(Txn {
                reads,
                write_set,
                is_checkpoint: true,
            });
        }
    }

    (all_txns, state_by_version)
}

fn replay_chunks_pipelined(chunks: Vec<Chunk>, state_by_version: Arc<StateByVersion>) {
    let empty = LedgerStateWithSummary::new_empty(TEST_CONFIG);
    let current_state = Arc::new(Mutex::new(empty.clone()));

    let persisted_state = PersistedState::new_empty_with_config(TEST_CONFIG);
    persisted_state.hack_reset(empty.deref().clone());

    let (to_summary_update, from_state_update) = channel();
    let (to_db_commit, from_summary_update) = channel();
    let (to_buffered_state_commit, from_buffered_state_commit) = channel();

    let mut threads = vec![];

    {
        let empty = empty.clone();
        let state_by_version = state_by_version.clone();
        let persisted_state = persisted_state.clone();
        threads.push(spawn(move || {
            update_state(
                chunks,
                state_by_version,
                empty,
                persisted_state,
                to_summary_update,
            );
        }));
    }

    {
        let empty = empty.clone();
        let state_by_version = state_by_version.clone();
        let persisted_state = persisted_state.clone();
        threads.push(spawn(move || {
            update_state_summary(
                state_by_version,
                empty.clone(),
                persisted_state,
                from_state_update,
                to_db_commit,
            );
        }));
    }

    {
        let empty = empty.clone();
        let state_by_version = state_by_version.clone();
        let current_state = current_state.clone();
        threads.push(spawn(move || {
            send_to_state_buffer(
                empty,
                state_by_version,
                from_summary_update,
                to_buffered_state_commit,
                current_state,
            );
        }));
    }

    {
        let persisted_state = persisted_state.clone();
        threads.push(spawn(move || {
            println!("begin commit_state_buffer thread");
            commit_state_buffer(from_buffered_state_commit, persisted_state);
            println!("done commit_state_buffer thread");
        }));
    }

    threads
        .into_iter()
        .for_each(|t| t.join().expect("join() failed."));

    let hot_state = persisted_state.get_hot_state();
    hot_state.drain_pending_commits();
    let all_entries_by_shard = hot_state.get_all_entries();

    let last_version = state_by_version.next_version() - 1;
    let naive_all_entries_by_shard: [BTreeMap<_, _>; NUM_STATE_SHARDS] =
        std::array::from_fn(|shard_id| {
            state_by_version.get_state(Some(last_version)).hot_state[shard_id]
                .iter()
                .map(|(k, v)| (k.clone(), v.clone()))
                .collect()
        });

    for shard_id in 0..NUM_STATE_SHARDS {
        println!("shard id: {shard_id}");
        let all_entries = &all_entries_by_shard[shard_id];
        let naive_all_entries = &naive_all_entries_by_shard[shard_id];
        assert_eq!(all_entries.len(), naive_all_entries.len());

        println!("ACTUAL:");
        for key in all_entries.keys() {
            println!("\t{:?}", key);
        }
        println!("EXPECTED:");
        for key in naive_all_entries.keys() {
            println!("\t{:?}", key);
        }
        for (key, slot) in all_entries {
            assert!(naive_all_entries.contains_key(key));
            let slot2 = naive_all_entries.get(key).unwrap();
            StateByVersion::assert_state_slot(slot, slot2);
        }
    }
}

fn arb_keys(num_keys: usize) -> impl Strategy<Value = Vec<StateKey>> {
    hash_set(
        "[a-z]{1,5}".prop_map(|raw| StateKey::raw(raw.as_bytes())),
        num_keys,
    )
    .prop_map(|hs| hs.into_iter().collect_vec())
    .boxed()
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(20))]

    #[test]
    fn test_speculative_state_workflow(
    (mut blocks, last_block) in arb_keys(NUM_KEYS)
            .prop_flat_map(move |keys| {
            (
                vec((
                    arb_user_block(keys.clone(), NUM_KEYS, NUM_KEYS, NUM_KEYS),
                    prop_oneof![1=>Just(false), 9=>Just(true)]
                ), 1..100),
                arb_user_block(keys, NUM_KEYS, NUM_KEYS, NUM_KEYS)
            )
            })
    ) {
        println!("is checkpoint: {:?}", blocks.iter().map(|(_, is)| *is).collect_vec());
        blocks.push((last_block, true));

        let (all_txns, state_by_version) = naive_run_blocks(blocks);
        println!("total versions: {}", state_by_version.next_version());

        let mut chunks = vec![];
        let mut next_version = 0;
        for chunk in &all_txns.into_iter().chunks(NUM_KEYS / 2) {
            let chunk: Vec<Txn> = chunk.collect();
            let first_version = next_version;
            next_version += chunk.len() as Version;
            println!("chunk range: [{}, {})", first_version, next_version);
            chunks.push(Chunk::from_txns(chunk, first_version));
        }

        replay_chunks_pipelined(chunks, Arc::new(state_by_version));
    }
}
