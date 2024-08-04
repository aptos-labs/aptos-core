// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use move_vm_runtime::{DummyCodeStorage, ScriptStorage};

/// Represents script storage used by the Aptos blockchain.
pub trait AptosScriptStorage: ScriptStorage {}

impl AptosScriptStorage for DummyCodeStorage {}
