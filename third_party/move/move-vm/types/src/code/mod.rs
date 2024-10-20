// Copyright (c) The Move Contributors
// SPDX-License-Identifier: Apache-2.0

mod cache;
pub mod errors;
mod storage;

#[cfg(any(test, feature = "testing"))]
pub use cache::types::{MockDeserializedCode, MockVerifiedCode};
pub use cache::{
    module_cache::{
        ambassador_impl_ModuleCache, ModuleCache, ModuleCode, ModuleCodeBuilder, SyncModuleCache,
        UnsyncModuleCache,
    },
    script_cache::{ambassador_impl_ScriptCache, ScriptCache, SyncScriptCache, UnsyncScriptCache},
    types::{Code, WithAddress, WithBytes, WithHash, WithName},
};
pub use storage::ModuleBytesStorage;
