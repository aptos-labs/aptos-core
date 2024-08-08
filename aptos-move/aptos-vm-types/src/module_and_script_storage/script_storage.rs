// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use move_vm_runtime::{DummyCodeStorage, ScriptStorage};

pub trait AptosScriptStorage: ScriptStorage {}

impl AptosScriptStorage for DummyCodeStorage {}
