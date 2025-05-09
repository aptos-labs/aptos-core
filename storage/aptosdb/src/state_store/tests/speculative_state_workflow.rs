// Copyright (c) Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{db::test_helper::arb_key_universe, state_store::persisted_state::PersistedState};
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
        state_key::StateKey,
        state_storage_usage::StateStorageUsage,
        state_value::{StateValue, ARB_STATE_VALUE_MAX_SIZE},
        StateViewId, TStateView,
    },
    transaction::Version,
};
use itertools::Itertools;
use proptest::{collection::vec, prelude::*, sample::Index};
use rayon::prelude::*;
use std::{
    collections::{HashMap, HashSet},
    fmt::{Debug, Formatter},
    ops::Deref,
    sync::{
        mpsc::{channel, Receiver, Sender},
        Arc,
    },
    thread::spawn,
};

const NUM_KEYS: usize = 10;
const HOT_STATE_MAX_ITEMS: usize = NUM_KEYS / 2;
const HOT_STATE_MAX_BYTES: usize = NUM_KEYS / 2 * ARB_STATE_VALUE_MAX_SIZE / 3;
const HOT_STATE_MAX_SINGLE_VALUE_BYTES: usize = ARB_STATE_VALUE_MAX_SIZE / 2;
const HOT_ITEM_REFRESH_INTERVAL_VERSIONS: usize = 8;

#[derive(Debug)]
struct Txn {
    reads: Vec<StateKey>,
    writes: Vec<(StateKey, Option<StateValue>)>,
    is_checkpoint: bool,
}

#[ouroboros::self_referencing]
struct Block {
    txns: Vec<Txn>,
    #[borrows(txns)]
    #[covariant]
    update_refs: StateUpdateRefs<'this>,
}

impl Block {
    fn from_txns(txns: Vec<Txn>, first_version: Version) -> Self {
        BlockBuilder {
            txns,
            update_refs_builder: |txns| {
                StateUpdateRefs::index(
                    first_version,
                    txns.iter()
                        .map(|t| t.writes.iter().map(|(k, v_opt)| (k, v_opt.as_ref()))),
                    txns.len(),
                    txns.iter().rposition(|t| t.is_checkpoint),
                )
            },
        }
        .build()
    }

    fn len(&self) -> usize {
        self.borrow_txns().len()
    }

    fn all_reads(&self) -> impl Iterator<Item = &StateKey> {
        self.borrow_txns().iter().flat_map(|t| &t.reads)
    }

    fn writes_by_version(
        &self,
    ) -> impl Iterator<Item = impl Iterator<Item = (&StateKey, Option<&StateValue>)>> {
        self.borrow_txns()
            .iter()
            .map(|t| t.writes.iter().map(|(k, v)| (k, v.as_ref())))
    }

    fn update_refs(&self) -> &StateUpdateRefs {
        self.borrow_update_refs()
    }
}

impl Debug for Block {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "Block")
    }
}

prop_compose! {
    pub fn arb_block(
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
                prop_oneof![5 => Just(false), 1 => Just(true)],
            ),
            1..=max_block_size
        )
    ) -> Vec<Txn> {
        input
            .into_iter()
            .map(|(reads, writes, is_checkpoint)| {
                let write_set: HashMap<_, _> = writes
                    .into_iter()
                    .map(|(idx, value)| (idx.get(&keys).clone(), value))
                    .collect();

                // The read set is a super set of the write set.
                let read_set: HashSet<_> = write_set
                    .keys()
                    .chain(reads.iter().map(|idx| idx.get(&keys)))
                    .collect();

                Txn {
                    reads: read_set.into_iter().cloned().collect(),
                    writes: write_set.into_iter().collect(),
                    is_checkpoint
                }
            })
            .collect_vec()
    }
}

#[derive(Clone)]
struct VersionState {
    usage: StateStorageUsage,
    state: HashMap<StateKey, (Version, StateValue)>,
    summary: NaiveSmt,
}

impl VersionState {
    fn new_empty() -> Self {
        Self {
            usage: StateStorageUsage::zero(),
            state: HashMap::new(),
            summary: NaiveSmt::default(),
        }
    }

    fn update<'a>(
        &self,
        version: Version,
        kvs: impl IntoIterator<Item = (&'a StateKey, Option<&'a StateValue>)>,
    ) -> Self {
        let mut state = self.state.clone();
        let mut smt_updates = vec![];

        for (k, v_opt) in kvs.into_iter() {
            match v_opt {
                None => {
                    state.remove(k);
                    smt_updates.push((k.hash(), None));
                },
                Some(v) => {
                    state.insert(k.clone(), (version, v.clone()));
                    smt_updates.push((k.hash(), Some(v.hash())));
                },
            }
        }

        let summary = self.summary.clone().update(&smt_updates);

        let items = state.len();
        let bytes = state.iter().map(|(k, v)| k.size() + v.1.size()).sum();
        let usage = StateStorageUsage::new(items, bytes);

        Self {
            state,
            summary,
            usage,
        }
    }
}

struct StateByVersion {
    state_by_version: Vec<VersionState>,
    updates_by_version: Vec<Vec<(StateKey, Option<StateValue>)>>,
}

