// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Checks that the same seed and config always produce the same `.masm` text, which corpus
//! replay relies on.

use arbitrary::Unstructured;
use masm_smith::{generate, Config};

const SEEDS: &[&[u8]] = &[
    &[
        0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08, 0x09, 0x0A, 0x0B, 0x0C,
    ],
    &[
        0xDE, 0xAD, 0xBE, 0xEF, 0xCA, 0xFE, 0xBA, 0xBE, 0x12, 0x34, 0x56, 0x78, 0x9A, 0xBC,
    ],
    &[0xFF; 64],
];

#[test]
fn seed_and_config_produce_byte_identical_output() {
    let cfg = Config::default();
    for (i, seed) in SEEDS.iter().enumerate() {
        let mut u1 = Unstructured::new(seed);
        let mut u2 = Unstructured::new(seed);
        let t1 = generate(&mut u1, &cfg);
        let t2 = generate(&mut u2, &cfg);
        match (t1, t2) {
            (Ok(a), Ok(b)) => assert_eq!(
                a.render(),
                b.render(),
                "seed #{} produced different outputs on repeat generation",
                i
            ),
            (Err(masm_smith::Error::OutOfInput), Err(masm_smith::Error::OutOfInput)) => {},
            (Err(e), _) | (_, Err(e)) => panic!("seed #{} generation error: {e}", i),
        }
    }
}
