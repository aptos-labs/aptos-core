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

/// Message type for the two-stage async commit pipeline shared by main state
/// (`buffered_state` → `state_snapshot_committer` → `state_merkle_batch_committer`)
/// and the native-position pipeline (`position_buffered_state` →
/// `position_snapshot_committer` → `position_merkle_batch_committer`).
///
/// `Data(T)` carries a snapshot or merkle batch payload. `Sync(Sender<()>)` is
/// a barrier — the receiver signals back when it has processed all preceding
/// `Data` messages. `Exit` shuts the worker down on graceful drop.
pub(crate) enum CommitMessage<T> {
    Data(T),
    Sync(Sender<()>),
    Exit,
}

/// Single-consumer async commit thread + sync channel + sync barrier +
/// Exit-on-drop. Used by both main-state and native-position pipelines
/// to drive their snapshot committer / batch committer threads.
///
/// The constructor takes:
/// - a thread `name`,
/// - the bounded channel `capacity` (use `0` for a rendezvous channel),
/// - a `run` closure that consumes the `Receiver<CommitMessage<T>>`.
///
/// The returned struct holds the `SyncSender` and the `JoinHandle`.
/// `drain_barrier()` pushes a `Sync` message and blocks until the
/// worker acknowledges, draining all preceding `Data` messages.
/// `Drop` sends `Exit` and joins the worker.
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

    /// Send a `Sync` barrier and wait for the worker to drain.
    pub(crate) fn drain_barrier(&self) {
        let (sync_sender, sync_receiver) = mpsc::channel();
        self.sender
            .send(CommitMessage::Sync(sync_sender))
            .expect("commit thread dropped");
        sync_receiver.recv().expect("sync barrier dropped");
    }

    /// Take the join handle without sending Exit — used by holders
    /// that want a different shutdown sequence (e.g. `sync_commit`
    /// before Exit).
    #[allow(dead_code)]
    pub(crate) fn take_join_handle(&mut self) -> Option<JoinHandle<()>> {
        self.join_handle.take()
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

/// Generic snapshot committer — the first stage of the two-stage
/// async commit pipeline. Replaces both `StateSnapshotCommitter` and
/// `PositionSnapshotCommitter`. Holds:
/// - `last_snapshot: S` — the most recent merklized snapshot
///   (advanced by the merklize closure on each `Data`).
/// - `receiver: Receiver<CommitMessage<I>>` — incoming payloads from
///   the buffered state.
/// - `batch_thread: AsyncCommitThread<O>` — channel + thread into the
///   downstream batch committer. `Drop` sends `Exit` + joins.
///
/// `run(merklize)` runs the loop: on each `Data(input)`, calls
/// `merklize(&mut last_snapshot, input) -> O`, forwards the result to
/// `batch_thread`. Pipeline-specific DB handles are captured by the
/// `merklize` closure, not held on the struct.
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

/// Spawn the standard two-stage commit pipeline (snapshot committer
/// + nested batch committer) and return the outer
/// [`AsyncCommitThread`] handle that the buffered state plugs into
/// `BufferedStateCore`.
///
/// All pipelines (main state, position, and future position-shaped
/// pipelines) follow the same structural pattern — outer
/// snapshot-committer thread, inner batch-committer thread,
/// `SnapshotCommitter::run` driving the loop with a pipeline-specific
/// `merklize` closure. The pipeline-specific bits (thread names,
/// channel sizes, batch-committer body, merklize body) are passed in
/// as parameters and closures.
///
/// Type params:
/// - `S` — last-snapshot type the merklize closure mutates.
/// - `I` — input payload type the buffered state sends.
/// - `O` — batch-commit output type passed downstream.
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

/// The bare outer loop, exposed for any future caller that needs the
/// channel-forwarding shape without the `SnapshotCommitter` shell.
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

/// Terminal-stage batch committer loop. Mirrors `run_snapshot_committer_loop`
/// but without forwarding — sink of the pipeline. Used by both
/// `StateMerkleBatchCommitter` and `PositionMerkleBatchCommitter`.
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

/// The pre-built RocksDB raw batches a snapshot committer hands to a
/// batch committer for `ShardedJmtMerkleDb::commit`. Shared between
/// main state's `StateMerkleCommit` (which carries one of these for
/// the cold half and another `Option<>` for the hot half) and
/// position's `PositionMerkleCommit` (which carries exactly one).
pub(crate) struct MerkleBatch {
    pub top_levels_batch: aptos_schemadb::batch::RawBatch,
    pub batches_for_shards: Vec<aptos_schemadb::batch::RawBatch>,
}

/// Translate a JMT `TreeUpdateBatch` into RocksDB writes against `batch`:
/// every node into `JellyfishMerkleNodeSchema`, every stale-node index
/// into either `StaleNodeIndexCrossEpochSchema` (if its node version is
/// `<= previous_epoch_ending_version`) or the regular `StaleNodeIndexSchema`.
///
/// Shared by both merkle batch committers. State writes one of these
/// per shard (+ a separate one for top-level nodes); position writes a
/// single one against its single merkle DB. The cross-epoch split is
/// identical in either case — that's the whole reason this lives here.
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

/// Per-version snapshot of state used by `BufferedStateCore` to gate
/// flushes. Two checkpoints are "the same" iff their `next_version`s
/// agree (versions are monotone in both pipelines).
pub trait CheckpointSnapshot: Clone {
    fn next_version(&self) -> Version;
}

/// Latest + last-checkpoint pair. Both pipelines expose this shape via
/// `aptos_storage_interface::state_store::state_with_summary::LedgerStateWithSummary`
/// (main state) and `PositionLedgerStateWithSummary` (position). The
/// generic `BufferedState` consumes `next_version` (latest's
/// next_version) and `last_checkpoint_snapshot` to decide when a
/// commit should fire.
pub trait LedgerStateView {
    type Snapshot: CheckpointSnapshot;
    fn next_version(&self) -> Version;
    fn last_checkpoint_snapshot(&self) -> Self::Snapshot;
}

/// Generic core of the buffered-state pattern shared between
/// `state_store::BufferedState` and `position_buffered_state::PositionBufferedState`.
///
/// Holds:
/// - the shared `Arc<Mutex<L>>` for outside readers,
/// - `last_snapshot` (most recent state handed to the snapshot committer),
/// - the `AsyncCommitThread<P>` driving the first-stage thread,
/// - the size + version-interval budget gating snapshot flushes.
///
/// `should_commit` decides whether a flush should fire (caller-provided
/// `sync_commit` OR size budget OR version interval). `enqueue` sends
/// the payload and advances `last_snapshot`. `drain_if_sync` does the
/// barrier wait. Caller types stack their own logic (e.g. main state's
/// hot-state-updates merging) on top.
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

    /// True iff `checkpoint` represents a new snapshot AND a flush
    /// trigger is met. Caller's responsibility to actually `enqueue`
    /// if this returns true.
    pub(crate) fn should_commit(&self, checkpoint: &S, sync_commit: bool) -> bool {
        checkpoint.next_version() != self.last_snapshot.next_version()
            && (sync_commit
                || self.estimated_items >= self.target_items
                || self.buffered_versions() >= self.target_snapshot_interval)
    }

    /// Send `payload` to the committer thread and advance bookkeeping.
    /// Resets `estimated_items` and stamps `last_snapshot = checkpoint`.
    pub(crate) fn enqueue(&mut self, checkpoint: S, payload: P) {
        self.commit_thread
            .sender()
            .send(CommitMessage::Data(payload))
            .expect("commit thread dropped");
        self.estimated_items = 0;
        self.last_snapshot = checkpoint;
    }

    /// If `sync_commit`, push a barrier and wait.
    pub(crate) fn drain_if_sync(&self, sync_commit: bool) {
        if sync_commit {
            self.commit_thread.drain_barrier();
        }
    }

    /// Override `last_snapshot` without enqueuing — restore-tooling hook.
    pub(crate) fn force_last_snapshot(&mut self, snapshot: S) {
        self.last_snapshot = snapshot;
    }
}

