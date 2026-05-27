// Copyright (c) The Move Contributors
// SPDX-License-Identifier: Apache-2.0

use crate::{loader::Script, ModuleStorage};
use move_binary_format::file_format::CompiledScript;
use move_vm_types::code::ScriptCache;

/// Represents storage which in addition to modules, also caches scripts.
pub trait CodeStorage:
    ModuleStorage + ScriptCache<Key = [u8; 32], Deserialized = CompiledScript, Verified = Script>
{
}

impl<T> CodeStorage for T where
    T: ModuleStorage
        + ScriptCache<Key = [u8; 32], Deserialized = CompiledScript, Verified = Script>
{
}
