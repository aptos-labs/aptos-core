// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::module_and_script_storage::module_storage::{AptosModuleStorage, TAptosModuleStorage};
use aptos_types::{executable::ModulePath, state_store::state_key::StateKey};
use move_vm_runtime::CodeStorage;
use std::fmt::Debug;

/// Represents code storage used by the Aptos blockchain, capable of caching scripts and modules.
pub trait AptosCodeStorage: TAptosCodeStorage<StateKey> {}

impl<C: TAptosCodeStorage<StateKey> + AptosModuleStorage> AptosCodeStorage for C {}

/// Similar to [AptosCodeStorage], but allows Aptos module storage to use generic keys.
pub trait TAptosCodeStorage<K: Debug + ModulePath>:
    TAptosModuleStorage<Key = K> + CodeStorage
{
}
