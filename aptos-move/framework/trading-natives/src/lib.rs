// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use aptos_native_interface::SafeNativeBuilder;
use move_core_types::account_address::AccountAddress;
use move_vm_runtime::native_functions::{NativeFunctionTable, make_table_from_iter};


pub mod orders_index;
pub mod order_book_types;


pub fn all_natives(
    framework_addr: AccountAddress,
    builder: &SafeNativeBuilder,
) -> NativeFunctionTable {
    let mut natives = vec![];

    macro_rules! add_natives_from_module {
        ($module_name:expr, $natives:expr) => {
            natives.extend(
                $natives.map(|(func_name, func)| ($module_name.to_string(), func_name, func)),
            );
        };
    }

    add_natives_from_module!("orders_index", orders_index::make_all(builder));

    make_table_from_iter(framework_addr, natives)
}
