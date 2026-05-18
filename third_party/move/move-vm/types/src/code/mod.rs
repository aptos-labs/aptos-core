// Parts of the file are Copyright (c) The Diem Core Contributors
// Parts of the file are Copyright (c) The Move Contributors
// Parts of the file are Copyright (c) Aptos Foundation
// All Aptos Foundation code and content is licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

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
