// Parts of the file are Copyright (c) The Diem Core Contributors
// Parts of the file are Copyright (c) The Move Contributors
// Parts of the file are Copyright (c) Aptos Foundation
// All Aptos Foundation code and content is licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

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
