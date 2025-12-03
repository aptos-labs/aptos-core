// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

pub mod code_storage;
pub mod module_storage;

mod state_view_adapter;
pub use state_view_adapter::{AptosCodeStorageAdapter, AsAptosCodeStorage};
