// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use move_binary_format::file_format::CompiledScript;
use move_vm_runtime::Script;
use std::sync::Arc;

/// An entry for the script cache, used by the Aptos code cache. Entries can live in the cache in
/// different representations.
#[derive(Debug, Clone)]
pub enum ScriptCacheEntry {
    /// Deserialized script, not verified with bytecode verifier.
    Deserialized(Arc<CompiledScript>),
    /// Verified script.
    Verified(Arc<Script>),
}

impl ScriptCacheEntry {
    /// Returns the deserialized script ([CompiledScript]).
    pub fn compiled_script(&self) -> &Arc<CompiledScript> {
        match self {
            Self::Deserialized(compiled_script) => compiled_script,
            Self::Verified(script) => script.compiled_script(),
        }
    }
}
