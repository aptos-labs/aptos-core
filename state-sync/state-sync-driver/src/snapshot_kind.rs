// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use aptos_storage_interface::StateKind;

/// A snapshot stage of the fast sync. The flat variants let the driver match
/// directly on each stage for scheduling, progress tracking, and receiver
/// routing.
///
/// Note: this is deliberately owned by the driver rather than shared with
/// storage or the streaming service. At those boundaries, main and position map
/// to their [`StateKind`], while hot state uses its own APIs because its leaves
/// are `HotStateValue`s.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum SnapshotKind {
    MainState,
    HotState,
    Position,
}

impl SnapshotKind {
    /// A short label used in log and error messages.
    pub fn get_label(&self) -> &'static str {
        match self {
            SnapshotKind::MainState => "state",
            SnapshotKind::HotState => "hot state",
            SnapshotKind::Position => "position state",
        }
    }
}

impl From<StateKind> for SnapshotKind {
    fn from(kind: StateKind) -> Self {
        match kind {
            StateKind::MainState => SnapshotKind::MainState,
            StateKind::Position => SnapshotKind::Position,
        }
    }
}
