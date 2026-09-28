// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use crate::snapshot_kind::SnapshotKind;
use aptos_crypto::HashValue;
use aptos_storage_interface::StateKind;
use aptos_types::state_store::{
    hot_state::HotStateValueChunkWithProof, state_value::StateValueChunkWithProof,
};

/// A chunk of a fast-sync snapshot. The stages share the chunk handling and
/// only differ in their leaf type: hot state leaves are `HotStateValue`s.
#[derive(Clone, Debug)]
pub enum SnapshotChunk {
    States(StateKind, StateValueChunkWithProof),
    HotStates(HotStateValueChunkWithProof),
}

impl SnapshotChunk {
    /// The snapshot stage this chunk belongs to
    pub fn kind(&self) -> SnapshotKind {
        match self {
            SnapshotChunk::States(state_kind, _) => SnapshotKind::from(*state_kind),
            SnapshotChunk::HotStates(_) => SnapshotKind::HotState,
        }
    }

    /// The index of the first value in the chunk
    pub fn first_index(&self) -> u64 {
        match self {
            SnapshotChunk::States(_, chunk) => chunk.first_index,
            SnapshotChunk::HotStates(chunk) => chunk.first_index,
        }
    }

    /// The index of the last value in the chunk
    pub fn last_index(&self) -> u64 {
        match self {
            SnapshotChunk::States(_, chunk) => chunk.last_index,
            SnapshotChunk::HotStates(chunk) => chunk.last_index,
        }
    }

    /// The number of values carried by the chunk
    pub fn num_values(&self) -> u64 {
        let num_values = match self {
            SnapshotChunk::States(_, chunk) => chunk.raw_values.len(),
            SnapshotChunk::HotStates(chunk) => chunk.raw_values.len(),
        };
        num_values as u64
    }

    /// The snapshot root the chunk claims to belong to
    pub fn root_hash(&self) -> HashValue {
        match self {
            SnapshotChunk::States(_, chunk) => chunk.root_hash,
            SnapshotChunk::HotStates(chunk) => chunk.root_hash,
        }
    }

    /// True iff the chunk ends the snapshot
    pub fn is_last_chunk(&self) -> bool {
        match self {
            SnapshotChunk::States(_, chunk) => chunk.is_last_chunk(),
            SnapshotChunk::HotStates(chunk) => chunk.is_last_chunk(),
        }
    }
}
