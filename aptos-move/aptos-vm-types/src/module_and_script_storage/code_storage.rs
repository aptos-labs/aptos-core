// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use crate::module_and_script_storage::module_storage::AptosModuleStorage;
use move_binary_format::file_format::CompiledScript;
use move_vm_runtime::Script;
use move_vm_types::code::ScriptCache;

/// Represents code storage used by the Aptos blockchain, capable of caching scripts and modules.
pub trait AptosCodeStorage:
    AptosModuleStorage + ScriptCache<Key = [u8; 32], Deserialized = CompiledScript, Verified = Script>
{
}

impl<T> AptosCodeStorage for T where
    T: AptosModuleStorage
        + ScriptCache<Key = [u8; 32], Deserialized = CompiledScript, Verified = Script>
{
}
