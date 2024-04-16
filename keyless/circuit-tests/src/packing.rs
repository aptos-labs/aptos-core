
// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use aptos_keyless_common::input_processing::circuit_input_signals::CircuitInputSignals;
use aptos_keyless_common::input_processing::config::CircuitPaddingConfig;
use crate::TestCircuitHandle;
use itertools::*;


fn expected_num2bits_be(n: u8, size: usize) -> Vec<u8> {
    let mut bits_le = Vec::new();
    for i in 0..size {
        bits_le.push( (n >> i) & 1 );
    }
    bits_le.into_iter().rev().collect()
}

#[test]
fn num2bits_be_test() {
    let circuit_handle = TestCircuitHandle::new("packing/num2bits_be_test.circom").unwrap();

    for n in 0..=255 {
        let config = CircuitPaddingConfig::new()
            .max_length("bits_out", 8);

        let circuit_input_signals = CircuitInputSignals::new()
            .byte_input("num_in", n)
            .bytes_input("bits_out", &expected_num2bits_be(n, 8))
            .pad(&config).unwrap();

        let result = circuit_handle.gen_witness(circuit_input_signals);
        println!("{:?}", result);
        assert!(result.is_ok());
    }
}

