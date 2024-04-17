
// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use aptos_keyless_common::input_processing::circuit_input_signals::CircuitInputSignals;
use aptos_keyless_common::input_processing::config::CircuitPaddingConfig;
use crate::TestCircuitHandle;
use itertools::*;
use rand_chacha::{rand_core::{SeedableRng as _, RngCore as _}, ChaCha20Rng};
use aptos_crypto::poseidon_bn254;
use ark_bn254::Fr;
use ark_ff::PrimeField;
use ark_ff::BigInteger;



fn expected_num2bits_be(n: u64, size: usize) -> Vec<u8> {

    let mut bits_le = Vec::new();
    for i in 0..size {
        bits_le.push( ( (n >> i) & 1 ) as u8 );
    }
    bits_le.into_iter().rev().collect()
}

fn expected_bits_to_field_elems(bits: &[bool], bits_per_field_elem : usize) -> Vec<Fr> {
    bits.chunks(bits_per_field_elem)
        .map(bits_to_field_elem)
        .collect()
}

fn bits_to_field_elem(bits: &[bool]) -> Fr {
    let bigint = BigInteger::from_bits_be(bits);
    Fr::from_bigint(bigint).unwrap()
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

#[test]
fn bits_to_field_elems_test() {
    let circuit_handle = TestCircuitHandle::new("packing/bits_to_field_elems_test.circom").unwrap();

    let mut rng = ChaCha20Rng::seed_from_u64(2513478);

    // Keeping both of these constant makes sense here, since the circuit only ever uses these two
    // particular parameters when calling this template
    const max_bits_len : usize = 256;
    const bits_per_field_elem : usize = 64;
    let num_field_elems = if max_bits_len % bits_per_field_elem == 0 { max_bits_len / bits_per_field_elem } else { (max_bits_len / bits_per_field_elem) + 1 };


    for i in 0..=255 {
        let bytes : &mut [u8] = &mut [ 0u8 ; max_bits_len / 8 ];
        rng.fill_bytes(bytes);

        let bits : Vec<bool> = bytes
            .into_iter()
            .map(|byte| expected_num2bits_be(*byte as u64, 8))
            .flatten()
            .map(|byte| byte != 0)
            .collect();

        let expected_field_elems = expected_bits_to_field_elems(&bits, bits_per_field_elem);
        println!("{}", expected_field_elems.len());

        let config = CircuitPaddingConfig::new()
            .max_length("bits_in", max_bits_len)
            .max_length("field_elems_out", num_field_elems);

        let circuit_input_signals = CircuitInputSignals::new()
            .bools_input("bits_in", &bits)
            .frs_input("field_elems_out", &expected_field_elems)
            .pad(&config).unwrap();

        let result = circuit_handle.gen_witness(circuit_input_signals);
        println!("{:?}", result);
        assert!(result.is_ok());
    }
}


