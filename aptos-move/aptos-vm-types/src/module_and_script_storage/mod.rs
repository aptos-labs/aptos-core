// Copyright (c) Aptos Foundation
// SPDX-License-Identifier: Innovation-Enabling Source Code License

pub mod code_storage;
pub mod module_storage;

mod state_view_adapter;
pub use state_view_adapter::{AptosCodeStorageAdapter, AsAptosCodeStorage};
