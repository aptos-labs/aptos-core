// Copyright (c) Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

pub mod state;
pub mod state_delta;
pub mod state_summary;
pub mod state_update_refs;
pub mod state_view;
pub mod state_with_summary;
pub mod versioned_state_value;

pub const NUM_STATE_SHARDS: usize = 16;
