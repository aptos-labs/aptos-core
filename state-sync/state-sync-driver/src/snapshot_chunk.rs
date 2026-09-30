// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use aptos_crypto::HashValue;
use aptos_data_streaming_service::streaming_client::SnapshotKind;
use aptos_storage_interface::StateKind;
use aptos_types::state_store::{
    hot_state::HotStateValueChunkWithProof, state_value::StateValueChunkWithProof,
};

/// A chunk of a fast-sync snapshot.
#[derive(Clone, Debug)]
pub enum SnapshotChunk {
    States(StateKind, StateValueChunkWithProof),
    HotStates(HotStateValueChunkWithProof),
}

/// Evaluates `$body` with `$chunk` bound to the inner chunk, whatever its leaf
/// type. Both chunk types share the fields the snapshot stages need.
macro_rules! with_chunk {
    ($snapshot_chunk:expr, $chunk:ident => $body:expr) => {
        match $snapshot_chunk {
            SnapshotChunk::States(_, $chunk) => $body,
            SnapshotChunk::HotStates($chunk) => $body,
        }
    };
}

impl SnapshotChunk {
    pub fn kind(&self) -> SnapshotKind {
        match self {
            Self::States(state_kind, _) => SnapshotKind::State(*state_kind),
            Self::HotStates(_) => SnapshotKind::HotState,
        }
    }

    pub fn first_index(&self) -> u64 {
        with_chunk!(self, chunk => chunk.first_index)
    }

    pub fn last_index(&self) -> u64 {
        with_chunk!(self, chunk => chunk.last_index)
    }

    pub fn num_values(&self) -> usize {
        with_chunk!(self, chunk => chunk.raw_values.len())
    }

    pub fn root_hash(&self) -> HashValue {
        with_chunk!(self, chunk => chunk.root_hash)
    }

    pub fn is_last_chunk(&self) -> bool {
        with_chunk!(self, chunk => chunk.is_last_chunk())
    }
}
