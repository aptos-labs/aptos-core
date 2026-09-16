// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Crate-level re-export of the position index types (defined in
//! `aptos-storage-interface` so the executor can name them in its
//! function signatures).

#![forbid(unsafe_code)]

pub use aptos_storage_interface::state_store::positions::{
    decode_position_writes, decode_rows_to_positions, position_key_of, shard_of, AccountKey,
    PositionBase, PositionBaseView, PositionKey, PositionOverlay, PositionWrites,
    NUM_POSITION_SHARDS,
};
