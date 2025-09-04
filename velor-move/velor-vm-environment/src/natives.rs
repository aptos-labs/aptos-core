// Copyright © Velor Foundation
// SPDX-License-Identifier: Apache-2.0

use velor_native_interface::SafeNativeBuilder;
use move_core_types::language_storage::CORE_CODE_ADDRESS;
use move_vm_runtime::native_functions::NativeFunctionTable;
use std::collections::HashSet;

/// Builds and returns all Velor native functions.
pub fn velor_natives_with_builder(
    builder: &mut SafeNativeBuilder,
    inject_create_signer_for_gov_sim: bool,
) -> NativeFunctionTable {
    let vector_bytecode_instruction_methods = HashSet::from([
        "empty",
        "length",
        "borrow",
        "borrow_mut",
        "push_back",
        "pop_back",
        "destroy_empty",
        "swap",
    ]);

    #[allow(unreachable_code)]
    velor_move_stdlib::natives::all_natives(CORE_CODE_ADDRESS, builder)
        .into_iter()
        .filter(|(_, name, func_name, _)|
            if name.as_str() == "vector" && vector_bytecode_instruction_methods.contains(func_name.as_str()) {
                println!("ERROR: Tried to register as native a vector bytecode_instruction method {}, skipping.", func_name.as_str());
                false
            } else {
                true
            }
        )
        .chain(velor_framework::natives::all_natives(
            CORE_CODE_ADDRESS,
            builder,
            inject_create_signer_for_gov_sim,
        ))
        .chain(velor_table_natives::table_natives(
            CORE_CODE_ADDRESS,
            builder,
        ))
        .collect()
}
