// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use crate::schema::{
    jellyfish_merkle_node::JellyfishMerkleNodeSchema, stale_node_index::StaleNodeIndexSchema,
    stale_node_index_cross_epoch::StaleNodeIndexCrossEpochSchema,
};
use aptos_infallible::Mutex;
use aptos_jellyfish_merkle::TreeUpdateBatch;
use aptos_schemadb::batch::WriteBatch;
use aptos_storage_interface::{
    state_store::state_with_summary::{LedgerStateWithSummary, StateWithSummary},
    Result,
};
use aptos_types::{state_store::state_key::StateKey, transaction::Version};
use std::{
    sync::{
        mpsc::{self, Receiver, Sender, SyncSender},
        Arc,
    },
    thread::JoinHandle,
};

pub const LEDGER_DB_NAME: &str = "ledger_db";
pub const STATE_MERKLE_DB_NAME: &str = "state_merkle_db";

// TODO: Either implement an iteration API to allow a very old client to loop through a long history
// or guarantee that there is always a recent enough waypoint and client knows to boot from there.
pub(crate) const MAX_NUM_EPOCH_ENDING_LEDGER_INFO: usize = 100;

/// `Sync(Sender<()>)` is a drain barrier — the receiver signals back
/// when it has processed all preceding `Data` messages.
pub(crate) enum CommitMessage<T> {
    Data(T),
    Sync(Sender<()>),
    Exit,
}

pub(crate) struct AsyncCommitThread<T: Send + 'static> {
    sender: SyncSender<CommitMessage<T>>,
    join_handle: Option<JoinHandle<()>>,
}

impl<T: Send + 'static> AsyncCommitThread<T> {
    pub(crate) fn spawn<F>(name: &'static str, capacity: usize, run: F) -> Self
    where
        F: FnOnce(Receiver<CommitMessage<T>>) + Send + 'static,
    {
        let (sender, receiver) = mpsc::sync_channel(capacity);
        let join_handle = std::thread::Builder::new()
            .name(name.to_string())
            .spawn(move || run(receiver))
            .unwrap_or_else(|e| panic!("Failed to spawn {name} thread: {e}"));
        Self {
            sender,
            join_handle: Some(join_handle),
        }
    }

    pub(crate) fn sender(&self) -> &SyncSender<CommitMessage<T>> {
        &self.sender
    }

    pub(crate) fn drain_barrier(&self) {
        let (sync_sender, sync_receiver) = mpsc::channel();
        self.sender
            .send(CommitMessage::Sync(sync_sender))
            .expect("commit thread dropped");
        sync_receiver.recv().expect("sync barrier dropped");
    }

    /// `false` once `quit()` has joined the thread; sends are unsafe.
    pub(crate) fn is_alive(&self) -> bool {
        self.join_handle.is_some()
    }
}

impl<T: Send + 'static> Drop for AsyncCommitThread<T> {
    fn drop(&mut self) {
        if let Some(handle) = self.join_handle.take() {
            let _ = self.sender.send(CommitMessage::Exit);
            let _ = handle.join();
        }
    }
}

pub(crate) struct SnapshotCommitter<S, I, O>
where
    S: CheckpointSnapshot,
    I: Send + 'static,
    O: Send + 'static,
{
    pub(crate) last_snapshot: S,
    pub(crate) receiver: Receiver<CommitMessage<I>>,
    pub(crate) batch_thread: AsyncCommitThread<O>,
}

impl<S, I, O> SnapshotCommitter<S, I, O>
where
    S: CheckpointSnapshot,
    I: Send + 'static,
    O: Send + 'static,
{
    pub(crate) fn new(
        last_snapshot: S,
        receiver: Receiver<CommitMessage<I>>,
        batch_thread: AsyncCommitThread<O>,
    ) -> Self {
        Self {
            last_snapshot,
            receiver,
            batch_thread,
        }
    }

    pub(crate) fn run<F>(self, mut merklize: F)
    where
        F: FnMut(&mut S, I) -> O,
    {
        let Self {
            mut last_snapshot,
            receiver,
            batch_thread,
        } = self;
        let batch_sender = batch_thread.sender().clone();
        run_snapshot_committer_loop(receiver, batch_sender, |input| {
            merklize(&mut last_snapshot, input)
        });
        // `batch_thread` drops here → Exit + join cascades downstream.
    }
}

pub(crate) fn spawn_commit_pipeline<S, I, O, RunBatch, Merklize>(
    snapshot_thread_name: &'static str,
    snapshot_channel_capacity: usize,
    batch_thread_name: &'static str,
    batch_channel_capacity: usize,
    initial_last_snapshot: S,
    run_batch_committer: RunBatch,
    mut merklize: Merklize,
) -> AsyncCommitThread<I>
where
    S: CheckpointSnapshot + Send + 'static,
    I: Send + 'static,
    O: Send + 'static,
    RunBatch: FnOnce(Receiver<CommitMessage<O>>) + Send + 'static,
    Merklize: FnMut(&mut S, I) -> O + Send + 'static,
{
    AsyncCommitThread::spawn(
        snapshot_thread_name,
        snapshot_channel_capacity,
        move |receiver| {
            let batch_thread = AsyncCommitThread::spawn(
                batch_thread_name,
                batch_channel_capacity,
                run_batch_committer,
            );
            let committer: SnapshotCommitter<S, I, O> =
                SnapshotCommitter::new(initial_last_snapshot, receiver, batch_thread);
            committer.run(move |last, input| merklize(last, input));
        },
    )
}

