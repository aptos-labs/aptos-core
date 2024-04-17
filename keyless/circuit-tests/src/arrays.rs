use aptos_keyless_common::input_processing::circuit_input_signals::CircuitInputSignals;
use aptos_keyless_common::input_processing::config::CircuitPaddingConfig;
use aptos_keyless_common::input_processing::witness_gen::witness_gen;
use crate::{TestCircuitHandle};
use itertools::*;
use aptos_crypto::poseidon_bn254;
use ark_bn254::Fr;
use ark_ff::{Zero, One};
     
#[test]
fn array_selector_test() {
    let circuit_handle = TestCircuitHandle::new("array_selector_test.circom").unwrap();
    let output = [0,0,1,1,1,0,0,0];
    let start_index = 2;
    let end_index = 5;
    let out_len = 8;
    let config = CircuitPaddingConfig::new().max_length("expected_output", out_len);
    let circuit_input_signals = CircuitInputSignals::new().u64_input("start_index", start_index).u64_input("end_index", end_index).bytes_input("expected_output", &output).pad(&config).unwrap();
     
    let result = circuit_handle.gen_witness(circuit_input_signals);
    assert!(result.is_ok());
}

#[test]
fn array_selector_complex_test() {
    let circuit_handle = TestCircuitHandle::new("array_selector_complex_test.circom").unwrap();
    let output = [0,0,1,1,1,0,0,0];
    let start_index = 2;
    let end_index = 5;
    let out_len = 8;
    let config = CircuitPaddingConfig::new().max_length("expected_output", out_len);
    let circuit_input_signals = CircuitInputSignals::new().u64_input("start_index", start_index).u64_input("end_index", end_index).bytes_input("expected_output", &output).pad(&config).unwrap();
     
    let result = circuit_handle.gen_witness(circuit_input_signals);
    assert!(result.is_ok());
}

#[test]
fn left_array_selector_test() {
    let circuit_handle = TestCircuitHandle::new("left_array_selector_test.circom").unwrap();
    let output = [1,1,0,0,0,0,0,0];
    let index = 2;
    let out_len = 8;
    let config = CircuitPaddingConfig::new().max_length("expected_output", out_len);
    let circuit_input_signals = CircuitInputSignals::new().u64_input("index", index).bytes_input("expected_output", &output).pad(&config).unwrap();
     
    let result = circuit_handle.gen_witness(circuit_input_signals);
    assert!(result.is_ok());
}

#[test]
fn right_array_selector_test() {
    let circuit_handle = TestCircuitHandle::new("right_array_selector_test.circom").unwrap();
    let output = [0,0,0,1,1,1,1,1];
    let index = 2;
    let out_len = 8;
    let config = CircuitPaddingConfig::new().max_length("expected_output", out_len);
    let circuit_input_signals = CircuitInputSignals::new().u64_input("index", index).bytes_input("expected_output", &output).pad(&config).unwrap();
     
    let result = circuit_handle.gen_witness(circuit_input_signals);
    assert!(result.is_ok());
}

#[test]
fn single_one_array_test() {
    let circuit_handle = TestCircuitHandle::new("single_one_array_test.circom").unwrap();
    let output = [0,0,1,0,0,0,0,0];
    let index = 2;
    let out_len = 8;
    let config = CircuitPaddingConfig::new().max_length("expected_output", out_len);
    let circuit_input_signals = CircuitInputSignals::new().u64_input("index", index).bytes_input("expected_output", &output).pad(&config).unwrap();
     
    let result = circuit_handle.gen_witness(circuit_input_signals);
    assert!(result.is_ok());
}

#[test]
fn select_array_value_test() {
    let circuit_handle = TestCircuitHandle::new("select_array_value_test.circom").unwrap();
    let array = [4,6,1,8,9,4,2,3];
    let index = 4;
    let output = 9;
    let in_len = 8;
    let config = CircuitPaddingConfig::new().max_length("array", in_len);
    let circuit_input_signals = CircuitInputSignals::new().u64_input("index", index).bytes_input("array", &array).u64_input("expected_output", output).pad(&config).unwrap();
     
    let result = circuit_handle.gen_witness(circuit_input_signals);
    assert!(result.is_ok());
}

