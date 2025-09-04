// Copyright © Velor Foundation
// SPDX-License-Identifier: Apache-2.0

pub mod code_storage;
pub mod module_storage;

mod state_view_adapter;
pub use state_view_adapter::{VelorCodeStorageAdapter, AsVelorCodeStorage};