pub(crate) fn run_snapshot_committer_loop<I, O>(
    receiver: Receiver<CommitMessage<I>>,
    batch_sender: SyncSender<CommitMessage<O>>,
    mut merklize: impl FnMut(I) -> O,
) {
    while let Ok(msg) = receiver.recv() {
        match msg {
            CommitMessage::Data(input) => {
                let output = merklize(input);
                batch_sender
                    .send(CommitMessage::Data(output))
                    .expect("downstream batch committer dropped");
            },
            CommitMessage::Sync(finish_sender) => {
                batch_sender
                    .send(CommitMessage::Sync(finish_sender))
                    .expect("downstream batch committer dropped");
            },
            CommitMessage::Exit => break,
        }
    }
}

pub(crate) fn run_batch_committer_loop<T>(
    receiver: Receiver<CommitMessage<T>>,
    mut handle: impl FnMut(T),
) {
    while let Ok(msg) = receiver.recv() {
        match msg {
            CommitMessage::Data(payload) => handle(payload),
            CommitMessage::Sync(finish_sender) => {
                finish_sender.send(()).expect("sync barrier dropped")
            },
            CommitMessage::Exit => break,
        }
    }
}

pub(crate) struct MerkleBatch {
    pub top_levels_batch: aptos_schemadb::batch::RawBatch,
    pub batches_for_shards: Vec<aptos_schemadb::batch::RawBatch>,
}

/// Routes each `stale_node_index_batch` entry into the cross-epoch
/// schema when its node version is `<= previous_epoch_ending_version`,
/// otherwise the regular stale-index schema.
pub(crate) fn populate_jmt_writes<W: WriteBatch>(
    batch: &mut W,
    tree_update_batch: &TreeUpdateBatch<StateKey>,
    previous_epoch_ending_version: Option<Version>,
) -> Result<()> {
    for (node_key, node) in tree_update_batch.node_batch.iter().flatten() {
        batch.put::<JellyfishMerkleNodeSchema>(node_key, node)?;
    }
    for row in tree_update_batch.stale_node_index_batch.iter().flatten() {
        if previous_epoch_ending_version.is_some_and(|prev| row.node_key.version() <= prev) {
            batch.put::<StaleNodeIndexCrossEpochSchema>(row, &())?;
        } else {
            batch.put::<StaleNodeIndexSchema>(row, &())?;
        }
    }
    Ok(())
}

pub trait CheckpointSnapshot: Clone {
    fn next_version(&self) -> Version;
}

pub trait LedgerStateView {
    type Snapshot: CheckpointSnapshot;
    fn next_version(&self) -> Version;
    fn last_checkpoint_snapshot(&self) -> Self::Snapshot;
    fn is_descendant_of(&self, other: &Self) -> bool;
}

pub(crate) struct BufferedStateCore<L, S, P>
where
    L: LedgerStateView<Snapshot = S>,
    S: CheckpointSnapshot,
    P: Send + 'static,
{
    pub(crate) current_state: Arc<Mutex<L>>,
    pub(crate) last_snapshot: S,
    pub(crate) commit_thread: AsyncCommitThread<P>,
    pub(crate) estimated_items: usize,
    pub(crate) target_items: usize,
    pub(crate) target_snapshot_interval: u64,
}

impl<L, S, P> BufferedStateCore<L, S, P>
where
    L: LedgerStateView<Snapshot = S>,
    S: CheckpointSnapshot,
    P: Send + 'static,
{
    pub(crate) fn new(
        current_state: Arc<Mutex<L>>,
        last_snapshot: S,
        commit_thread: AsyncCommitThread<P>,
        target_items: usize,
        target_snapshot_interval: u64,
    ) -> Self {
        Self {
            current_state,
            last_snapshot,
            commit_thread,
            estimated_items: 0,
            target_items,
            target_snapshot_interval,
        }
    }

    pub(crate) fn add_estimated_items(&mut self, n: usize) {
        self.estimated_items += n;
    }

    pub(crate) fn buffered_versions(&self) -> u64 {
        let latest_next = self.current_state.lock().next_version();
        latest_next.saturating_sub(self.last_snapshot.next_version())
    }

    pub(crate) fn should_commit(&self, checkpoint: &S, sync_commit: bool) -> bool {
        checkpoint.next_version() != self.last_snapshot.next_version()
            && (sync_commit
                || self.estimated_items >= self.target_items
                || self.buffered_versions() >= self.target_snapshot_interval)
    }

    pub(crate) fn enqueue(&mut self, checkpoint: S, payload: P) {
        self.commit_thread
            .sender()
            .send(CommitMessage::Data(payload))
            .expect("commit thread dropped");
        self.estimated_items = 0;
        self.last_snapshot = checkpoint;
    }

    pub(crate) fn drain_if_sync(&self, sync_commit: bool) {
        if sync_commit {
            self.commit_thread.drain_barrier();
        }
    }

    pub(crate) fn force_last_snapshot(&mut self, snapshot: S) {
        self.last_snapshot = snapshot;
    }

    /// Drain + shut down the commit pipeline against the current
    /// `current_state` family. Must run *before* any caller repoints
    /// `current_state` at a new MapLayer family — otherwise the
    /// committer thread's drop-time `sync_commit` would compute deltas
    /// across families and panic on `is_descendant_of`.
    pub(crate) fn quit(&mut self) {
        if !self.commit_thread.is_alive() {
            return;
        }
        self.commit_thread.drain_barrier();
        if let Some(handle) = self.commit_thread.join_handle.take() {
            let _ = self.commit_thread.sender.send(CommitMessage::Exit);
            let _ = handle.join();
        }
    }
}

