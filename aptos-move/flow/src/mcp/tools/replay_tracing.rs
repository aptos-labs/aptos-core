// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Trace recorder for `move_replay_transaction`.
//!
//! [`TraceRecorder`] is a small `Arc<Mutex<_>>`-backed buffer of typed
//! [`TraceEntry`] values. [`TracingDebugger`] wraps a [`MoveDebugger`] and
//! records one entry per `state_view_at_version[_with_overrides]` call; the
//! [`DynStateView`] it hands to the VM is in turn wrapped by
//! `TracingStateView`, which optionally records one entry per storage read.
//! When replay finishes, the orchestrator calls
//! [`TraceRecorder::into_capture`] to take ownership of the buffer.
//!
//! We deliberately do *not* go through the `tracing` ecosystem: the Move VM
//! emits no `tracing` events today, so the only callers are the handful of
//! callsites in this file. A direct recorder is one allocation per entry,
//! thread-safe across any future VM-internal parallelism (`set_default` is
//! per-thread), and uses statically-typed payloads instead of stringly-typed
//! `Debug`-formatted fields.
//!
//! The wrapper only records on `state_view_at_version[_with_overrides]`.
//! `execute_transaction_*` and `get_committed_transaction_at_version` are
//! pure delegations because the replay flow never reaches those entry
//! points — the simulation drivers in `aptos-move-cli::local_simulation`
//! call only the state-view methods on the debugger.

use aptos_gas_profiling::TransactionGasLog;
use aptos_move_cli::{DynStateView, MoveDebugger};
use aptos_types::{
    state_store::{
        errors::StateViewError, state_key::StateKey, state_slot::StateSlot,
        state_storage_usage::StateStorageUsage, state_value::StateValue, StateViewId, TStateView,
    },
    transaction::{
        AuxiliaryInfo, PersistedAuxiliaryInfo, SignedTransaction, Transaction, TransactionInfo,
        TransactionOutput, Version,
    },
};
use aptos_validator_interface::LocalModuleOverrides;
use aptos_vm_types::output::VMOutput;
use async_trait::async_trait;
use move_core_types::vm_status::VMStatus;
use rmcp::schemars;
use std::sync::{Arc, Mutex};

// ── Public types surfaced in the MCP tool schema ──

/// Knobs for a single replay's trace capture.
#[derive(Debug, Clone, Copy)]
pub struct CaptureOpts {
    /// Stop recording after this many entries; further entries bump the
    /// `truncated` counter on the resulting [`TraceCapture`].
    pub max_events: usize,
    /// When `true`, record one [`TraceEntry::StorageRead`] per storage hit
    /// on a wrapped state view. Off by default: replays typically issue
    /// hundreds of storage reads that drown the signal of the higher-level
    /// state-view event.
    pub record_storage_reads: bool,
    /// When `true`, [`TraceEntry::StorageRead::key`] is omitted instead of
    /// carrying the `Debug`-formatted `StateKey`. Only consulted when
    /// `record_storage_reads` is also `true`.
    pub redact_storage_keys: bool,
}

/// One captured event from a wrapped debugger / state view.
#[derive(Debug, Clone, serde::Serialize, schemars::JsonSchema)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum TraceEntry {
    /// The VM asked the debugger for a state view, with or without local
    /// module overrides. Fires once per replay.
    StateView {
        version: u64,
        /// `true` when the simulator took the local-overrides path.
        with_overrides: bool,
    },
    /// One `TStateView::get_state_slot` / `get_state_value` call against
    /// the wrapped state view. Only emitted when `record_storage_reads` is
    /// set on the originating [`CaptureOpts`].
    StorageRead {
        version: u64,
        /// Which `TStateView` method was invoked (`"get_state_slot"` or
        /// `"get_state_value"`).
        op: &'static str,
        /// `Debug`-formatted `StateKey`. `None` iff `redact_storage_keys`
        /// was set on the originating [`CaptureOpts`].
        #[serde(skip_serializing_if = "Option::is_none")]
        key: Option<String>,
    },
}

