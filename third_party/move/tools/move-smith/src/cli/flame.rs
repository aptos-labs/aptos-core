// Copyright (c) Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use arbitrary::Unstructured;
use move_smith::{codegen::*, config::*, move_smith::*, utils::*};

fn main() {
    let raw_data = get_random_bytes(21289, 8192);
    let mut u = Unstructured::new(&raw_data);

    let mut smith = MoveSmith::default();
    smith.generate(&mut u).unwrap();
    let compile_unit = smith.get_compile_unit();
    let code = compile_unit.emit_code();

    compile_move_code(code.clone(), true, true);

    println!("{}", code.clone());

    run_transactional_test(code.clone(), &Config::default()).unwrap();
}
