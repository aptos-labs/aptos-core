// Copyright (c) The Move Contributors
// SPDX-License-Identifier: Apache-2.0

mod cache;
pub mod errors;
mod storage;

#[cfg(any(test, feature = "testing"))]
pub use cache::test_types::{
    mock_deserialized_code, mock_extension, mock_verified_code, MockDeserializedCode,
    MockExtension, MockVerifiedCode,
};
pub use cache::{
    module_cache::{
        ambassador_impl_ModuleCache, ModuleCache, ModuleCode, ModuleCodeBuilder, SyncModuleCache,
        UnsyncModuleCache,
    },
    script_cache::{ambassador_impl_ScriptCache, ScriptCache, SyncScriptCache, UnsyncScriptCache},
    types::{Code, WithAddress, WithBytes, WithHash, WithName, WithSize},
};
pub use storage::ModuleBytesStorage;
