// Copyright © Velor Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::module_and_script_storage::module_storage::VelorModuleStorage;
use move_binary_format::file_format::CompiledScript;
use move_vm_runtime::Script;
use move_vm_types::code::ScriptCache;

/// Represents code storage used by the Velor blockchain, capable of caching scripts and modules.
pub trait VelorCodeStorage:
    VelorModuleStorage + ScriptCache<Key = [u8; 32], Deserialized = CompiledScript, Verified = Script>
{
}

impl<T> VelorCodeStorage for T where
    T: VelorModuleStorage
        + ScriptCache<Key = [u8; 32], Deserialized = CompiledScript, Verified = Script>
{
}
