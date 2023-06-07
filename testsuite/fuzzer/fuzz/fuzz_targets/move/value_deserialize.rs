// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

#![no_main]
use arbitrary::Arbitrary;
use libfuzzer_sys::fuzz_target;
use move_core_types::value::MoveTypeLayout;
use move_vm_types::values::Value;

mod utils;

#[derive(Arbitrary, Debug)]
struct FuzzData {
    data: Vec<u8>,
    layout: MoveTypeLayout,
}

fuzz_target!(|fuzz_data: FuzzData| {
    if fuzz_data.data.is_empty() || !utils::is_valid_layout(&fuzz_data.layout) {
        return;
    }
    let _ = Value::simple_deserialize(&fuzz_data.data, &fuzz_data.layout);
});
