// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use crate::move_vm_ext::{aggregator_natives, NativeAggregatorContext};
use aptos_gas::NativeGasParameters;
use aptos_types::account_config::CORE_CODE_ADDRESS;
use framework::natives::{code::NativeCodeContext, transaction_context::NativeTransactionContext};
use move_deps::move_unit_test;
use move_deps::move_vm_runtime::native_extensions::NativeContextExtensions;
use move_deps::move_vm_test_utils::BlankStorage;
use move_deps::{
    move_stdlib, move_table_extension, move_vm_runtime::native_functions::NativeFunctionTable,
};
use once_cell::sync::Lazy;

static DUMMY_RESOLVER: Lazy<BlankStorage> = Lazy::new(|| BlankStorage);

pub fn aptos_natives(gas_params: NativeGasParameters) -> NativeFunctionTable {
    move_stdlib::natives::all_natives(CORE_CODE_ADDRESS, gas_params.move_stdlib)
        .into_iter()
        .chain(framework::natives::all_natives(
            CORE_CODE_ADDRESS,
            gas_params.aptos_framework,
        ))
        .chain(move_table_extension::table_natives(CORE_CODE_ADDRESS))
        .chain(aggregator_natives(CORE_CODE_ADDRESS))
        // TODO(Gas): this isn't quite right yet...
        .chain(
            move_stdlib::natives::nursery_natives(
                CORE_CODE_ADDRESS,
                move_stdlib::natives::NurseryGasParameters::zeros(),
            )
            .into_iter()
            .filter(|(addr, module_name, _, _)| {
                !(*addr == CORE_CODE_ADDRESS && module_name.as_str() == "event")
            }),
        )
        .collect()
}

pub fn configure_for_unit_test() {
    move_unit_test::extensions::set_extension_hook(Box::new(unit_test_extensions_hook))
}

fn unit_test_extensions_hook(exts: &mut NativeContextExtensions) {
    exts.add(NativeCodeContext::default());
    exts.add(NativeTransactionContext::new(vec![1]));
    exts.add(NativeAggregatorContext::new(0, &*DUMMY_RESOLVER));
}
