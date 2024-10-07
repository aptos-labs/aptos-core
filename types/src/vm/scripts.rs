// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use move_binary_format::file_format::CompiledScript;
use move_vm_runtime::Script;
use std::sync::Arc;

/// An entry for the script cache, used by the Aptos code cache. Entries
/// can live in the cache in different states (deserialized / verified).
#[derive(Debug, Clone)]
pub enum ScriptCacheEntry {
    Deserialized(Arc<CompiledScript>),
    Verified(Arc<Script>),
}

impl ScriptCacheEntry {
    /// Returns the deserialized (compiled) representation of the script.
    pub fn as_compiled_script(&self) -> Arc<CompiledScript> {
        match self {
            Self::Deserialized(compiled_script) => compiled_script.clone(),
            Self::Verified(script) => script.as_compiled_script(),
        }
    }

    /// Returns true if the script entry has already been verified.
    pub fn is_verified(&self) -> bool {
        match self {
            Self::Verified(_) => true,
            Self::Deserialized(_) => false,
        }
    }
}
