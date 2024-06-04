// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::TestCircuitHandle;
use aptos_keyless_common::input_processing::{
    circuit_input_signals::CircuitInputSignals, config::CircuitPaddingConfig,
};
use itertools::*;

fn expected_decoded(b64: &str) -> Vec<u8> {
    base64::decode_config(b64, base64::URL_SAFE_NO_PAD).unwrap()
}

fn expected_lookup(b64_char: u8) -> u8 {
    match b64_char {
        b'A'..=b'Z' => b64_char - b'A',
        b'a'..=b'z' => b64_char - b'a' + 26,
        b'0'..=b'9' => b64_char - b'0' + 52,
        b'-' => 62,
        b'_' => 63,
        _ => panic!("Tried to lookup a non-base64 char."),
    }
}

#[test]
fn base64_lookup_test() {
    let circuit_handle = TestCircuitHandle::new("base64_lookup_test.circom").unwrap();

    let base64_chars = (b'A'..=b'Z')
        .chain(b'a'..=b'z')
        .chain(b'0'..=b'9')
        .chain([b'-', b'_']);

    for in_b64_char in base64_chars {
        let config = CircuitPaddingConfig::new();

        let circuit_input_signals = CircuitInputSignals::new()
            .byte_input("in_b64_char", in_b64_char)
            .byte_input("out_num", expected_lookup(in_b64_char))
            .pad(&config)
            .unwrap();

        let result = circuit_handle.gen_witness(circuit_input_signals);
        assert!(result.is_ok());
    }
}

#[test]
fn base64_decode_test_short_all_dashes() {
    let circuit_handle = TestCircuitHandle::new("base64_decode_test_short.circom").unwrap();

    let jwt_payload = "----";

    let ascii_jwt_payload = expected_decoded(jwt_payload);

    let max_jwt_payload_len = 4;
    let max_ascii_jwt_payload_len = 3 * max_jwt_payload_len / 4;
    let config = CircuitPaddingConfig::new()
        .max_length("jwt_payload", max_jwt_payload_len)
        .max_length("ascii_jwt_payload", max_ascii_jwt_payload_len);

    let circuit_input_signals = CircuitInputSignals::new()
        .str_input("jwt_payload", jwt_payload)
        .bytes_input("ascii_jwt_payload", &ascii_jwt_payload)
        .pad(&config)
        .unwrap();

    let result = circuit_handle.gen_witness(circuit_input_signals);
    assert!(result.is_ok());
}

#[test]
fn base64_decode_test_short_three_chars() {
    let circuit_handle = TestCircuitHandle::new("base64_decode_test_short.circom").unwrap();

    // Last character must have ascii encoding with two trailing zeros, since we are encoding
    // 16 bits
    let jwt_payload = "--E";

    let ascii_jwt_payload = expected_decoded(jwt_payload);

    let max_jwt_payload_len = 4;
    let max_ascii_jwt_payload_len = 3 * max_jwt_payload_len / 4;
    let config = CircuitPaddingConfig::new()
        .max_length("jwt_payload", max_jwt_payload_len)
        .max_length("ascii_jwt_payload", max_ascii_jwt_payload_len);

    let circuit_input_signals = CircuitInputSignals::new()
        .str_input("jwt_payload", jwt_payload)
        .bytes_input("ascii_jwt_payload", &ascii_jwt_payload)
        .pad(&config)
        .unwrap();

    let result = circuit_handle.gen_witness(circuit_input_signals);
    assert!(result.is_ok());
}

// ignoring b/c takes forever
#[test]
#[ignore]
fn base64_decode_test_short_exhaustive() {
    let circuit_handle = TestCircuitHandle::new("base64_decode_test_short.circom").unwrap();

    let base64_chars = ('A'..='Z')
        .chain('a'..='z')
        .chain('0'..='9')
        .chain(['-', '_']);

    let exhaustive_iter = (0..=3)
        .map(|_| base64_chars.clone())
        .multi_cartesian_product()
        .map(|s| s.into_iter().collect::<String>());
    //.collect();

    for jwt_payload in exhaustive_iter {
        println!("{jwt_payload}");

        let ascii_jwt_payload = expected_decoded(&jwt_payload);

        let max_jwt_payload_len = 4;
        let max_ascii_jwt_payload_len = 3 * max_jwt_payload_len / 4;
        let config = CircuitPaddingConfig::new()
            .max_length("jwt_payload", max_jwt_payload_len)
            .max_length("ascii_jwt_payload", max_ascii_jwt_payload_len);

        let circuit_input_signals = CircuitInputSignals::new()
            .str_input("jwt_payload", &jwt_payload)
            .bytes_input("ascii_jwt_payload", &ascii_jwt_payload)
            .pad(&config)
            .unwrap();

        let result = circuit_handle.gen_witness(circuit_input_signals);
        assert!(result.is_ok());
    }
}
