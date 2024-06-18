// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::TestCircuitHandle;
use aptos_keyless_common::input_processing::{
    circuit_input_signals::CircuitInputSignals, config::CircuitPaddingConfig,
};
use ark_bn254::Fr;
use ark_ff::{BigInteger, PrimeField};
use rand_chacha::{
    rand_core::{RngCore as _, SeedableRng as _},
    ChaCha20Rng,
};

const TEST_RNG_SEED: u64 = 6896401633680249901;

fn expected_num2bits_be(n: u64, size: usize) -> Vec<bool> {
    let mut bits_le = Vec::new();
    for i in 0..size {
        bits_le.push(((n >> i) & 1) == 1);
    }
    bits_le.into_iter().rev().collect()
}

fn bits_to_field_elems(bits: &[bool], bits_per_field_elem: usize) -> Vec<Fr> {
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
        let config = CircuitPaddingConfig::new().max_length("bits_out", bits_max_size);

        let circuit_input_signals = CircuitInputSignals::new()
            .byte_input("num_in", n)
            .bools_input("bits_out", &expected_num2bits_be(n as u64, bits_max_size))
            .pad(&config)
            .unwrap();

        let result = circuit_handle.gen_witness(circuit_input_signals);
        println!("n: {}, {:?}", n, result);
        assert!(result.is_ok());
    }
}

#[test]
fn bits2num_big_endian_test() {
    let circuit_handle = TestCircuitHandle::new("packing/bits2num_big_endian_test.circom").unwrap();
    let mut rng = ChaCha20Rng::seed_from_u64(TEST_RNG_SEED);
    let bits_max_size = 64;

    for i in 0..=255 {
        let expected_n = rng.next_u64();

        let config = CircuitPaddingConfig::new().max_length("bits_in", bits_max_size);

        let circuit_input_signals = CircuitInputSignals::new()
            .bools_input("bits_in", &expected_num2bits_be(expected_n, bits_max_size))
            .u64_input("num_out", expected_n)
            .pad(&config)
            .unwrap();

        let result = circuit_handle.gen_witness(circuit_input_signals);
        println!("i: {}, {:?}", i, result);
        assert!(result.is_ok());
    }
}

#[test]
fn bytes_to_bits_test() {
    let circuit_handle = TestCircuitHandle::new("packing/bytes_to_bits_test.circom").unwrap();

    let mut rng = ChaCha20Rng::seed_from_u64(TEST_RNG_SEED);
    const BYTES_MAX_SIZE: usize = 10;
    const BITS_MAX_SIZE: usize = BYTES_MAX_SIZE * 8;

    for i in 0..=255 {
        let bytes: &mut [u8] = &mut [0u8; BYTES_MAX_SIZE];
        rng.fill_bytes(bytes);

        let expected_bits: Vec<bool> = bytes
            .iter()
            .flat_map(|byte| expected_num2bits_be(*byte as u64, 8))
            .collect();

        let config = CircuitPaddingConfig::new()
            .max_length("bytes_in", BYTES_MAX_SIZE)
            .max_length("bits_out", BITS_MAX_SIZE);

        let circuit_input_signals = CircuitInputSignals::new()
            .bytes_input("bytes_in", bytes)
            .bools_input("bits_out", &expected_bits)
            .pad(&config)
            .unwrap();

        let result = circuit_handle.gen_witness(circuit_input_signals);
        println!("i: {}, {:?}", i, result);
        assert!(result.is_ok());
    }
}

#[test]
fn bits_to_field_elems_test() {
    let circuit_handle = TestCircuitHandle::new("packing/bits_to_field_elems_test.circom").unwrap();

    let mut rng = ChaCha20Rng::seed_from_u64(TEST_RNG_SEED);

    // Keeping both of these constant makes sense here, since the circuit only ever uses these two
    // particular parameters when calling this template
    const MAX_BITS_LEN: usize = 256;
    const BITS_PER_FIELD_ELEM: usize = 64;
    let num_field_elems = MAX_BITS_LEN.div_ceil(BITS_PER_FIELD_ELEM);

    for i in 0..=255 {
        let bytes: &mut [u8] = &mut [0u8; MAX_BITS_LEN / 8];
        rng.fill_bytes(bytes);

        let bits: Vec<bool> = bytes
            .iter()
            .flat_map(|byte| expected_num2bits_be(*byte as u64, 8))
            .collect();

        let expected_field_elems = bits_to_field_elems(&bits, BITS_PER_FIELD_ELEM);
        println!("{}", expected_field_elems.len());

        let config = CircuitPaddingConfig::new()
            .max_length("bits_in", MAX_BITS_LEN)
            .max_length("field_elems_out", num_field_elems);

        let circuit_input_signals = CircuitInputSignals::new()
            .bools_input("bits_in", &bits)
            .frs_input("field_elems_out", &expected_field_elems)
            .pad(&config)
            .unwrap();

        let result = circuit_handle.gen_witness(circuit_input_signals);
        println!("i: {}, {:?}", i, result);
        assert!(result.is_ok());
    }
}