/// Result of draining a [`TraceRecorder`]. `truncated` counts how many
/// entries were dropped because `max_events` had been reached.
#[derive(Debug, Default, Clone, serde::Serialize, schemars::JsonSchema)]
pub struct TraceCapture {
    pub entries: Vec<TraceEntry>,
    pub truncated: usize,
}

// ── Recorder ──

struct State {
    max_events: usize,
    entries: Vec<TraceEntry>,
    truncated: usize,
}

/// Shared, thread-safe buffer for [`TraceEntry`] values. Construct with
/// [`TraceRecorder::new`], hand a clone of the `Arc` to [`TracingDebugger`],
/// then call [`TraceRecorder::into_capture`] once the wrapper has been
/// dropped to take ownership of the buffer.
pub struct TraceRecorder {
    state: Mutex<State>,
    record_storage_reads: bool,
    redact_storage_keys: bool,
}

impl TraceRecorder {
    pub fn new(opts: CaptureOpts) -> Arc<Self> {
        Arc::new(Self {
            state: Mutex::new(State {
                max_events: opts.max_events,
                entries: Vec::new(),
                truncated: 0,
            }),
            record_storage_reads: opts.record_storage_reads,
            redact_storage_keys: opts.redact_storage_keys,
        })
    }

    fn record(&self, entry: TraceEntry) {
        let mut state = self.state.lock().expect("trace recorder mutex poisoned");
        if state.entries.len() >= state.max_events {
            state.truncated += 1;
        } else {
            state.entries.push(entry);
        }
    }

    fn render_key(&self, key: &StateKey) -> Option<String> {
        (!self.redact_storage_keys).then(|| format!("{:?}", key))
    }

    /// Take ownership of the recorder's buffer. The caller is expected to
    /// have dropped every clone of the `Arc` (e.g. the [`TracingDebugger`]
    /// wrapper) before calling this, so the `try_unwrap` always succeeds.
    pub fn into_capture(self: Arc<Self>) -> TraceCapture {
        let recorder = Arc::try_unwrap(self).unwrap_or_else(|_| {
            panic!(
                "TraceRecorder::into_capture called while wrappers still hold the Arc; \
                 drop the TracingDebugger first",
            )
        });
        let state = recorder
            .state
            .into_inner()
            .expect("trace recorder mutex poisoned");
        TraceCapture {
            entries: state.entries,
            truncated: state.truncated,
        }
    }
}

// ── TracingDebugger: wraps a MoveDebugger to record state-view requests ──

/// Wraps a [`MoveDebugger`] so the state views it returns to the VM emit a
/// [`TraceEntry::StateView`] event when handed out and an optional
/// [`TraceEntry::StorageRead`] per read against them. All other debugger
/// methods are forwarded unchanged — the replay flow does not call them on
/// the wrapper, so recording them would be dead code.
pub struct TracingDebugger {
    inner: Arc<dyn MoveDebugger>,
    recorder: Arc<TraceRecorder>,
}

impl TracingDebugger {
    pub fn new(inner: Arc<dyn MoveDebugger>, recorder: Arc<TraceRecorder>) -> Self {
        Self { inner, recorder }
    }

    fn wrap_view(&self, version: u64, view: DynStateView) -> DynStateView {
        DynStateView::new(Box::new(TracingStateView {
            inner: view,
            version,
            recorder: Arc::clone(&self.recorder),
        }))
    }
}

#[async_trait]
impl MoveDebugger for TracingDebugger {
    fn state_view_at_version(&self, version: u64) -> DynStateView {
        self.recorder.record(TraceEntry::StateView {
            version,
            with_overrides: false,
        });
        self.wrap_view(version, self.inner.state_view_at_version(version))
    }