#[test]
fn single_neg_one_array_test() {
    let circuit_handle = TestCircuitHandle::new("single_neg_one_array_test.circom").unwrap();
    let output = [Fr::zero(), Fr::zero(), Fr::zero()-Fr::one(), Fr::zero(), Fr::zero(), Fr::zero(), Fr::zero(), Fr::zero()]; //[0,0,-1,0,0,0,0,0];
    let index = 2;
    let out_len = 8;
    let config = CircuitPaddingConfig::new().max_length("expected_output", out_len);
    let circuit_input_signals = CircuitInputSignals::new().u64_input("index", index).frs_input("expected_output", &output).pad(&config).unwrap();
     
    let result = circuit_handle.gen_witness(circuit_input_signals);
    assert!(result.is_ok());
}

#[test]
fn check_substr_inclusion_poly_test() {
    let circuit_handle = TestCircuitHandle::new("check_substr_inclusion_poly_test.circom").unwrap();

    let max_str_len = 100;
    let max_substr_len = 20;
    let config = CircuitPaddingConfig::new().max_length("str", max_str_len).max_length("substr", max_substr_len);
    let string = "Hello World!";
    let string_hash = poseidon_bn254::pad_and_hash_string(&string, max_str_len).unwrap();
    let substring = "lo Wor";
    let substring_len = substring.len();
    let start_index = 3;


    let circuit_input_signals = CircuitInputSignals::new().str_input("str", string).str_input("substr", substring).u64_input("substr_len", substring_len as u64).u64_input("start_index", start_index).fr_input("str_hash", string_hash).pad(&config).unwrap();

     
    let result = circuit_handle.gen_witness(circuit_input_signals);
    assert!(result.is_ok());
}

#[test]
fn check_substr_inclusion_poly_boolean_test() {
    let circuit_handle = TestCircuitHandle::new("check_substr_inclusion_poly_boolean_test.circom").unwrap();

    let max_str_len = 100;
    let max_substr_len = 20;
    let config = CircuitPaddingConfig::new().max_length("str", max_str_len).max_length("substr", max_substr_len);
    let string = "Hello World!";
    let string_hash = poseidon_bn254::pad_and_hash_string(&string, max_str_len).unwrap();
    let substring = "lo Wor";
    let substring_len = substring.len();
    let start_index = 3;
    let expected_output = 1;


    let circuit_input_signals = CircuitInputSignals::new().str_input("str", string).str_input("substr", substring).u64_input("substr_len", substring_len as u64).u64_input("start_index", start_index).fr_input("str_hash", string_hash).u64_input("expected_output", expected_output).pad(&config).unwrap();

     
    let result = circuit_handle.gen_witness(circuit_input_signals);
    assert!(result.is_ok());
}

#[test]
fn concatenation_check_test() {
    let circuit_handle = TestCircuitHandle::new("concatenation_check_test.circom").unwrap();

    let max_full_str_len = 100;
    let max_left_str_len = 70;
    let max_right_str_len = 70;
    let config = CircuitPaddingConfig::new().max_length("full_string", max_full_str_len).max_length("left", max_left_str_len).max_length("right", max_right_str_len);
    let full_string = "Hello World!";
    let left_string = "Hello ";
    let right_string = "World!";
    let left_len = left_string.len();
    let right_len = right_string.len();


    let circuit_input_signals = CircuitInputSignals::new().str_input("full_string", full_string).str_input("left", left_string).str_input("right", right_string).u64_input("left_len", left_len as u64).u64_input("right_len", right_len as u64).pad(&config).unwrap();

     
    let result = circuit_handle.gen_witness(circuit_input_signals);
    assert!(result.is_ok());
}

#[test]
fn check_are_ascii_digits_test() {
    let circuit_handle = TestCircuitHandle::new("check_are_ascii_digits_test.circom").unwrap();
    let max_input_len = 20;
    let input_arr = [48,49,50,52,55,3,0,200];
    let len = 5;
    let config = CircuitPaddingConfig::new().max_length("in", max_input_len);
    let circuit_input_signals = CircuitInputSignals::new().u64_input("len", len).bytes_input("in", &input_arr).pad(&config).unwrap();
     
    let result = circuit_handle.gen_witness(circuit_input_signals);
    assert!(result.is_ok());
}

#[test]
fn ascii_digits_to_field_test() {
    let circuit_handle = TestCircuitHandle::new("ascii_digits_to_field_test.circom").unwrap();
    let max_input_len = 20;
    let digits = [50,49,50,52,55,3,0,200]; // 21247
    let len = 5;
    let expected_output = 21247;
    let config = CircuitPaddingConfig::new().max_length("digits", max_input_len);
    let circuit_input_signals = CircuitInputSignals::new().u64_input("len", len).bytes_input("digits", &digits).u64_input("expected_output", expected_output).pad(&config).unwrap();
     
    let result = circuit_handle.gen_witness(circuit_input_signals);
    assert!(result.is_ok());
}
