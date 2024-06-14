// Copyright (c) Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

#[macro_use]
extern crate afl;

use arbitrary::Unstructured;
use move_smith::{utils::run_transactional_test, CodeGenerator, MoveSmith};

fn main() {
    fuzz!(|data: &[u8]| {
        let u = &mut Unstructured::new(data);
        let mut smith = MoveSmith::default();
        match smith.generate(u) {
            Ok(()) => (),
            Err(_) => return,
        };
        let code = smith.get_compile_unit().emit_code();
        run_transactional_test(code, Some(&smith.config)).unwrap();
    });
}
