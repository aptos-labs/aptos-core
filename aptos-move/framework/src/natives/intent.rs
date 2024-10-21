// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use super::dispatchable_fungible_asset::native_dispatch;
use aptos_native_interface::{
    RawSafeNative, SafeNativeBuilder,
};
use move_vm_runtime::native_functions::NativeFunction;

/***************************************************************************************************
 * module
 *
 **************************************************************************************************/
pub fn make_all(
    builder: &SafeNativeBuilder,
) -> impl Iterator<Item = (String, NativeFunction)> + '_ {
    let natives = [
        ("dispatch_consumption", native_dispatch as RawSafeNative),
    ];

    builder.make_named_natives(natives)
}
