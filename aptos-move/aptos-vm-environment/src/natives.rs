// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use aptos_native_interface::SafeNativeBuilder;
use move_core_types::language_storage::CORE_CODE_ADDRESS;
use move_vm_runtime::native_functions::NativeFunctionTable;

pub fn aptos_natives_with_builder(
    builder: &mut SafeNativeBuilder,
    inject_create_signer_for_gov_sim: bool,
) -> NativeFunctionTable {
    #[allow(unreachable_code)]
    aptos_move_stdlib::natives::all_natives(CORE_CODE_ADDRESS, builder)
        .into_iter()
        .filter(|(_, name, _, _)| name.as_str() != "vector")
        .chain(aptos_framework::natives::all_natives(
            CORE_CODE_ADDRESS,
            builder,
            inject_create_signer_for_gov_sim,
        ))
        .chain(aptos_table_natives::table_natives(
            CORE_CODE_ADDRESS,
            builder,
        ))
        .collect()
}
