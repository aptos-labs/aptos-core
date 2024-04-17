// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::TestCircuitHandle;
use aptos_keyless_common::input_processing::{
    circuit_input_signals::CircuitInputSignals, config::CircuitPaddingConfig,
};
use rand::{thread_rng, RngCore};
use sha2::Digest;
use std::sync::Arc;

fn sha256_padding(message: &[u8]) -> Vec<u8> {
    let mut padded_message = Vec::from(message);
    let message_bit_length = message.len() * 8;

    // Append the bit '1' to the message
    padded_message.push(0x80); // 0x80 is 10000000 in binary

    // Append zeros until the message length is 448 mod 512
    let current_length_bits = (padded_message.len() * 8) % 512;
    let padding_bits = (960 - current_length_bits) % 512;

    // Convert padding from bits to bytes and subtract 1 byte (the 0x80 already added)
    let padding_bytes = padding_bits / 8;
    padded_message.extend(vec![0; padding_bytes]);

    // Append the length of the original message as a 64-bit big-endian integer
    let bit_len_bytes = (message_bit_length as u64).to_be_bytes();
    padded_message.extend_from_slice(&bit_len_bytes);

    padded_message
}

fn byte_to_bits_msb(byte: u8) -> Vec<bool> {
    (0..8).map(|i| (byte >> (7 - i)) & 1 != 0).collect()
}

fn bytes_to_bits_msb(msg: Vec<u8>) -> Vec<bool> {
    msg.into_iter().flat_map(byte_to_bits_msb).collect()
}

#[test]
fn sha_test() {
    let mut rng = thread_rng();
    let circuit_handle = Arc::new(TestCircuitHandle::new("sha_test.circom").unwrap());

    // TODO: figure out how to parallelize and why `tokio::task::spawn()` does not work.
    // Is it supported to do multiple `node generate_witness.js xxx` in parallel at all?
    for input_byte_len in 0..248 {
        let mut input = vec![0; input_byte_len];
        rng.fill_bytes(&mut input);
        let padded_input = sha256_padding(input.as_slice());
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
