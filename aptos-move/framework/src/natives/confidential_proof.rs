// Copyright (c) Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use aptos_native_interface::SafeNativeBuilder;
use move_vm_runtime::native_functions::NativeFunction;

pub fn make_all(
    builder: &SafeNativeBuilder,
) -> impl Iterator<Item = (String, NativeFunction)> + '_ {
    let mut natives = vec![];

    natives.extend([
        (
            "verify_batch_range_proof_internal",
            crate::natives::cryptography::bulletproofs::native_verify_batch_range_proof,
        ),
    ]);

    builder.make_named_natives(natives)
}
