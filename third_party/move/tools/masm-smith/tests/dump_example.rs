// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Prints the rendered `.masm` text for a fixed seed. Run with:
//!   cargo test -p masm-smith --test dump_example -- --nocapture

use arbitrary::Unstructured;
use masm_smith::{generate, Config};

#[test]
fn dump_seed_one() {
    let seed: &[u8] = &[
        0x10, 0x20, 0x30, 0x40, 0x50, 0x60, 0x70, 0x80, 0x90, 0xA0, 0xB0, 0xC0, 0xD0, 0xE0, 0xF0,
        0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08,
    ];
    let mut u = Unstructured::new(seed);
    let test = generate(&mut u, &Config::default()).expect("generation");
    let text = test.render();
    println!("---\n{}\n---", text);
}
