// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use aptos_keyless_common::input_processing::circuit_input_signals::CircuitInputSignals;
use aptos_keyless_common::input_processing::config::CircuitPaddingConfig;
use aptos_keyless_common::input_processing::witness_gen::witness_gen;
use crate::{TestCircuitHandle};

fn expected_decoded(b64: &str) -> Vec<u8> {
    base64::decode_config(
        b64,
        base64::URL_SAFE_NO_PAD,
    ).unwrap()
}

#[test]
fn base64_decode_test() {
    let circuit_handle = TestCircuitHandle::new("base64_decode_test.circom").unwrap();

    let jwt_payload = "eyJpc3MiOiJ0ZXN0Lm9pZGMucHJvdmlkZXIiLCJhenAiOiI1MTEyNzY0NTY4ODAtaTdpNDc4N2MxODYzZGFtdG82ODk5dHM5ODlqMmUzNXIuYXBwcy5nb29nbGV1c2VyY29udGVudC5jb20iLCJhdWQiOiI1MTEyNzY0NTY4ODAtaTdpNDc4N2MxODYzZGFtdG82ODk5dHM5ODlqMmUzNXIuYXBwcy5nb29nbGV1c2VyY29udGVudC5jb20iLCJzdWIiOiIxMDI5MDQ2MzAxNzE1OTI1MjA1OTIiLCJlbWFpbCI6Imhlcm8xMjAwMDkxQGdtYWlsLmNvbSIsImVtYWlsX3ZlcmlmaWVkIjp0cnVlLCJub25jZSI6IjEyNzcyMTIzMTUwODA5NDk2ODYwMTkzNDU3OTc2OTM3MTgyOTY0Mjk3MjgzNjMzNzA1ODcyMzkxNTM0OTQ2ODY2NzE5NjgxOTA0MzExIiwibmJmIjoxNzExNTUyMzMwLCJuYW1lIjoi44Kz44Oz44OJ44Km44OP44Or44KtIiwicGljdHVyZSI6Imh0dHBzOi8vbGgzLmdvb2dsZXVzZXJjb250ZW50LmNvbS9hL0FDZzhvY0lNWmZJa05XR1JCVEQ5MjR4bF9pZWZwTWNjTGd1d2RNSWluTVB6YWo1TDRRPXM5Ni1jIiwiZ2l2ZW5fbmFtZSI6IuODq-OCrSIsImZhbWlseV9uYW1lIjoi44Kz44Oz44OJ44KmIiwiaWF0IjoxNzExNTUyNjMwLCJleHAiOjE5MTE1NTYyMzB9";

    let ascii_jwt_payload = expected_decoded(jwt_payload);


    let max_jwt_payload_len = 192*8-64;
    let max_ascii_jwt_payload_len = 3*max_jwt_payload_len/4;
    let config = CircuitPaddingConfig::new()
        .max_length("jwt_payload", max_jwt_payload_len)
        .max_length("ascii_jwt_payload", max_ascii_jwt_payload_len);

    let circuit_input_signals = CircuitInputSignals::new()
        .str_input("jwt_payload", jwt_payload)
        .bytes_input("ascii_jwt_payload", &ascii_jwt_payload)
        .pad(&config).unwrap();

    assert!(circuit_handle.gen_witness(circuit_input_signals).is_ok());
}

#[test]
fn base64_decode_test_short_all_dashes() {
    let circuit_handle = TestCircuitHandle::new("base64_decode_test_short.circom").unwrap();

    let jwt_payload = "----";

    let ascii_jwt_payload = expected_decoded(jwt_payload);


    let max_jwt_payload_len = 4;
    let max_ascii_jwt_payload_len = 3*max_jwt_payload_len/4;
    let config = CircuitPaddingConfig::new()
        .max_length("jwt_payload", max_jwt_payload_len)
        .max_length("ascii_jwt_payload", max_ascii_jwt_payload_len);

    let circuit_input_signals = CircuitInputSignals::new()
        .str_input("jwt_payload", &jwt_payload)
        .bytes_input("ascii_jwt_payload", &ascii_jwt_payload)
        .pad(&config).unwrap();

    let result = circuit_handle.gen_witness(circuit_input_signals);
    assert!(result.is_ok());
}