// --- Trait impls for main state ---

impl CheckpointSnapshot for StateWithSummary {
    fn next_version(&self) -> Version {
        // StateWithSummary derefs to State; State has next_version().
        (**self).next_version()
    }
}

impl LedgerStateView for LedgerStateWithSummary {
    type Snapshot = StateWithSummary;

    fn next_version(&self) -> Version {
        // LedgerStateWithSummary derefs to its `latest: StateWithSummary`,
        // which in turn derefs to State.
        (***self).next_version()
    }

    fn last_checkpoint_snapshot(&self) -> Self::Snapshot {
        // Inherent method `last_checkpoint()` returns `&StateWithSummary`.
        self.last_checkpoint().clone()
    }
}

// --- Trait impls for position ---

impl CheckpointSnapshot for crate::position_buffered_state::PositionStateWithSummary {
    fn next_version(&self) -> Version {
        self.next_version()
    }
}

impl LedgerStateView for crate::position_buffered_state::PositionLedgerStateWithSummary {
    type Snapshot = crate::position_buffered_state::PositionStateWithSummary;

    fn next_version(&self) -> Version {
        self.latest().next_version()
    }

    fn last_checkpoint_snapshot(&self) -> Self::Snapshot {
        self.last_checkpoint().clone()
    }
}