    fn state_view_at_version_with_overrides(
        &self,
        version: u64,
        overrides: Arc<LocalModuleOverrides>,
    ) -> DynStateView {
        self.recorder.record(TraceEntry::StateView {
            version,
            with_overrides: true,
        });
        let view = self
            .inner
            .state_view_at_version_with_overrides(version, overrides);
        self.wrap_view(version, view)
    }

    fn execute_transaction_at_version_with_gas_profiler(
        &self,
        version: u64,
        txn: SignedTransaction,
        auxiliary_info: AuxiliaryInfo,
    ) -> anyhow::Result<(VMStatus, VMOutput, TransactionGasLog)> {
        self.inner
            .execute_transaction_at_version_with_gas_profiler(version, txn, auxiliary_info)
    }

    fn execute_transaction_at_version(
        &self,
        version: u64,
        transaction: Transaction,
        auxiliary_info: PersistedAuxiliaryInfo,
    ) -> anyhow::Result<TransactionOutput> {
        self.inner
            .execute_transaction_at_version(version, transaction, auxiliary_info)
    }

    async fn get_committed_transaction_at_version(
        &self,
        version: u64,
    ) -> anyhow::Result<(Transaction, TransactionInfo, PersistedAuxiliaryInfo)> {
        self.inner
            .get_committed_transaction_at_version(version)
            .await
    }
}

// ── TracingStateView: records one event per storage read ──

struct TracingStateView {
    inner: DynStateView,
    version: u64,
    recorder: Arc<TraceRecorder>,
}

impl TracingStateView {
    fn record_read(&self, op: &'static str, key: &StateKey) {
        if !self.recorder.record_storage_reads {
            return;
        }
        self.recorder.record(TraceEntry::StorageRead {
            version: self.version,
            op,
            key: self.recorder.render_key(key),
        });
    }
}

impl TStateView for TracingStateView {
    type Key = StateKey;

    fn id(&self) -> StateViewId {
        self.inner.id()
    }

    fn get_usage(&self) -> Result<StateStorageUsage, StateViewError> {
        self.inner.get_usage()
    }

    fn next_version(&self) -> Version {
        self.inner.next_version()
    }

    fn get_state_slot(&self, state_key: &StateKey) -> Result<StateSlot, StateViewError> {
        self.record_read("get_state_slot", state_key);
        self.inner.get_state_slot(state_key)
    }

