// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Crate-level re-export of the per-account position index types
//! (now defined in `aptos-storage-interface` so the executor can name
//! them in its function signatures).

#![forbid(unsafe_code)]

pub use aptos_storage_interface::state_store::user_positions::{
    decode_rows_to_user_position_states, materialize_user_position_updates, PositionWrite,
    UserPositionKey, UserPositionState, UserPositions,
};