/// Per-pipeline "extras" that hook into the generic [`BufferedState`].
/// State side carries hot-state pre/post-checkpoint accumulation;
/// position side has no extras and ships a unit impl.
///
/// `absorb_chunk` is called once per `update` BEFORE `maybe_commit`
/// fires. It receives `checkpoint_advanced` so impls can fold prior
/// post-checkpoint state into the pending pre-checkpoint accumulator,
/// then merge the chunk's contributions in both directions (its
/// pre-checkpoint share into pending, its post-checkpoint share into
/// the post-checkpoint accumulator). `build_payload` is called from
/// `maybe_commit` to drain the pre-checkpoint accumulator into the
/// snapshot payload `P`.
pub trait BufferedStateExtras<P, S>: Send + 'static {
    type ChunkInput;
    fn absorb_chunk(&mut self, input: Self::ChunkInput, checkpoint_advanced: bool);
    fn build_payload(&mut self, snapshot: S) -> P;
}

/// Generic buffered state that unifies main-state's `BufferedState`
/// and the native-position `PositionBufferedState`. Holds a
/// `BufferedStateCore` (shared gating + channel + thread) and a
/// pipeline-specific `extras` value implementing
/// [`BufferedStateExtras`].
///
/// `update(new_state, chunk_input, items, sync_commit)` is the single
/// entry point. State callers pass a `HotStateUpdates` chunk; position
/// callers pass `()`. The `extras` decides what (if anything) to
/// accumulate from the chunk and how the snapshot payload is built.
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

    /// Apply `new_state` to the buffer. `chunk_input` is the
    /// pipeline-specific chunk payload (e.g. `HotStateUpdates` for main
    /// state, `()` for position). Flushes a snapshot to the committer
    /// thread iff a new checkpoint advanced and a budget threshold is
    /// met (or `sync_commit` forces it).
    pub(crate) fn update(
        &mut self,
        new_state: L,
        chunk_input: X::ChunkInput,
        estimated_new_items: usize,
        sync_commit: bool,
    ) -> aptos_storage_interface::Result<()> {
        let old_next_version = self.core.current_state.lock().next_version();
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

    /// Drain pending state to the committer synchronously.
    pub(crate) fn sync_commit(&mut self) {
        let checkpoint = self.core.current_state.lock().last_checkpoint_snapshot();
        self.maybe_commit(Some(checkpoint), /* sync_commit = */ true);
    }

    /// Restore-tooling hook — override `last_snapshot` without
    /// enqueuing a commit.
    #[allow(dead_code)]
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
}

impl<L, S, P, X> Drop for BufferedState<L, S, P, X>
where
    L: LedgerStateView<Snapshot = S> + Clone,
    S: CheckpointSnapshot,
    P: Send + 'static,
    X: BufferedStateExtras<P, S>,
{
    fn drop(&mut self) {
        // Flush pending state synchronously before the
        // `AsyncCommitThread::drop` inside `BufferedStateCore` sends
        // Exit + joins.
        self.sync_commit();
    }
}

/// Generic owner / coordinator for a pipeline's async commit
/// pipeline. Wraps:
///
/// - the shared `current_state` mutex (handed to outside readers, so
///   they can observe the latest ledger-state-with-summary without
///   taking the heavier buffered-state mutex)
/// - the [`BufferedState`] instance under its own mutex (so concurrent
///   commit-path writers serialize on it)
///
/// Position-shaped pipelines (position today; future order /
/// collateral) instantiate this with their per-pipeline
/// `LedgerStateWithSummary` type alias and their `BufferedState`
/// instantiation. The shared infrastructure lives here; construction
/// of the underlying `BufferedState` (per-pipeline merkle DB, commit
/// closures) is pipeline-specific and lives on the alias's inherent
/// impl.
pub struct PipelineStateStore<L, BS> {
    /// Shared with outside readers. The same `Arc` is held inside
    /// the buffered state so writes through `update()` are visible
    /// here immediately.
    current_state: Arc<Mutex<L>>,
    /// The buffered state itself. Wrapped in a `Mutex` so concurrent
    /// commit-path callers serialize. Mirrors main state's
    /// `StateStore::buffered_state: Mutex<BufferedState>`.
    buffered_state: Mutex<BS>,
}

impl<L, BS> PipelineStateStore<L, BS> {
    /// Construct from a pre-built `current_state` Arc + buffered
    /// state. Each pipeline's inherent-impl `new_at_snapshot` wires
    /// these together with its own merkle DB / ledger DB.
    pub fn from_parts(current_state: Arc<Mutex<L>>, buffered_state: BS) -> Self {
        Self {
            current_state,
            buffered_state: Mutex::new(buffered_state),
        }
    }

    /// Hand back the shared `current_state` so outside readers can
    /// take their own `Arc` clones. Cheap (single atomic).
    pub fn current_state(&self) -> Arc<Mutex<L>> {
        Arc::clone(&self.current_state)
    }

    /// Lock the buffered state for the commit path. Returns the
    /// `MutexGuard` directly so callers can call `update` /
    /// `sync_commit` on it.
    pub(crate) fn buffered_state_locked(&self) -> aptos_infallible::MutexGuard<'_, BS> {
        self.buffered_state.lock()
    }
}
