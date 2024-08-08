// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use move_vm_runtime::{DummyCodeStorage, ModuleStorage};

pub trait AptosModuleStorage: ModuleStorage {}

impl AptosModuleStorage for DummyCodeStorage {}