    fn get_state_value(&self, state_key: &StateKey) -> Result<Option<StateValue>, StateViewError> {
        self.record_read("get_state_value", state_key);
        self.inner.get_state_value(state_key)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use aptos_types::state_store::state_slot::StateSlotKind;

    fn opts(
        max_events: usize,
        record_storage_reads: bool,
        redact_storage_keys: bool,
    ) -> CaptureOpts {
        CaptureOpts {
            max_events,
            record_storage_reads,
            redact_storage_keys,
        }
    }

    /// Empty `TStateView` used to drive the storage-read wrapper without a
    /// real backing store. Returns trivial defaults rather than panicking
    /// so it's safe to wire up in future tests.
    struct EmptyView;

    impl TStateView for EmptyView {
        type Key = StateKey;

        fn id(&self) -> StateViewId {
            StateViewId::Miscellaneous
        }

        fn get_usage(&self) -> Result<StateStorageUsage, StateViewError> {
            Ok(StateStorageUsage::zero())
        }

        fn next_version(&self) -> Version {
            0
        }

        fn get_state_slot(&self, _: &StateKey) -> Result<StateSlot, StateViewError> {
            Ok(StateSlot::new_without_state_key(StateSlotKind::ColdVacant))
        }

        fn get_state_value(&self, _: &StateKey) -> Result<Option<StateValue>, StateViewError> {
            Ok(None)
        }
    }

    fn empty_state_view(version: u64, recorder: &Arc<TraceRecorder>) -> TracingStateView {
        TracingStateView {
            inner: DynStateView::new(Box::new(EmptyView)),
            version,
            recorder: Arc::clone(recorder),
        }
    }

    #[test]
    fn records_typed_entries() {
        let recorder = TraceRecorder::new(opts(16, true, true));
        recorder.record(TraceEntry::StateView {
            version: 7,
            with_overrides: false,
        });
        recorder.record(TraceEntry::StorageRead {
            version: 7,
            op: "get_state_slot",
            key: None,
        });
        let capture = recorder.into_capture();
        assert_eq!(capture.entries.len(), 2);
        assert_eq!(capture.truncated, 0);
        assert!(matches!(capture.entries[0], TraceEntry::StateView {
            version: 7,
            with_overrides: false
        }));
        assert!(matches!(capture.entries[1], TraceEntry::StorageRead {
            version: 7,
            op: "get_state_slot",
            ..
        }));
    }

    #[test]
    fn truncates_when_max_events_reached() {
        let recorder = TraceRecorder::new(opts(2, true, true));
        for version in 0..5 {
            recorder.record(TraceEntry::StateView {
                version,
                with_overrides: false,
            });
        }
        let capture = recorder.into_capture();
        assert_eq!(capture.entries.len(), 2);
        assert_eq!(capture.truncated, 3);
    }

    #[test]
    fn redact_omits_key() {
        let recorder = TraceRecorder::new(opts(4, true, true));
        let rendered = recorder.render_key(&StateKey::raw(&[0xDE, 0xAD, 0xBE, 0xEF]));
        assert!(rendered.is_none(), "redact should yield None");
    }

    #[test]
    fn non_redact_renders_debug_key() {
        let recorder = TraceRecorder::new(opts(4, true, false));
        let key = StateKey::raw(&[0x01, 0x02]);
        let rendered = recorder.render_key(&key).expect("should render");
        assert_eq!(rendered, format!("{:?}", key));
    }

    #[test]
    fn empty_recorder_yields_empty_capture() {
        let recorder = TraceRecorder::new(opts(4, true, true));
        let capture = recorder.into_capture();
        assert!(capture.entries.is_empty());
        assert_eq!(capture.truncated, 0);
    }

    #[test]
    fn storage_reads_dropped_when_disabled() {
        let recorder = TraceRecorder::new(opts(16, false, true));
        let view = empty_state_view(1, &recorder);
        for _ in 0..5 {
            view.record_read("get_state_value", &StateKey::raw(&[0x01]));
        }
        recorder.record(TraceEntry::StateView {
            version: 1,
            with_overrides: false,
        });
        drop(view);
        let capture = recorder.into_capture();
        assert_eq!(capture.entries.len(), 1, "only the state_view should land");
        assert_eq!(
            capture.truncated, 0,
            "skipped reads should not count as truncated"
        );
    }

    #[test]
    fn storage_reads_recorded_when_enabled() {
        let recorder = TraceRecorder::new(opts(16, true, true));
        let view = empty_state_view(42, &recorder);
        view.record_read("get_state_slot", &StateKey::raw(&[0x01]));
        view.record_read("get_state_value", &StateKey::raw(&[0x02]));
        drop(view);
        let capture = recorder.into_capture();
        assert_eq!(capture.entries.len(), 2);
        for (entry, expected_op) in capture
            .entries
            .iter()
            .zip(["get_state_slot", "get_state_value"])
        {
            match entry {
                TraceEntry::StorageRead { version, op, key } => {
                    assert_eq!(*version, 42);
                    assert_eq!(*op, expected_op);
                    assert!(key.is_none(), "redact should omit the key");
                },
                TraceEntry::StateView { .. } => panic!("expected StorageRead"),
            }
        }
    }

    #[test]
    #[should_panic(
        expected = "TraceRecorder::into_capture called while wrappers still hold the Arc"
    )]
    fn into_capture_panics_when_arc_still_shared() {
        let recorder = TraceRecorder::new(opts(4, true, true));
        let _alias = Arc::clone(&recorder);
        let _ = recorder.into_capture();
    }
}
