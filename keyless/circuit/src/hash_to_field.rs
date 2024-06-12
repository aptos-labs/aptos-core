// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::TestCircuitHandle;
use aptos_crypto::{
    poseidon_bn254::{
        keyless::{pad_and_hash_limbs_with_len, BYTES_PACKED_PER_SCALAR, LIMBS_PACKED_PER_SCALAR},
        pad_and_hash_bytes_no_len, pad_and_hash_bytes_with_len, MAX_NUM_INPUT_SCALARS,
    },
    test_utils::random_bytes,
};
use aptos_keyless_common::input_processing::{
    circuit_input_signals::CircuitInputSignals, config::CircuitPaddingConfig,
};
use ark_bn254::Fr;
use ark_ff::Field;
use rand::{thread_rng, Rng};

#[test]
fn hash_bytes_to_field_with_len() {
    let circuit_src_template = r#"
pragma circom 2.1.3;

include "helpers/hashtofield.circom";

template hash_bytes_to_field_with_len_test(max_len) {
    signal input in[max_len];
    signal input len;
    signal input expected_output;
    component c1 = HashBytesToFieldWithLen(max_len);
    c1.in <== in;
    c1.len <== len;
    expected_output === c1.hash;
}

component main = hash_bytes_to_field_with_len_test(__MAX_LEN__);
"#;

    // Have to save 1 scalar slot for the length.
    let max_supported_byte_len = (MAX_NUM_INPUT_SCALARS - 1) * BYTES_PACKED_PER_SCALAR;

    let mut rng = thread_rng();
    let num_iterations = std::env::var("NUM_ITERATIONS")
        .unwrap_or("10".to_string())
        .parse::<usize>()
        .unwrap_or(10);

    //TODO: hardcode some interesting circuit dimensions that's widely used in keyless.

    for i in 0..num_iterations {
        println!();
        println!("Iteration {} starts.", i);
        let num_bytes_circuit_capacity: usize = rng.gen_range(1, max_supported_byte_len);
        println!("num_bytes_circuit_capacity={}", num_bytes_circuit_capacity);
        let circuit_src = circuit_src_template.replace(
            "__MAX_LEN__",
            num_bytes_circuit_capacity.to_string().as_str(),
        );
        let circuit = TestCircuitHandle::new_from_str(circuit_src.as_str()).unwrap();
        let input_len = rng.gen_range(0, num_bytes_circuit_capacity + 1);
        println!("input_len={}", input_len);
        let msg = random_bytes(&mut rng, input_len);
        let expected_output =
            pad_and_hash_bytes_with_len(msg.as_slice(), num_bytes_circuit_capacity).unwrap();
        println!("expected_output={}", expected_output);
        let config = CircuitPaddingConfig::new().max_length("in", num_bytes_circuit_capacity);
        let circuit_input_signals = CircuitInputSignals::new()
            .bytes_input("in", msg.as_slice())
            .usize_input("len", msg.len())
            .fr_input("expected_output", expected_output)
            .pad(&config)
            .unwrap();
        let result = circuit.gen_witness(circuit_input_signals);
        println!("gen_witness_result={:?}", result);
        assert!(result.is_ok());
    }
}

#[test]
fn hash_bytes_to_field_no_len() {
    let circuit_src_template = r#"
pragma circom 2.1.3;

include "helpers/hashtofield.circom";

template HashBytesToFieldTest(max_len) {
    signal input in[max_len];
    signal input expected_output;
    component c1 = HashBytesToField(max_len);
    c1.in <== in;
    expected_output === c1.hash;
}

component main = HashBytesToFieldTest(__MAX_LEN__);
"#;

    let max_supported_byte_len = MAX_NUM_INPUT_SCALARS * BYTES_PACKED_PER_SCALAR;

    let mut rng = thread_rng();
    let num_iterations = std::env::var("NUM_ITERATIONS")
        .unwrap_or("10".to_string())
        .parse::<usize>()
        .unwrap_or(10);

    //TODO: hardcode some interesting circuit dimensions that's widely used in keyless.

    for i in 0..num_iterations {
        println!();
        println!("Iteration {} starts.", i);
        let num_bytes_circuit_capacity: usize = rng.gen_range(1, max_supported_byte_len);
        println!("num_bytes_circuit_capacity={}", num_bytes_circuit_capacity);
        let circuit_src = circuit_src_template.replace(
            "__MAX_LEN__",
            num_bytes_circuit_capacity.to_string().as_str(),
        );
        let circuit = TestCircuitHandle::new_from_str(circuit_src.as_str()).unwrap();
        let input_len = rng.gen_range(0, num_bytes_circuit_capacity + 1);
        println!("input_len={}", input_len);
        let msg = random_bytes(&mut rng, input_len);
        let expected_output =
            pad_and_hash_bytes_no_len(msg.as_slice(), num_bytes_circuit_capacity).unwrap();
        println!("expected_output={}", expected_output);
        let config = CircuitPaddingConfig::new().max_length("in", num_bytes_circuit_capacity);
        let circuit_input_signals = CircuitInputSignals::new()
            .bytes_input("in", msg.as_slice())
            .fr_input("expected_output", expected_output)
            .pad(&config)
            .unwrap();
        let result = circuit.gen_witness(circuit_input_signals);
        println!("gen_witness_result={:?}", result);
        assert!(result.is_ok());
    }
}

