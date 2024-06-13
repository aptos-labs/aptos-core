// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::TestCircuitHandle;
use aptos_keyless_common::input_processing::{
    circuit_input_signals::CircuitInputSignals, config::CircuitPaddingConfig, sha,
};
use rand_chacha::{
    rand_core::{RngCore as _, SeedableRng as _},
    ChaCha20Rng,
};
use sha2::Digest;
use std::sync::Arc;

const TEST_RNG_SEED: u64 = 13673030044145830633;

fn byte_to_bits_msb(byte: u8) -> Vec<bool> {
    (0..8).map(|i| (byte >> (7 - i)) & 1 != 0).collect()
}

pub fn bytes_to_bits_msb(msg: Vec<u8>) -> Vec<bool> {
    msg.into_iter().flat_map(byte_to_bits_msb).collect()
}

#[test]
fn sha_test() {
    let mut rng = ChaCha20Rng::seed_from_u64(TEST_RNG_SEED);
    let circuit_handle = Arc::new(TestCircuitHandle::new("sha_test.circom").unwrap());

    // TODO: figure out how to parallelize and why `tokio::task::spawn()` does not work.
    // Is it supported to do multiple `node generate_witness.js xxx` in parallel at all?
    for input_byte_len in 0..248 {
        let mut input = vec![0; input_byte_len];
        rng.fill_bytes(&mut input);
        let padded_input = sha::with_sha_padding_bytes(&input);
        let padded_input_bits = bytes_to_bits_msb(padded_input);

        let mut hasher = sha2::Sha256::new();
        hasher.update(input);
        let expected_output = hasher.finalize().to_vec();
        let expected_output_bits = bytes_to_bits_msb(expected_output);

        let config = CircuitPaddingConfig::new()
            .max_length("padded_input_bits", 2048) // should align with the `max_num_blocks=4` in `sha_test.circom`.
            .max_length("expected_digest_bits", 256);

        let circuit_input_signals = CircuitInputSignals::new()
            .bits_input("padded_input_bits", padded_input_bits.as_slice())
            .usize_input("input_bit_len", padded_input_bits.len())
            .bits_input("expected_digest_bits", expected_output_bits.as_slice())
            .pad(&config)
            .unwrap();
        let result = circuit_handle.gen_witness(circuit_input_signals);
        println!("input_byte_len={}, result={:?}", input_byte_len, result);
        assert!(result.is_ok());
    }
}

#[test]
fn sha_padding_verify_test() {
    let mut rng = ChaCha20Rng::seed_from_u64(TEST_RNG_SEED);
    let circuit_handle =
        Arc::new(TestCircuitHandle::new("sha2_padding_verify_test.circom").unwrap());

    for input_byte_len in 180..248 {
        let input_bit_len = input_byte_len * 8;
        let mut input = vec![0; input_byte_len];
        rng.fill_bytes(&mut input);
        let padded_input = sha::with_sha_padding_bytes(&input);
        let padded_input_byte_len = padded_input.len();
        let config = CircuitPaddingConfig::new()
            .max_length("in", 256)
            .max_length("L_byte_encoded", 8)
            .max_length("padding_without_len", 64);

        let circuit_input_signals = CircuitInputSignals::new()
            .bytes_input("in", padded_input.as_slice())
            .usize_input("num_blocks", padded_input_byte_len / 64)
            .usize_input("padding_start", input_byte_len)
            .bytes_input(
                "L_byte_encoded",
                (input_bit_len as u64).to_be_bytes().as_slice(),
            )
            .bytes_input(
                "padding_without_len",
                &padded_input[input_byte_len..(padded_input_byte_len - 8)],
            )
            .pad(&config)
            .unwrap();
        println!("circuit_input_signals={:?}", circuit_input_signals);
        let result = circuit_handle.gen_witness(circuit_input_signals);
        println!("input_byte_len={}, result={:?}", input_byte_len, result);
        assert!(result.is_ok());
    }
}
