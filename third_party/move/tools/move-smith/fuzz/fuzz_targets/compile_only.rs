// Copyright (c) Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

#![no_main]

use arbitrary::Unstructured;
use libfuzzer_sys::fuzz_target;
use move_smith::{utils::compile_move_code, CodeGenerator, MoveSmith};

fuzz_target!(|data: &[u8]| {
    let u = &mut Unstructured::new(data);
    let mut smith = MoveSmith::default();
    match smith.generate(u) {
        Ok(()) => (),
        Err(_) => return,
    };
    let code = smith.get_compile_unit().emit_code();
    match compile_move_code(code, true, true) {
        true => (),
        false => panic!("Compilation results are different"),
    }
});