#[test]
fn hash_limbs_to_field_with_len() {
    let circuit_src_template = r#"
pragma circom 2.1.3;

include "helpers/hashtofield.circom";

template Hash64BitLimbsToFieldWithLenTest(max_len) {
    signal input in[max_len];
    signal input len;
    signal input expected_output;
    component c1 = Hash64BitLimbsToFieldWithLen(max_len);
    c1.in <== in;
    c1.len <== len;
    expected_output === c1.hash;
}

component main = Hash64BitLimbsToFieldWithLenTest(__MAX_LEN__);
"#;

    // Have to save 1 scalar slot for the length.
    let max_supported_limb_len = (MAX_NUM_INPUT_SCALARS - 1) * LIMBS_PACKED_PER_SCALAR;

    let mut rng = thread_rng();
    let num_iterations = std::env::var("NUM_ITERATIONS")
        .unwrap_or("10".to_string())
        .parse::<usize>()
        .unwrap_or(10);

    //TODO: hardcode some interesting circuit dimensions that's widely used in keyless.

    for i in 0..num_iterations {
        println!();
        println!("Iteration {} starts.", i);
        let num_limbs_circuit_capacity: usize = rng.gen_range(1, max_supported_limb_len);
        println!("num_limbs_circuit_capacity={}", num_limbs_circuit_capacity);
        let circuit_src = circuit_src_template.replace(
            "__MAX_LEN__",
            num_limbs_circuit_capacity.to_string().as_str(),
        );
        let circuit = TestCircuitHandle::new_from_str(circuit_src.as_str()).unwrap();
        let input_len = rng.gen_range(0, num_limbs_circuit_capacity + 1);
        println!("input_len={}", input_len);
        let limbs: Vec<u64> = (0..input_len).map(|_| rng.gen()).collect();
        let expected_output =
            pad_and_hash_limbs_with_len(limbs.as_slice(), num_limbs_circuit_capacity).unwrap();
        println!("expected_output={}", expected_output);
        let config = CircuitPaddingConfig::new().max_length("in", num_limbs_circuit_capacity);
        let circuit_input_signals = CircuitInputSignals::new()
            .limbs_input("in", limbs.as_slice())
            .usize_input("len", limbs.len())
            .fr_input("expected_output", expected_output)
            .pad(&config)
            .unwrap();
        let result = circuit.gen_witness(circuit_input_signals);
        println!("gen_witness_result={:?}", result);
        assert!(result.is_ok());
    }
}

#[test]
fn check_are_64bit_limbs_should_pass_with_valid_limbs() {
    let mut rng = thread_rng();
    let circuit_src_template = r#"
pragma circom 2.1.3;
include "helpers/hashtofield.circom";
component main = CheckAre64BitLimbs(__NUM_LIMBS__);
"#;

    for num_limbs in 0..60 {
        println!();
        println!("Iteration {} starts.", num_limbs);
        let circuit_src =
            circuit_src_template.replace("__NUM_LIMBS__", num_limbs.to_string().as_str());
        let circuit = TestCircuitHandle::new_from_str(circuit_src.as_str()).unwrap();
        let limbs: Vec<u64> = (0..num_limbs).map(|_| rng.gen()).collect();
        let config = CircuitPaddingConfig::new().max_length("in", num_limbs);
        let circuit_input_signals = CircuitInputSignals::new()
            .limbs_input("in", limbs.as_slice())
            .pad(&config)
            .unwrap();
        let result = circuit.gen_witness(circuit_input_signals);
        println!("gen_witness_result={:?}", result);
        assert!(result.is_ok());
    }
}

#[test]
fn check_are_64bit_limbs_should_fail_with_invalid_limbs() {
    let circuit_src_template = r#"
pragma circom 2.1.3;
include "helpers/hashtofield.circom";
component main = CheckAre64BitLimbs(__NUM_LIMBS__);
"#;

    for num_limbs in 1..60 {
        let circuit_src =
            circuit_src_template.replace("__NUM_LIMBS__", num_limbs.to_string().as_str());
        let circuit = TestCircuitHandle::new_from_str(circuit_src.as_str()).unwrap();
        let invalid_limb_value = Fr::from(u64::MAX) + Fr::ONE;
        let frs = vec![invalid_limb_value; num_limbs];
        let config = CircuitPaddingConfig::new().max_length("in", num_limbs);
        let circuit_input_signals = CircuitInputSignals::new()
            .frs_input("in", frs.as_slice())
            .pad(&config)
            .unwrap();
        let result = circuit.gen_witness(circuit_input_signals);
        println!("gen_witness_result={:?}", result);
        assert!(result.is_err());
    }
}
