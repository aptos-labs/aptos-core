// Copyright © Velor Foundation
// SPDX-License-Identifier: Apache-2.0

#![no_main]
use arbitrary::Arbitrary;
use libfuzzer_sys::fuzz_target;
use move_core_types::value::{MoveTypeLayout, MoveValue};
mod utils;
use utils::helpers::is_valid_layout;

#[derive(Arbitrary, Debug)]
struct FuzzData {
    move_value: MoveValue,
    layout: MoveTypeLayout,
}

fuzz_target!(|fuzz_data: FuzzData| {
    if !is_valid_layout(&fuzz_data.layout) {
        return;
    }

    // Undecorate value
    let move_value = fuzz_data.move_value.clone();
    let undecorated_move_value = move_value.undecorate();

    // Decorate value
    let move_value = fuzz_data.move_value.clone();
    let decorated_move_value = move_value.decorate(&fuzz_data.layout);

    // Undecorate decorated value
    decorated_move_value.undecorate();

    // Decorate undecorated value
    undecorated_move_value.decorate(&fuzz_data.layout);
});
