// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use crate::move_vm_ext::transaction_context_natives;
use aptos_types::account_config::CORE_CODE_ADDRESS;
use move_deps::{
    move_stdlib, move_table_extension, move_vm_runtime::native_functions::NativeFunctionTable,
};

pub fn aptos_natives() -> NativeFunctionTable {
    move_stdlib::natives::all_natives(CORE_CODE_ADDRESS)
        .into_iter()
        .chain(framework::natives::all_natives(CORE_CODE_ADDRESS))
        .chain(move_table_extension::table_natives(CORE_CODE_ADDRESS))
        .chain(transaction_context_natives(CORE_CODE_ADDRESS))
        .chain(aptos_extensions::aggregator_natives(CORE_CODE_ADDRESS))
        .collect()
}
