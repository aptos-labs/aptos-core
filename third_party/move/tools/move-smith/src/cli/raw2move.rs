// Copyright (c) Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

//! Simple CLI tool that converts raw bytes read from stdin to Move source code.
//! This can be helpful to check libfuzzer corpus or crashing inputs.

use move_smith::{utils::raw_to_compile_unit, CodeGenerator};
use std::io::{self, Read, Write};

fn main() -> io::Result<()> {
    let mut buffer = Vec::new();
    io::stdin().read_to_end(&mut buffer)?;

    let code = match raw_to_compile_unit(&buffer) {
        Ok(module) => module.emit_code(),
        Err(_) => panic!("Failed to parse raw bytes"),
    };

    io::stdout().write_all(code.as_bytes())
}