impl StateByVersion {
    pub fn from_updates<'a>(
        updates: impl IntoIterator<
            Item = impl IntoIterator<Item = (&'a StateKey, Option<&'a StateValue>)>,
        >,
    ) -> Self {
        updates
            .into_iter()
            .fold(Self::new_empty(), |mut state, kvs| {
                state.append_version(kvs);
                state
            })
    }

    fn new_empty() -> Self {
        Self {
            state_by_version: vec![],
            updates_by_version: vec![],
        }
    }

    fn append_version<'a>(
        &mut self,
        kvs: impl IntoIterator<Item = (&'a StateKey, Option<&'a StateValue>)>,
    ) {
        let kvs = kvs.into_iter().collect_vec();
        self.state_by_version.push(
            self.state_by_version
                .last()
                .unwrap_or(&VersionState::new_empty())
                .update(self.next_version(), kvs.clone()),
        );
        self.updates_by_version.push(
            kvs.into_iter()
                .map(|(k, v)| (k.clone(), v.cloned()))
                .collect(),
        );
    }

    fn next_version(&self) -> Version {
        self.state_by_version.len() as Version
    }

    fn assert_state(&self, state: &State) {
        let expected_usage = match state.version() {
            Some(version) => self.state_by_version[version as usize].usage,
            None => StateStorageUsage::zero(),
        };
        assert_eq!(state.usage(), expected_usage);
    }

    pub fn assert_ledger_state(&self, ledger_state: &LedgerState) {
        self.assert_state(ledger_state.last_checkpoint());
        self.assert_state(ledger_state.latest());
    }

    fn assert_state_summary(&self, state_summary: &StateSummary) {
        if let Some(version) = state_summary.version() {
            assert_eq!(
                state_summary.root_hash(),
                self.state_by_version[version as usize]
                    .summary
                    .get_root_hash(),
            );
        }
    }

    pub fn assert_ledger_state_summary(&self, ledger_state_summary: &LedgerStateSummary) {
        self.assert_state_summary(ledger_state_summary.last_checkpoint());
        self.assert_state_summary(ledger_state_summary.latest());
    }

    pub fn assert_jmt_updates(&self, last_snapshot: &State, snapshot: &State) {
        let jmt_updates = snapshot
            .make_delta(last_snapshot)
            .shards
            .iter()
            .flat_map(|shard| shard.iter())
            .flat_map(|(key, slot)| slot.maybe_update_jmt(key, last_snapshot.next_version()))
            .collect::<HashMap<_, _>>();

        let expected_jmt_updates =
            (last_snapshot.next_version()..snapshot.next_version()).fold(
                HashMap::new(),
                |mut updates, version| {
                    updates.extend(self.updates_by_version[version as usize].iter().map(
                        |(k, v_opt)| (k.hash(), v_opt.as_ref().map(|v| (v.hash(), k.clone()))),
                    ));
                    updates
                },
            );

        assert_eq!(jmt_updates, expected_jmt_updates, "JMT updates mismatch.");
    }
}

impl DbReader for StateByVersion {
    fn get_state_value_with_version_by_version(
        &self,
        state_key: &StateKey,
        version: Version,
    ) -> DbResult<Option<(Version, StateValue)>> {
        Ok(self.state_by_version[version as usize]
            .state
            .get(state_key)
            .cloned())
    }

    fn get_state_proof_by_version_ext(
        &self,
        key_hash: &HashValue,
        version: Version,
        _root_depth: usize,
    ) -> DbResult<SparseMerkleProofExt> {
        Ok(self.state_by_version[version as usize]
            .summary
            .get_proof(key_hash))
    }
}

fn update_state(
    blocks: Vec<Block>,
    state_by_version: Arc<StateByVersion>,
    empty: LedgerStateWithSummary,
    persisted_state: PersistedState,
    to_summary_update: Sender<(Block, LedgerState)>,
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
            HOT_ITEM_REFRESH_INTERVAL_VERSIONS,
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
    from_state_update: Receiver<(Block, LedgerState)>,
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
    from_db_commit: Receiver<StateWithSummary>,
    persisted_state: PersistedState,
) {
    while let Ok(snapshot) = from_db_commit.recv() {
        persisted_state.set(snapshot);
    }
}

fn test_impl(blocks: Vec<Block>) {
    let state_by_version = Arc::new(StateByVersion::from_updates(
        blocks.iter().flat_map(|block| block.writes_by_version()),
    ));

    let empty = LedgerStateWithSummary::new_empty();
    let current_state = Arc::new(Mutex::new(empty.clone()));

    let persisted_state = PersistedState::new_empty_with_config(
        HOT_STATE_MAX_ITEMS,
        HOT_STATE_MAX_BYTES,
        HOT_STATE_MAX_SINGLE_VALUE_BYTES,
    );
    persisted_state.hack_reset(empty.deref().clone());

    let (to_summary_update, from_state_update) = channel();
    let (to_db_commit, from_summary_update) = channel();
    let (to_buffered_state_commit, from_db_commit) = channel();

    let mut threads = vec![];

    {
        let empty = empty.clone();
        let state_by_version = state_by_version.clone();
        let persisted_state = persisted_state.clone();
        threads.push(spawn(move || {
            update_state(
                blocks,
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
                state_by_version.clone(),
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
            commit_state_buffer(from_db_commit, persisted_state);
        }));
    }

    threads
        .into_iter()
        .for_each(|t| t.join().expect("join() failed."))
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(50))]

    #[test]
    fn test_speculative_state_workflow(
        txns_by_block in arb_key_universe(NUM_KEYS).prop_flat_map(move |keys| vec(arb_block(keys, NUM_KEYS, NUM_KEYS, NUM_KEYS), 1..100))
    ) {
        let blocks = txns_by_block.into_iter().scan(0, |next_version, txns| {
            let block = Block::from_txns(txns, *next_version);
            *next_version += block.len() as Version;
            Some(block)
        }).collect();
        test_impl(blocks);
    }
}
