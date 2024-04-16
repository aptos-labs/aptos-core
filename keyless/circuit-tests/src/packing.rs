
// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use aptos_keyless_common::input_processing::circuit_input_signals::CircuitInputSignals;
use aptos_keyless_common::input_processing::config::CircuitPaddingConfig;
use crate::TestCircuitHandle;
use itertools::*;
use rand_chacha::{rand_core::{SeedableRng as _, RngCore as _}, ChaCha20Rng};


fn expected_num2bits_be(n: u64, size: usize) -> Vec<u8> {
    let mut bits_le = Vec::new();
    for i in 0..size {
        bits_le.push( ( (n >> i) & 1 ) as u8 );
    }
    bits_le.into_iter().rev().collect()
}

#[test]
fn num2bits_be_test() {
    let circuit_handle = TestCircuitHandle::new("packing/num2bits_be_test.circom").unwrap();
    let bits_max_size = 8;


    for n in 0..=255 {
        let config = CircuitPaddingConfig::new()
            .max_length("bits_out", bits_max_size);

        let circuit_input_signals = CircuitInputSignals::new()
            .byte_input("num_in", n)
            .bytes_input("bits_out", &expected_num2bits_be(n as u64, bits_max_size))
            .pad(&config).unwrap();

        let result = circuit_handle.gen_witness(circuit_input_signals);
        println!("{:?}", result);
        assert!(result.is_ok());
    }
}

#[test]
fn bits2num_big_endian_test() {
    let circuit_handle = TestCircuitHandle::new("packing/bits2num_big_endian_test.circom").unwrap();
    let mut rng = ChaCha20Rng::seed_from_u64(2513478);
    let bits_max_size = 64;

    for i in 0..=255 {
        let expected_n = rng.next_u64();
        let bits_in = expected_num2bits_be(expected_n, bits_max_size);

        let config = CircuitPaddingConfig::new()
            .max_length("bits_in", bits_max_size);

        let circuit_input_signals = CircuitInputSignals::new()
            .bytes_input("bits_in", &expected_num2bits_be(expected_n, bits_max_size))
            .u64_input("num_out", expected_n)
            .pad(&config).unwrap();

        let result = circuit_handle.gen_witness(circuit_input_signals);
        println!("{:?}", result);
        assert!(result.is_ok());
   }
}

#[test]
fn bytes_to_bits_test() {
    let circuit_handle = TestCircuitHandle::new("packing/bytes_to_bits_test.circom").unwrap();

    let mut rng = ChaCha20Rng::seed_from_u64(2513478);
    const bytes_max_size : usize = 10;
    const bits_max_size : usize = bytes_max_size * 8;


    for i in 0..=255 {
        let bytes : &mut [u8] = &mut [0u8; bytes_max_size];
        rng.fill_bytes(bytes);

        let expected_bits : Vec<u8> = bytes
            .into_iter()
            .map(|byte| expected_num2bits_be(*byte as u64, 8))
            .flatten()
            .collect();

        let config = CircuitPaddingConfig::new()
            .max_length("bytes_in", bytes_max_size)
            .max_length("bits_out", bits_max_size);

        let circuit_input_signals = CircuitInputSignals::new()
            .bytes_input("bytes_in", bytes)
            .bytes_input("bits_out", &expected_bits)
            .pad(&config).unwrap();

        let result = circuit_handle.gen_witness(circuit_input_signals);
        println!("{:?}", result);
        assert!(result.is_ok());
    }
}


