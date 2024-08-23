// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

pub mod module_storage;
pub mod script_storage;

mod state_view_adapter;
pub use state_view_adapter::{AptosCodeStorageAdapter, AsAptosCodeStorage};
