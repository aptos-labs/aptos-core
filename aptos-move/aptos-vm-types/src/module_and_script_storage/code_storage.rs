// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::module_and_script_storage::module_storage::AptosModuleStorage;
use move_vm_runtime::CodeStorage;

/// Represents code storage used by the Aptos blockchain, capable of caching scripts and modules.
pub trait AptosCodeStorage: AptosModuleStorage + CodeStorage {}

impl<T: AptosModuleStorage + CodeStorage> AptosCodeStorage for T {}
