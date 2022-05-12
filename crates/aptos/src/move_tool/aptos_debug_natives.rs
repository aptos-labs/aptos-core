// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use aptos_types::account_config::CORE_CODE_ADDRESS;
use move_deps::{
    move_stdlib, move_table_extension, move_vm_runtime::native_functions::NativeFunctionTable,
};

// move_stdlib has the testing feature enabled to include debug native functions
pub fn aptos_debug_natives() -> NativeFunctionTable {
    move_stdlib::natives::all_natives(CORE_CODE_ADDRESS)
        .into_iter()
        .chain(framework::natives::all_natives(CORE_CODE_ADDRESS))
        .chain(move_table_extension::table_natives(CORE_CODE_ADDRESS))
        .collect()
}
