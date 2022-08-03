// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use crate::move_vm_ext::transaction_context_natives;
use crate::move_vm_ext::{code_natives, NativeCodeContext, NativeTransactionContext};
use aptos_types::account_config::CORE_CODE_ADDRESS;
use move_deps::move_unit_test;
use move_deps::move_vm_runtime::native_extensions::NativeContextExtensions;
use move_deps::{
    move_stdlib, move_table_extension, move_vm_runtime::native_functions::NativeFunctionTable,
};

pub fn aptos_natives() -> NativeFunctionTable {
    move_stdlib::natives::all_natives(CORE_CODE_ADDRESS)
        .into_iter()
        .chain(framework::natives::all_natives(CORE_CODE_ADDRESS))
        .chain(move_table_extension::table_natives(CORE_CODE_ADDRESS))
        .chain(transaction_context_natives(CORE_CODE_ADDRESS))
        .chain(code_natives(CORE_CODE_ADDRESS))
        .collect()
}

pub fn configure_for_unit_test() {
    move_unit_test::extensions::set_extension_hook(Box::new(unit_test_extensions_hook))
}

fn unit_test_extensions_hook(exts: &mut NativeContextExtensions) {
    exts.add(NativeCodeContext::default());
    exts.add(NativeTransactionContext::new(vec![1]))
}
