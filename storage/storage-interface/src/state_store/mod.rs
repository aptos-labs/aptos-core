// Copyright (c) Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

pub mod sharded_state_update_refs;
pub mod sharded_state_updates;
pub mod state;
pub mod state_delta;
pub mod state_summary;
pub mod state_update;
pub mod state_view;

pub const NUM_STATE_SHARDS: usize = 16;