impl CheckpointSnapshot for StateWithSummary {
    fn next_version(&self) -> Version {
        (**self).next_version()
    }
}

impl LedgerStateView for LedgerStateWithSummary {
    type Snapshot = StateWithSummary;

    fn next_version(&self) -> Version {
        (***self).next_version()
    }

    fn last_checkpoint_snapshot(&self) -> Self::Snapshot {
        self.last_checkpoint().clone()
    }

    fn is_descendant_of(&self, other: &Self) -> bool {
        LedgerStateWithSummary::is_descendant_of(self, other)
    }
}

pub trait BufferedStateExtras<P, S>: Send + 'static {
    type ChunkInput;
    fn absorb_chunk(&mut self, input: Self::ChunkInput, checkpoint_advanced: bool);
    fn build_payload(&mut self, snapshot: S) -> P;
}

pub struct BufferedState<L, S, P, X>
where
    L: LedgerStateView<Snapshot = S> + Clone,
    S: CheckpointSnapshot,
    P: Send + 'static,
    X: BufferedStateExtras<P, S>,
{
    pub(crate) core: BufferedStateCore<L, S, P>,
    pub(crate) extras: X,
}

impl<L, S, P, X> BufferedState<L, S, P, X>
where
    L: LedgerStateView<Snapshot = S> + Clone,
    S: CheckpointSnapshot,
    P: Send + 'static,
    X: BufferedStateExtras<P, S>,
{
    pub(crate) fn new(core: BufferedStateCore<L, S, P>, extras: X) -> Self {
        Self { core, extras }
    }

    pub(crate) fn update(
        &mut self,
        new_state: L,
        chunk_input: X::ChunkInput,
        estimated_new_items: usize,
        sync_commit: bool,
    ) -> aptos_storage_interface::Result<()> {
        let old_next_version = {
            let current_state = self.core.current_state.lock();
            assert!(
                new_state.is_descendant_of(&current_state),
                "BufferedState::update: new_state must descend from current_state"
            );
            current_state.next_version()
        };
        self.core.add_estimated_items(estimated_new_items);

        let new_last_checkpoint = new_state.last_checkpoint_snapshot();
        let new_checkpoint_next = new_last_checkpoint.next_version();
        let checkpoint_advanced = old_next_version < new_checkpoint_next;
        let checkpoint_to_commit_opt = checkpoint_advanced.then(|| new_last_checkpoint.clone());

        *self.core.current_state.lock() = new_state;

        self.extras.absorb_chunk(chunk_input, checkpoint_advanced);
        self.maybe_commit(checkpoint_to_commit_opt, sync_commit);

        Ok(())
    }

    pub(crate) fn sync_commit(&mut self) {
        let checkpoint = self.core.current_state.lock().last_checkpoint_snapshot();
        self.maybe_commit(Some(checkpoint), /* sync_commit = */ true);
    }

    pub(crate) fn force_last_snapshot(&mut self, snapshot: S) {
        self.core.force_last_snapshot(snapshot);
    }

    fn maybe_commit(&mut self, checkpoint: Option<S>, sync_commit: bool) {
        if let Some(cp) = checkpoint {
            if self.core.should_commit(&cp, sync_commit) {
                let payload = self.extras.build_payload(cp.clone());
                self.core.enqueue(cp, payload);
            }
        }
        self.core.drain_if_sync(sync_commit);
    }

    /// Drain + shut down the commit pipeline. After this the
    /// `BufferedState` is dead; only safe next op is `Drop` (which
    /// short-circuits because the pipeline is already joined).
    pub(crate) fn quit(&mut self) {
        self.sync_commit();
        self.core.quit();
    }
}

impl<L, S, P, X> Drop for BufferedState<L, S, P, X>
where
    L: LedgerStateView<Snapshot = S> + Clone,
    S: CheckpointSnapshot,
    P: Send + 'static,
    X: BufferedStateExtras<P, S>,
{
    fn drop(&mut self) {
        // Skip if `quit()` already shut the pipeline down — sending
        // on the closed channel would panic.
        if self.core.commit_thread.is_alive() {
            self.sync_commit();
        }
    }
}
