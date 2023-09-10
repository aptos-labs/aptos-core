// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

mod respawned_view_adapter;
mod state_view_adapter;

pub use crate::storage_adapter::{
    respawned_view_adapter::ExecutorViewWithChanges,
    state_view_adapter::{AsAdapter, ExecutorViewAdapter},
};
