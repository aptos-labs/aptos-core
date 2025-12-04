// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

#![no_main]
use arbitrary::Arbitrary;
use libfuzzer_sys::fuzz_target;
use move_core_types::value::{MoveTypeLayout, MoveValue};

mod utils;
use utils::helpers::is_valid_layout;

#[derive(Arbitrary, Debug)]
struct FuzzData {
    data: Vec<u8>,
    layout: MoveTypeLayout,
}

fuzz_target!(|fuzz_data: FuzzData| {
    if fuzz_data.data.is_empty() || !is_valid_layout(&fuzz_data.layout) {
        return;
    }
    let _ = MoveValue::simple_deserialize(&fuzz_data.data, &fuzz_data.layout);
});
